use errors::{Errors, Result};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

use crate::{BundlePath, Entry};

/// A file tree kept in memory: the files and directories of a project, in
/// the order of their paths, with the contents of equal files stored once.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Bundle {
    entries: BTreeMap<BundlePath, Entry>,
    contents: BTreeMap<String, Vec<u8>>,
}

impl Bundle {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a file, creating the directories above it.
    pub fn add_file(&mut self, path: &str, contents: impl AsRef<[u8]>, mode: u32) -> Result<()> {
        let path = BundlePath::try_from(path)?;
        if self.entries.contains_key(&path) {
            return Err(Errors::BundleDuplicate {
                path: path.to_string(),
            }
            .into());
        }
        self.add_parents(&path)?;
        let contents = contents.as_ref();
        let hash = Sha256::digest(contents)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        self.contents
            .entry(hash.clone())
            .or_insert_with(|| contents.to_vec());
        self.entries.insert(
            path,
            Entry::File {
                size: contents.len() as u64,
                hash,
                mode,
            },
        );
        Ok(())
    }

    /// Add a directory, creating the directories above it. A directory that
    /// is there already is kept.
    pub fn add_directory(&mut self, path: &str) -> Result<()> {
        let path = BundlePath::try_from(path)?;
        match self.entries.get(&path) {
            Some(Entry::Directory) => return Ok(()),
            Some(Entry::File { .. }) => {
                return Err(Errors::BundleNotDirectory {
                    path: path.to_string(),
                }
                .into());
            }
            None => {}
        }
        self.add_parents(&path)?;
        self.entries.insert(path, Entry::Directory);
        Ok(())
    }

    fn add_parents(&mut self, path: &BundlePath) -> Result<()> {
        let mut parents = Vec::new();
        let mut current = path.parent();
        while let Some(parent) = current {
            current = parent.parent();
            parents.push(parent);
        }
        for parent in parents.into_iter().rev() {
            match self.entries.get(&parent) {
                Some(Entry::File { .. }) => {
                    return Err(Errors::BundleNotDirectory {
                        path: parent.to_string(),
                    }
                    .into());
                }
                Some(Entry::Directory) => {}
                None => {
                    self.entries.insert(parent, Entry::Directory);
                }
            }
        }
        Ok(())
    }

    pub fn get(&self, path: &str) -> Option<&Entry> {
        BundlePath::try_from(path)
            .ok()
            .and_then(|path| self.entries.get(&path))
    }

    /// The contents of a file.
    pub fn read(&self, path: &str) -> Option<&[u8]> {
        match self.get(path)? {
            Entry::File { hash, .. } => self.contents.get(hash).map(Vec::as_slice),
            Entry::Directory => None,
        }
    }

    /// The names and entries directly in a directory; an empty path is the
    /// root.
    pub fn read_dir(&self, path: &str) -> Vec<(&str, &Entry)> {
        let parent = if path.is_empty() {
            None
        } else {
            match BundlePath::try_from(path) {
                Ok(path) => Some(path),
                Err(_) => return Vec::new(),
            }
        };
        self.entries
            .iter()
            .filter(|(entry, _)| entry.parent() == parent)
            .map(|(entry, value)| (entry.file_name(), value))
            .collect()
    }

    /// Every path of the bundle with its entry, in the order of the paths.
    pub fn entries(&self) -> impl Iterator<Item = (&BundlePath, &Entry)> {
        self.entries.iter()
    }

    /// A hash of the whole tree: its paths, kinds, modes and contents.
    pub fn digest(&self) -> String {
        let mut digest = Sha256::new();
        for (path, entry) in &self.entries {
            digest.update(path.as_str().as_bytes());
            digest.update([0]);
            match entry {
                Entry::Directory => digest.update([0]),
                Entry::File { hash, mode, .. } => {
                    digest.update([1]);
                    digest.update(mode.to_le_bytes());
                    digest.update(hash.as_bytes());
                }
            }
        }
        digest
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    /// How many different file contents the bundle holds.
    pub fn distinct_contents(&self) -> usize {
        self.contents.len()
    }
}
