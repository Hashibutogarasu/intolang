use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Component, Path, PathBuf};

use super::Vfs;

/// Files held in memory, for a place with no disk. Paths are compared after
/// their `.` and `..` are resolved, so `/a/b/../c` and `/a/c` are one file.
#[derive(Debug, Clone, Default)]
pub struct MemoryFs {
    files: BTreeMap<PathBuf, String>,
}

impl MemoryFs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, path: impl AsRef<Path>, contents: impl Into<String>) {
        self.files.insert(normalize(path.as_ref()), contents.into());
    }
}

impl<P: AsRef<Path>, C: Into<String>> FromIterator<(P, C)> for MemoryFs {
    fn from_iter<I: IntoIterator<Item = (P, C)>>(files: I) -> Self {
        let mut fs = Self::new();
        for (path, contents) in files {
            fs.insert(path, contents);
        }
        fs
    }
}

impl Vfs for MemoryFs {
    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        self.files
            .get(&normalize(path))
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no such file"))
    }

    fn is_file(&self, path: &Path) -> bool {
        self.files.contains_key(&normalize(path))
    }

    fn is_dir(&self, path: &Path) -> bool {
        let path = normalize(path);
        self.files
            .keys()
            .any(|file| file != &path && file.starts_with(&path))
    }

    fn read_dir(&self, path: &Path) -> io::Result<Vec<PathBuf>> {
        let path = normalize(path);
        let children: BTreeSet<PathBuf> = self
            .files
            .keys()
            .filter(|file| file.starts_with(&path))
            .filter_map(|file| {
                let next = file.components().nth(path.components().count())?;
                Some(path.join(next))
            })
            .collect();
        if children.is_empty() {
            return Err(io::Error::new(io::ErrorKind::NotFound, "no such directory"));
        }
        Ok(children.into_iter().collect())
    }
}

fn normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other),
        }
    }
    normalized
}
