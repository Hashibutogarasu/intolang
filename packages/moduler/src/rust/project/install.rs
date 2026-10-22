use errors::{Errors, Result};
use package::Package;
use std::path::Path;

use super::manifest::ProjectManifest;
use super::project::{Project, commit, write_pending};
use crate::package::{
    BUILTIN_ALIAS, PackageAlias, PackageHash, PackageInfo, is_builtin, link_directory,
    relative_path, remove_installed,
};

impl Project {
    /// Installs the package at `source`. Builtin packages are never linked or
    /// copied: they are referenced from the compiler's `embedded` packages.
    pub fn install(&self, source: &Path, embedded: &Path) -> Result<PackageInfo> {
        let source =
            std::fs::canonicalize(source).map_err(|error| Errors::PackageDirectoryNotFound {
                path: source.display().to_string(),
                reason: error.to_string(),
            })?;
        let found = Package::load(&source)?;
        let manifest = &found.manifest;
        if is_builtin(manifest.required_alias()?) {
            return self.install_builtin(embedded);
        }
        let alias = PackageAlias::try_from(manifest.required_alias()?)?;
        if manifest.version.is_empty() {
            return Err(Errors::PackageManifestMissing {
                key: "version".to_owned(),
            }
            .into());
        }
        let version = manifest.version.clone();

        let mut project_manifest = ProjectManifest::load(&self.manifest_path())?;
        let mut lock = self.load_lock()?.unwrap_or_default();
        let destination = self.package_dir(&alias);
        let package = PackageInfo::new(alias.as_str(), destination.clone(), manifest)
            .with_hash(PackageHash::of_directory(&source)?)
            .with_location(relative_path(&self.canonical_root()?, &source));
        project_manifest.set_dependency(&alias, &version)?;
        lock.upsert(package.clone());

        let pending_manifest = write_pending(&self.manifest_path(), &project_manifest.to_string())?;
        let pending_lock = write_pending(&self.lock_path(), &lock.to_string())?;
        if let Err(error) = link_directory(&source, &destination) {
            std::fs::remove_file(&pending_manifest).ok();
            std::fs::remove_file(&pending_lock).ok();
            return Err(error);
        }
        commit(&pending_manifest, &self.manifest_path())?;
        commit(&pending_lock, &self.lock_path())?;
        Ok(package)
    }

    pub(super) fn install_builtin(&self, embedded: &Path) -> Result<PackageInfo> {
        let alias = PackageAlias::try_from(BUILTIN_ALIAS)?;
        let found = Package::find_by_alias(embedded, BUILTIN_ALIAS)?.ok_or_else(|| {
            Errors::PackageNotEmbedded {
                alias: BUILTIN_ALIAS.to_string(),
            }
        })?;
        let package = PackageInfo::new(alias.as_str(), found.root, &found.manifest);

        let mut project_manifest = ProjectManifest::load(&self.manifest_path())?;
        let mut lock = self.load_lock()?.unwrap_or_default();
        project_manifest.set_dependency(&alias, &package.version)?;
        lock.upsert(package.clone());

        let pending_manifest = write_pending(&self.manifest_path(), &project_manifest.to_string())?;
        let pending_lock = write_pending(&self.lock_path(), &lock.to_string())?;
        if let Err(error) = remove_installed(&self.package_dir(&alias)) {
            std::fs::remove_file(&pending_manifest).ok();
            std::fs::remove_file(&pending_lock).ok();
            return Err(error);
        }
        commit(&pending_manifest, &self.manifest_path())?;
        commit(&pending_lock, &self.lock_path())?;
        Ok(package)
    }
}
