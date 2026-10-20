mod current;
mod memory;
mod real;

use std::io;
use std::path::{Path, PathBuf};

pub use current::{current, is_real, with_vfs};
pub use memory::MemoryFs;
pub use real::RealFs;

/// `std::fs::read_to_string` on the current files.
pub fn read_to_string(path: &Path) -> io::Result<String> {
    current().read_to_string(path)
}

/// `Path::is_file` on the current files.
pub fn is_file(path: &Path) -> bool {
    current().is_file(path)
}

/// `Path::is_dir` on the current files.
pub fn is_dir(path: &Path) -> bool {
    current().is_dir(path)
}

/// `std::fs::read_dir` on the current files, as paths.
pub fn read_dir(path: &Path) -> io::Result<Vec<PathBuf>> {
    current().read_dir(path)
}

/// The files a package is read from. The disk on a computer, a table held in
/// memory where there is no disk.
pub trait Vfs {
    fn read_to_string(&self, path: &Path) -> io::Result<String>;

    fn is_file(&self, path: &Path) -> bool;

    fn is_dir(&self, path: &Path) -> bool;

    /// The paths directly inside the directory.
    fn read_dir(&self, path: &Path) -> io::Result<Vec<PathBuf>>;

    /// Whether this is the disk of the computer, where links and temporary
    /// directories can be made.
    fn is_real(&self) -> bool {
        false
    }
}
