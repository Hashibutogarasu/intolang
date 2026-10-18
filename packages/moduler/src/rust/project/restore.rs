use anyhow::{Context, Result, anyhow, bail};

use super::manifest::ProjectManifest;
use super::project::{Project, Restoration, commit, write_pending};
use crate::lock::LockFile;
use crate::package::{PackageAlias, PackageHash};

impl Project {
    pub fn restore(&self) -> Result<Vec<Restoration>> {
        let lock = self.load_lock()?.ok_or_else(|| {
            anyhow!(
                "No {} found; install a package with `into install <package>` first",
                LockFile::FILE_NAME
            )
        })?;
        let mut manifest = ProjectManifest::load(&self.manifest_path())?;
        let root = self.canonical_root()?;
        let mut manifest_changed = false;
        let mut restored = Vec::new();
        for locked in &lock.packages {
            let alias = PackageAlias::try_from(locked.alias.as_str())?;
            let expected = locked
                .hash
                .clone()
                .ok_or_else(|| anyhow!("Lock entry for `{alias}` has no hash"))?;
            let destination = self.package_dir(&alias);
            let current = destination.is_dir()
                && PackageHash::of_directory(&destination).is_ok_and(|hash| hash == expected);
            if !current {
                let location = root.join(&locked.location);
                let source = std::fs::canonicalize(&location).with_context(|| {
                    format!("Could not find package `{alias}` at {}", location.display())
                })?;
                let staged = self.stage(&source)?;
                if staged.hash != expected {
                    let found = staged.hash.clone();
                    staged.discard();
                    bail!(
                        "Package `{alias}` at {} does not match the lock file (expected {expected}, found {found})",
                        source.display()
                    );
                }
                self.replace(staged, &destination)?;
            }
            manifest_changed |= manifest.set_dependency(&alias, &locked.version)?;
            let mut package = locked.clone();
            package.root = destination;
            restored.push(Restoration {
                package,
                installed: !current,
            });
        }
        if manifest_changed {
            let pending = write_pending(&self.manifest_path(), &manifest.to_string())?;
            commit(&pending, &self.manifest_path())?;
        }
        Ok(restored)
    }
}
