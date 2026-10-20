use errors::{Errors, Result};
use std::path::{Path, PathBuf};

use crate::lock::LockFile;
use crate::package::{PackageAlias, PackageInfo};

use package::MANIFEST_FILE as MANIFEST;
const PACKAGES: &str = ".into/packages";
const PENDING_SUFFIX: &str = ".install-tmp";

pub struct Project {
    root: PathBuf,
}

pub struct Restoration {
    pub package: PackageInfo,
    pub installed: bool,
}

impl Project {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn manifest_path(&self) -> PathBuf {
        self.root.join(MANIFEST)
    }

    pub fn lock_path(&self) -> PathBuf {
        self.root.join(LockFile::FILE_NAME)
    }

    pub fn packages_dir(&self) -> PathBuf {
        self.root.join(PACKAGES)
    }

    pub fn package_dir(&self, alias: &PackageAlias) -> PathBuf {
        self.packages_dir().join(alias.as_str())
    }

    pub(super) fn canonical_root(&self) -> Result<PathBuf> {
        std::fs::canonicalize(&self.root).map_err(|error| {
            Errors::ProjectDirectoryNotFound {
                path: self.root.display().to_string(),
                reason: error.to_string(),
            }
            .into()
        })
    }

    pub(super) fn load_lock(&self) -> Result<Option<LockFile>> {
        LockFile::load(&self.lock_path())
    }
}

pub(super) fn write_pending(target: &Path, text: &str) -> Result<PathBuf> {
    let mut name = target.file_name().unwrap_or_default().to_os_string();
    name.push(PENDING_SUFFIX);
    let pending = target.with_file_name(name);
    std::fs::write(&pending, text).map_err(|error| Errors::write(&pending, error))?;
    Ok(pending)
}

pub(super) fn commit(pending: &Path, target: &Path) -> Result<()> {
    std::fs::rename(pending, target)
        .map_err(|error| Errors::file_action("update", target, error).into())
}
