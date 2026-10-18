use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::lock::LockFile;
use crate::package::{PackageAlias, PackageInfo};

const MANIFEST: &str = "into-package.json";
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
        std::fs::canonicalize(&self.root)
            .with_context(|| format!("Could not find project directory: {}", self.root.display()))
    }

    pub(super) fn load_lock(&self) -> Result<Option<LockFile>> {
        LockFile::load(&self.lock_path())
    }
}

pub(super) fn write_pending(target: &Path, text: &str) -> Result<PathBuf> {
    let mut name = target.file_name().unwrap_or_default().to_os_string();
    name.push(PENDING_SUFFIX);
    let pending = target.with_file_name(name);
    std::fs::write(&pending, text)
        .with_context(|| format!("Could not write {}", pending.display()))?;
    Ok(pending)
}

pub(super) fn commit(pending: &Path, target: &Path) -> Result<()> {
    std::fs::rename(pending, target)
        .with_context(|| format!("Could not update {}", target.display()))
}
