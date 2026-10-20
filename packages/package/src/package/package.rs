use errors::Result;
use std::path::{Path, PathBuf};

use crate::manifest::{MANIFEST_FILE, Manifest};
use crate::vfs::Vfs;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    pub root: PathBuf,
    pub manifest: Manifest,
}

impl Package {
    pub fn load(root: &Path) -> Result<Self> {
        Self::load_in(&*crate::vfs::current(), root)
    }

    pub fn load_in(files: &dyn Vfs, root: &Path) -> Result<Self> {
        Ok(Self {
            manifest: Manifest::load_in(files, &root.join(MANIFEST_FILE))?,
            root: root.to_path_buf(),
        })
    }

    pub fn manifest_path(&self) -> PathBuf {
        self.root.join(MANIFEST_FILE)
    }

    pub fn join(&self, path: impl AsRef<Path>) -> PathBuf {
        self.root.join(path)
    }
}
