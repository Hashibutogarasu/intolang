use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use super::package::Package;

static NEXT: AtomicUsize = AtomicUsize::new(0);

/// A temporary directory holding packages, removed when dropped. Its path can
/// be used as the packages directory of the compiler tools.
pub struct Workspace {
    root: PathBuf,
}

impl Workspace {
    pub fn with_packages(packages: &[Package]) -> Result<Self> {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let root = std::env::temp_dir().join(format!(
            "intolang-eval-{}-{unique}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let workspace = Self { root };
        for package in packages {
            let directory = workspace.root.join(&package.alias);
            std::fs::create_dir_all(&directory)
                .with_context(|| format!("Could not create {}", directory.display()))?;
            std::fs::write(directory.join("into-package.json"), package.manifest())?;
            for (path, contents) in &package.files {
                let target = directory.join(path);
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&target, contents)
                    .with_context(|| format!("Could not write {}", target.display()))?;
            }
        }
        Ok(workspace)
    }

    pub fn path(&self) -> &Path {
        &self.root
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).ok();
    }
}
