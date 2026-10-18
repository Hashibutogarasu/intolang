use anyhow::{Context, Result, anyhow};
use std::path::Path;

use super::manifest::ProjectManifest;
use super::project::{Project, commit, write_pending};
use crate::package::{
    BUILTIN_ALIAS, PackageAlias, PackageHash, PackageInfo, find_by_alias, is_builtin,
    link_directory, relative_path, remove_installed,
};

const PACKAGE_MANIFEST: &str = "into-package.json";

impl Project {
    /// Installs the package at `source`. Builtin packages are never linked or
    /// copied: they are referenced from the compiler's `embedded` packages.
    pub fn install(&self, source: &Path, embedded: &Path) -> Result<PackageInfo> {
        let source = std::fs::canonicalize(source)
            .with_context(|| format!("Could not find package directory: {}", source.display()))?;
        let manifest_path = source.join(PACKAGE_MANIFEST);
        let package_manifest: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&manifest_path).with_context(|| {
                format!(
                    "Could not read package manifest: {}",
                    manifest_path.display()
                )
            })?)
            .with_context(|| format!("Invalid package manifest: {}", manifest_path.display()))?;
        let field = |key: &str| {
            package_manifest
                .get(key)
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| anyhow!("Package manifest must define `{key}`"))
        };
        if is_builtin(field("alias")?) {
            return self.install_builtin(embedded);
        }
        let alias = PackageAlias::try_from(field("alias")?)?;
        let version = field("version")?.to_owned();

        let mut project_manifest = ProjectManifest::load(&self.manifest_path())?;
        let mut lock = self.load_lock()?.unwrap_or_default();
        let destination = self.package_dir(&alias);
        let package = PackageInfo::new(alias.as_str(), destination.clone(), &package_manifest)
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

    fn install_builtin(&self, embedded: &Path) -> Result<PackageInfo> {
        let alias = PackageAlias::try_from(BUILTIN_ALIAS)?;
        let root = find_by_alias(embedded, BUILTIN_ALIAS)?
            .ok_or_else(|| anyhow!("The compiler does not embed the `{BUILTIN_ALIAS}` package"))?;
        let manifest_path = root.join(PACKAGE_MANIFEST);
        let package_manifest: serde_json::Value = serde_json::from_slice(
            &std::fs::read(&manifest_path)
                .with_context(|| format!("Could not read {}", manifest_path.display()))?,
        )?;
        let package = PackageInfo::new(alias.as_str(), root, &package_manifest);

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
