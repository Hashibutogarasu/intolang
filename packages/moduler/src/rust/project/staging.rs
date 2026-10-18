use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use super::project::Project;
use crate::package::{PackageHash, copy_package};

pub(super) struct Staged {
    pub path: PathBuf,
    pub hash: PackageHash,
}

impl Staged {
    pub fn discard(self) {
        std::fs::remove_dir_all(self.path).ok();
    }
}

impl Project {
    pub(super) fn stage(&self, source: &Path) -> Result<Staged> {
        let packages = self.packages_dir();
        std::fs::create_dir_all(&packages)?;
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let path = packages.join(format!(".install-{}-{unique}", std::process::id()));
        let prepared = copy_package(source, &path)
            .and_then(|()| PackageHash::of_directory(&path))
            .map(|hash| Staged {
                path: path.clone(),
                hash,
            });
        if prepared.is_err() {
            std::fs::remove_dir_all(&path).ok();
        }
        prepared
    }

    pub(super) fn replace(&self, staged: Staged, destination: &Path) -> Result<()> {
        if destination.exists() {
            std::fs::remove_dir_all(destination)
                .with_context(|| format!("Could not replace {}", destination.display()))?;
        }
        std::fs::rename(&staged.path, destination)
            .with_context(|| format!("Could not install {}", destination.display()))
    }
}
