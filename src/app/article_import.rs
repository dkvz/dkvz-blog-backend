use std::fs::{read_dir, DirEntry};
use std::io;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use lazy_static::lazy_static;

// On the Java app this is a service.
// I could also make this happen in a struct
// that I put in the app state in which case
// the "import lock" atomic bool could be 
// stored in that struct.

// OK let's do that I guess.

lazy_static! {
  static ref JSON_OS_STR: &'static OsStr = 
    OsStr::new("json");
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

  
}

fn list_json_earliest_first(
  path: &str
) -> Result<Vec<DirEntry>, io::Error> {
  let files = read_dir(path)?;
  let mut json_files: Vec<DirEntry> = Vec::new();
  
  Err(io::Error::new(io::ErrorKind::NotFound, "NOT IMPL"))
}