use anyhow::{Context, Result, anyhow};
use std::path::Path;

use super::manifest::ProjectManifest;
use super::project::{Project, commit, write_pending};
use crate::package::{PackageAlias, PackageInfo, relative_path};

const PACKAGE_MANIFEST: &str = "into-package.json";

impl Project {
    pub fn install(&self, source: &Path) -> Result<PackageInfo> {
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
        let alias = PackageAlias::try_from(field("alias")?)?;
        let version = field("version")?.to_owned();

        let mut project_manifest = ProjectManifest::load(&self.manifest_path())?;
        let mut lock = self.load_lock()?.unwrap_or_default();
        let staged = self.stage(&source)?;
        let destination = self.package_dir(&alias);
        let package = PackageInfo::new(alias.as_str(), destination.clone(), &package_manifest)
            .with_hash(staged.hash.clone())
            .with_location(relative_path(&self.canonical_root()?, &source));
        project_manifest.set_dependency(&alias, &version)?;
        lock.upsert(package.clone());

        let pending = write_pending(&self.manifest_path(), &project_manifest.to_string()).and_then(
            |manifest| {
                Ok((
                    manifest,
                    write_pending(&self.lock_path(), &lock.to_string())?,
                ))
            },
        );
        let (pending_manifest, pending_lock) = match pending {
            Ok(pending) => pending,
            Err(error) => {
                staged.discard();
                return Err(error);
            }
        };
        self.replace(staged, &destination)?;
        commit(&pending_manifest, &self.manifest_path())?;
        commit(&pending_lock, &self.lock_path())?;
        Ok(package)
    }
}
