use std::path::Path;

/// Where the text of a module comes from. The file system by default; an
/// editor supplies its open buffers first.
pub trait SourceProvider {
    fn read(&self, path: &Path) -> Option<String>;
}

pub struct FileSystem;

impl SourceProvider for FileSystem {
    fn read(&self, path: &Path) -> Option<String> {
        package::read_to_string(path).ok()
    }
}
