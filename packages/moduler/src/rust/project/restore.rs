use errors::{Errors, Result};
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
        let lock = self.load_lock()?.ok_or_else(|| Errors::LockMissing {
            file: LockFile::FILE_NAME.to_string(),
        })?;
        let mut manifest = ProjectManifest::load(&self.manifest_path())?;
        let root = self.canonical_root()?;
        let mut manifest_changed = false;
        let mut restored = Vec::new();
        for locked in &lock.packages {
            let alias = PackageAlias::try_from(locked.alias.as_str())?;
            let Some(location) = locked.location.as_ref() else {
                if !is_builtin(&locked.alias) {
                    return Err(Errors::LockEntryNoLocation {
                        alias: alias.to_string(),
                    }
                    .into());
                }
                let package_root = find_by_alias(embedded, &locked.alias)?.ok_or_else(|| {
                    Errors::PackageNotEmbedded {
                        alias: alias.to_string(),
                    }
                })?;
                let manifest_path = package_root.join("into-package.json");
                let package_manifest: serde_json::Value = serde_json::from_slice(
                    &std::fs::read(&manifest_path)
                        .map_err(|error| Errors::read(&manifest_path, error))?,
                )?;
                let embedded_version =
                    PackageInfo::new(&locked.alias, package_root.clone(), &package_manifest)
                        .version;
                if embedded_version != locked.version {
                    return Err(Errors::PackageVersionLocked {
                        alias: alias.to_string(),
                        locked: locked.version.clone(),
                        provided: embedded_version,
                    }
                    .into());
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
            let expected = locked.hash.clone().ok_or_else(|| Errors::LockEntryNoHash {
                alias: alias.to_string(),
            })?;
            let location = root.join(location);
            let source = std::fs::canonicalize(&location).map_err(|error| {
                Errors::PackageLocationMissing {
                    alias: alias.to_string(),
                    path: location.display().to_string(),
                    reason: error.to_string(),
                }
            })?;
            let found = PackageHash::of_directory(&source)?;
            if found != expected {
                return Err(Errors::PackageHashMismatch {
                    alias: alias.to_string(),
                    path: source.display().to_string(),
                    expected: expected.to_string(),
                    found: found.to_string(),
                }
                .into());
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
