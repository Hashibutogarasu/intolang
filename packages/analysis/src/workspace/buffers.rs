use std::collections::HashMap;
use std::path::{Path, PathBuf};

use moduler::{FileSystem, SourceProvider};

/// The text of the open documents, then the files on disk. Paths are compared
/// after links are resolved, because the packages of a build are linked.
pub struct Buffers {
    open: HashMap<PathBuf, String>,
}

impl Buffers {
    pub fn new(open: HashMap<PathBuf, String>) -> Self {
        Self {
            open: open
                .into_iter()
                .map(|(path, text)| (real(&path), text))
                .collect(),
        }
    }
}

impl SourceProvider for Buffers {
    fn read(&self, path: &Path) -> Option<String> {
        self.open
            .get(&real(path))
            .cloned()
            .or_else(|| FileSystem.read(path))
    }
}

pub fn real(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}
