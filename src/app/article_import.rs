use tokio::fs::{read_dir, DirEntry};
use std::io;
use std::time::SystemTime;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::cmp::Ordering;
use derive_more::Display;
use serde_json;

// On the Java app this is a service.
// I could also make this happen in a struct
// that I put in the app state in which case
// the "import lock" atomic bool could be 
// stored in that struct.

// OK let's do that I guess.

const IMPORT_EXT: &'static str = "json";

// I thought it'd be a good time to start using
// custom error types more, even though this is 
// mostly an internal thing.
#[derive(Debug, Display)]
enum ImportError {
  #[display(fmt = "IO error")]
  IOError,
  #[display(fmt = "Parse error")]
  ParseError
}
// Standard way to implement the Error trait is
// to not actually implement any function at all.
impl std::error::Error for ImportError {}

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

  // At some point I discovered I could just use tokio
  // async/await version of std::fs.
  async fn list_json_earliest_first(
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
    while let Some(file) = files.next_entry().await? {
      let is_import_ext: bool = file.path()
        .extension()
        .map(
          |ext| 
          ext.to_str().unwrap_or("").to_lowercase() == IMPORT_EXT
        )
        .unwrap_or(false);
      // Add to the list of import files if has the right
      // extension and is a file:
      if is_import_ext && file.path().is_file() {
        let modified = modified_time(&file).await;
        import_files.push((file, modified));
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
async fn modified_time(file: &DirEntry) -> u64 {
  file.metadata()
    .await
    .and_then(|f| f.modified())
    .map_or(0, |f| {
      match f.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(t) => t.as_secs(),
        Err(_) => u64::MAX
      }
    })
}

fn parse_article() {
  
}