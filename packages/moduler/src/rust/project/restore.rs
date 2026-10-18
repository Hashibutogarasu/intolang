use anyhow::{Context, Result, anyhow, bail};
use std::path::Path;

use super::manifest::ProjectManifest;
use super::project::{Project, Restoration, commit, write_pending};
use crate::lock::LockFile;
use crate::package::{
    PackageAlias, PackageHash, PackageInfo, find_by_alias, is_builtin, link_directory,
    remove_installed,
};

impl Project {
    /// Restores the locked packages. Builtin entries carry no location and are
    /// resolved from the compiler's `embedded` packages.
    pub fn restore(&self, embedded: &Path) -> Result<Vec<Restoration>> {
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
            let Some(location) = locked.location.as_ref() else {
                if !is_builtin(&locked.alias) {
                    bail!("Lock entry for `{alias}` has no location");
                }
                let package_root = find_by_alias(embedded, &locked.alias)?.ok_or_else(|| {
                    anyhow!("The compiler does not embed the `{alias}` package")
                })?;
                let manifest_path = package_root.join("into-package.json");
                let package_manifest: serde_json::Value = serde_json::from_slice(
                    &std::fs::read(&manifest_path)
                        .with_context(|| format!("Could not read {}", manifest_path.display()))?,
                )?;
                let embedded_version =
                    PackageInfo::new(&locked.alias, package_root.clone(), &package_manifest)
                        .version;
                if embedded_version != locked.version {
                    bail!(
                        "Package `{alias}` is locked at version {} but the compiler provides version {embedded_version}",
                        locked.version
                    );
                }
                remove_installed(&self.package_dir(&alias))?;
                manifest_changed |= manifest.set_dependency(&alias, &locked.version)?;
                let mut package = locked.clone();
                package.root = package_root;
                restored.push(Restoration {
                    package,
                    installed: false,
                });
                continue;
            };
            let expected = locked
                .hash
                .clone()
                .ok_or_else(|| anyhow!("Lock entry for `{alias}` has no hash"))?;
            let location = root.join(location);
            let source = std::fs::canonicalize(&location).with_context(|| {
                format!("Could not find package `{alias}` at {}", location.display())
            })?;
            let found = PackageHash::of_directory(&source)?;
            if found != expected {
                bail!(
                    "Package `{alias}` at {} does not match the lock file (expected {expected}, found {found})",
                    source.display()
                );
            }
            let destination = self.package_dir(&alias);
            let linked = std::fs::canonicalize(&destination).is_ok_and(|target| target == source);
            if !linked {
                link_directory(&source, &destination)?;
            }
            manifest_changed |= manifest.set_dependency(&alias, &locked.version)?;
            let mut package = locked.clone();
            package.root = destination;
            restored.push(Restoration {
                package,
                installed: !linked,
            });
        }
        if manifest_changed {
            let pending = write_pending(&self.manifest_path(), &manifest.to_string())?;
            commit(&pending, &self.manifest_path())?;
        }
        Ok(restored)
    }
}
