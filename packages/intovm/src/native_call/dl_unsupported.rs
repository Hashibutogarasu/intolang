use errors::{Errors, Result};
use std::ffi::c_void;
use std::path::Path;

/// Where shared libraries cannot be loaded, as in a browser. A program that
/// calls native functions never gets this far; the VM runs it on its own
/// functions.
#[derive(Debug)]
pub struct Library {
    _private: (),
}

impl Library {
    pub fn open(path: &Path) -> Result<Self> {
        Err(errors::Error::from(Errors::NativeLibraryLoad {
            path: path.display().to_string(),
            reason: "shared libraries cannot be loaded here".to_owned(),
        }))
    }

    pub fn symbol(&self, _symbol: &str) -> Result<Option<*const c_void>> {
        Ok(None)
    }
}
