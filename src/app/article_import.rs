use tokio::fs::{read_dir, DirEntry, read_to_string};
use tokio::io;
use eyre::Report;
use color_eyre;
//use std::io;
use std::fs::Metadata;
use std::time::SystemTime;
use std::path::{PathBuf, Path};
use std::sync::atomic::{self, AtomicBool};
use std::cmp::Ordering;
use std::convert::From;
use derive_more::Display;
use log::{error, info, warn};
use serde_json;
use crate::db::{self, Pool};
use crate::db::entities::ArticleUpdate;
use super::dtos::{
  ImportedArticleDto, 
  JsonStatus,
  JsonStatusType
};

// On the Java app this is a service.
// I could also make this happen in a struct
// that I put in the app state in which case
// the "import lock" atomic bool could be 
// stored in that struct.

// OK let's do that I guess.

const IMPORT_EXT: &'static str = "json";
// 30 MB size limit for import files just in
// case:
const MAX_FILE_SIZE: u64 = 31457280;

// I thought it'd be a good time to start using
// custom error types more, even though this is 
// mostly an internal thing.
#[derive(Debug, Display)]
enum ImportError {
  #[display(fmt = "IO error")]
  IOError,
  #[display(fmt = "Parse error")]
  ParseError(String)
}
// Standard way to implement the Error trait is
// to not actually implement any function at all.
impl std::error::Error for ImportError {}

impl From<serde_json::error::Error> for ImportError {
  fn from(error: serde_json::error::Error) -> Self {
    error!("JSON parsing error when importing article: {}", error);
    ImportError::ParseError(error.to_string())
  }
}

// I'm using JsonStatus as an error type for one
// of the main import functions.
impl From<ImportError> for JsonStatus {
  fn from(e: ImportError) -> Self {
    JsonStatus::new(JsonStatusType::Error, &e.to_string())
  }
}

// My database errors use eyre, so uh... Yeah.
impl From<Report<color_eyre::Handler>> for JsonStatus {
  fn from(r: Report<color_eyre::Handler>) -> Self {
    error!("Encountered DB error while importing articles: {}", r);
    JsonStatus::new(JsonStatusType::Error, format!("Database error: {}", r))
  }
}

// A struct can't own a "Path" directly, you
// have to use references with lifetimes and
// all that business so I'm using PathBuf 
// instead.
pub struct ImportService {
  import_path: PathBuf,
  is_import_locked: AtomicBool
}

impl ImportService {

  pub fn from(
    path: &str
  ) -> Result<Self, io::Error> {
    // We have to check if the directory is writable.
    // I also suddenly decided coding like this is much
    // clearer:
    let import_path = PathBuf::from(path);
    let read_only = import_path.metadata()?.permissions().readonly();
    let is_dir = import_path.is_dir();
    match (read_only, is_dir) {
      (false, true) => Ok(Self {
        import_path,
        is_import_locked: AtomicBool::new(false)
      }),
      _ => Err(
        io::Error::new(
          io::ErrorKind::PermissionDenied, 
          "Import directory is not writable"
        )
      ) 
    }
  }

  // Main method for the service.
  // I guess the pool could be owned by the struct, I
  // can probably clone it.
  pub async fn import_articles(
    &self,
    pool: &Pool
  ) -> Result<Vec<JsonStatus>, JsonStatus> {
    // Have to check if an import is already in progress.
    // Lock for import otherwise.
    if self.check_lock_set_if_unlocked() {
      warn!("An import was attempted while the import service is locked");
      return Err(
        JsonStatus::new(
          JsonStatusType::Error, 
          "An import is already in progress"
        )
      );
    }
    let result = self.import_articles_no_lock(pool).await;
    self.unlock();
    result
  }

  async fn import_articles_no_lock(
    &self,
    pool: &Pool
  ) -> Result<Vec<JsonStatus>, JsonStatus> {
    // List all the files in the import directory.
    // The only possible IOError means the directory
    // could not be read for some reason, which is 
    // fatal.
    let files = self.list_files_earliest_first()
      .await
      .map_err(|e| {
        error!("Error reading import directory: {}", e);
        JsonStatus::new(
          JsonStatusType::Error, 
          "Could not list files in import directory"
        )
      })?;

    // Now would have been a good time to use map()
    // except await isn't allowed in there. So it's
    // time for a good old for.
    let mut statuses: Vec<JsonStatus> = Vec::new();
    'outer: for file in files {
      match parse_article(file.path()).await {
        Ok(article) => {
          // Check what we're doing and if we have
          // everything required to do it.
          // - action = 1 and id is present => Delete
          // - no action but id present => Update
          // - no action, no id => Insert
          // When inserting and "short" is absent, 
          // default to make it true.
          // Check if the article exist if we got 
          // an id first:
          if let Some(id) = article.id {
            if !db::article_exists(pool, id)? {
              statuses.push(JsonStatus::new_with_id(
                JsonStatusType::Error, 
                "Article ID doesn't exist", 
                id
              ));
              continue;
            }
          }
          match (article.id, article.action) {
            (Some(id), Some(1)) => {
              // Deleting.
              db::delete_article(pool, id)?;
              statuses.push(JsonStatus::new_with_id(
                JsonStatusType::Success, 
                "Article deleted", 
                id
              ));
            },
            _ => {
              // Inserting or updating.
              // If tags are present, do they all exist?
              if let Some(tags) = article.tags {
                for tag in tags {
                  if !db::tag_exists(pool, tag.id)? {
                    statuses.push(JsonStatus::new(
                      JsonStatusType::Error, 
                      format!("Tag with ID {} does not exist", tag.id)
                    ));
                    continue 'outer;
                  }
                }
              }
              // If user ID is present, does it exist?
              // We could cache that stuff.
              if let Some(user_id) = article.user_id {
                if !db::user_exists(pool, user_id)? {
                  statuses.push(JsonStatus::new(
                    JsonStatusType::Error, 
                    format!("User with ID {} does not exist", user_id)
                  ));
                  continue 'outer;
                }
              }
              // When article_url is present, check that it doesn't
              // exist already (it could be that it's the current 
              // article when updating).
              // Note that I'm currently allowing inserting an 
              // article (thus not a short) with no article URL, even
              // though that shouldn't be allowed.
              if let Some(article_url) = article.article_url {
                let valid_url = 
                  match (db::article_id_by_url(pool, &article_url)?, article.id) {
                    (Some(id_for_url), Some(id)) => id_for_url == id,
                    (Some(_), None) => false,
                    _ => true
                  };
                if !valid_url {
                  statuses.push(JsonStatus::new(
                    JsonStatusType::Error, 
                    format!("Article URL {} already exists", article_url)
                  ));
                  continue 'outer;
                }
              }
              // We need to know the short status:
              let is_short = article.short.unwrap_or(false);
              // Check if updating or inserting:
              match (article.id, article.user_id) {
                (Some(id), _) => {
                  
                },
                (None, Some(user_id)) => {

                },
                _ => {
                  // Missing user_id for insertion:
                  statuses.push(JsonStatus::new(
                    JsonStatusType::Error, 
                    "Field userId is required when inserting articles"
                  ));
                }
              }
            }
          }

          // Attempt to save to DB:

          // Delete the file:

          // Add the success result:

        },
        Err(e) => {
          warn!(
            "Article parsing failed while importing for {:?} - {:?}", 
            file.path(),
            e
          );
          // Add the status to the list:
          statuses.push(e.into());
        }
      }
    }
    
    Ok(statuses)
  }

  // Returns false if the import wasn't locked, but it's
  // now locked.
  // Returns true if it was already locked.
  fn check_lock_set_if_unlocked(&self) -> bool {
    // There's a thing on AtomicBool to only update the bool
    // if it was equal to a given value, then returns a result
    // Ok if the value was updated (previous value in it) and
    // Err if nothing was changed, current value is returned
    // in the Err.
    //self.is_import_locked.load(atomic::Ordering::SeqCst)
    match self.is_import_locked.compare_exchange(
      false, 
      true, 
      atomic::Ordering::SeqCst, 
      atomic::Ordering::Acquire
    ) {
      Ok(_) => false,
      Err(_) => true
    }
  }

  fn lock(&self) {
    self.is_import_locked.store(true, atomic::Ordering::SeqCst);
  }

  fn unlock(&self) {
    self.is_import_locked.store(false, atomic::Ordering::SeqCst);
  }

  // At some point I discovered I could just use tokio
  // async/await version of std::fs.
  async fn list_files_earliest_first(
    &self
  ) -> Result<Vec<DirEntry>, io::Error> {
    let mut files = read_dir(&self.import_path).await?;
    // I had a cool way to filter JSON using 
    // standard fs and a chain of high order functions
    // buttokio fs requires calling an async "next_entry" 
    // function a whole bunch of times.
    // In short, async/await doesn't like closures that
    // much.
    
    // Let's create a list of the files and their modified 
    // timestamp as a u64.
    let mut import_files: Vec<(DirEntry, u64)> =  Vec::new();
    // I'm ignoring IO errors from here, files that give 
    // out weird vibes are just ignore silently.
    while let Ok(Some(file)) = files.next_entry().await {
      let is_import_ext: bool = file.path()
        .extension()
        .map(
          |ext| 
          ext.to_str().unwrap_or("").to_lowercase() == IMPORT_EXT
        )
        .unwrap_or(false);
      // Add to the list of import files if has the right
      // extension and is a file. We ignore the file if we
      // can't process its metadata and if it's larger than
      // a certain size.
      if let Ok(metadata) = file.metadata().await {
        // Big ifs are ugly in Rust. I'm so sorry.
        if is_import_ext && 
          file.path().is_file() && 
          metadata.len() < MAX_FILE_SIZE &&
          !metadata.permissions().readonly() 
          {
            let modified = modified_time(&metadata).await;
            import_files.push((file, modified));
          }
      }
    }
    // We can't use await easily in the sort closure, which
    // is why I made the weird Vec of tuples with the modified
    // date already in it.
    import_files.sort_by(
      |a, b| 
        a.1
        .partial_cmp(&b.1)
        .unwrap_or(Ordering::Equal)
    );
    // I could just return the Vec of tuples (or maybe an 
    // iterator) and spare a few CPU cycles but I couldn't
    // bother.
    Ok(import_files.into_iter().map(|f| f.0).collect())
  }

}

// Ignores the chain of errors when reading
// file modified date, just returns "0" if
// something went wrong.
async fn modified_time(file: &Metadata) -> u64 {
    file.modified()
    .map_or(0, |f| {
      match f.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(t) => t.as_secs(),
        Err(_) => u64::MAX
      }
    })
}

// Was gonna use &DirEntry as the argument type but
// I saw this fancy construct somewhere:
async fn parse_article<P: AsRef<Path>>(
  path: P
) -> Result<ImportedArticleDto, ImportError> {
  /*let file = File::open(path)
    .await
    .map_err(|_| ImportError::IOError)?;*/
  // Tokio's BufReader can't be used with the 
  // standard serde, so I decided to load the whole
  // thing in memory, provided file size is smaller
  // than a certain threshold.
  // Which should be taken are of by the thing that
  // lists all JSON files.
  //let reader = BufReader::new(file);
  let contents = read_to_string(path)
    .await
    .map_err(|_| ImportError::IOError)?;
  // Attempt to parse the JSON. We need a DTO that 
  // is close to what ArticleUpdate is but should 
  // also allow deleting articles.
  let imported: ImportedArticleDto = 
    serde_json::from_str(&contents)?;
    //.map_err(|_| ImportError::ParseError)?;
  Ok(imported)
}

// There's a specific annotation required for async tests.
#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn article_import_valid() {
    let parsed_article = 
      parse_article("./resources/fixtures/import_tests/valid.json")
      .await
      .unwrap();
    assert_eq!(32, parsed_article.id.unwrap());
    assert_eq!("some_url", parsed_article.article_url.unwrap());
    assert_eq!(
      7, 
      parsed_article.tags.unwrap()[0].id
    );
  }

  #[tokio::test]
  async fn article_import_delete() {
    let parsed_article = 
      parse_article("./resources/fixtures/import_tests/delete.json")
      .await
      .unwrap();
    assert_eq!(42, parsed_article.id.unwrap());
    assert_eq!(1, parsed_article.action.unwrap());
  }

}

