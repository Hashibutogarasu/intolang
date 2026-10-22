use errors::{Errors, Result};
use package::Package;
use std::path::Path;

use super::project::{Project, Update};
use crate::package::{PackageHash, PackageInfo, is_builtin};

impl Project {
    /// The packages recorded in the lock file; none when nothing is installed
    /// yet.
    pub fn locked(&self) -> Result<Vec<PackageInfo>> {
        Ok(self
            .load_lock()?
            .map(|lock| lock.packages)
            .unwrap_or_default())
    }

    /// Brings one locked package up to date. A package whose source directory
    /// hashes differently from the locked hash is installed over the old one; a
    /// builtin package is replaced when the compiler embeds another version.
    pub fn update(&self, locked: &PackageInfo, embedded: &Path) -> Result<Update> {
        let Some(location) = locked.location.as_ref() else {
            if !is_builtin(&locked.alias) {
                return Err(Errors::LockEntryNoLocation {
                    alias: locked.alias.to_string(),
                }
                .into());
            }
            let found = Package::find_by_alias(embedded, &locked.alias)?.ok_or_else(|| {
                Errors::PackageNotEmbedded {
                    alias: locked.alias.to_string(),
                }
            })?;
            if found.manifest.version == locked.version {
                return Ok(Update {
                    package: locked.clone(),
                    updated: false,
                });
            }
            return Ok(Update {
                package: self.install_builtin(embedded)?,
                updated: true,
            });
        };
        let location = self.canonical_root()?.join(location);
        let source = std::fs::canonicalize(&location).map_err(|error| {
            Errors::PackageLocationMissing {
                alias: locked.alias.to_string(),
                path: location.display().to_string(),
                reason: error.to_string(),
            }
        })?;
        if locked.hash.as_ref() == Some(&PackageHash::of_directory(&source)?) {
            return Ok(Update {
                package: locked.clone(),
                updated: false,
            });
        }
        Ok(Update {
            package: self.install(&source, embedded)?,
            updated: true,
        })
    }
}
