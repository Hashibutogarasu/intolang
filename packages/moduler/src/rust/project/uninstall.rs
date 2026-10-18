use anyhow::{Result, bail};

use super::manifest::ProjectManifest;
use super::project::{Project, commit, write_pending};
use crate::package::{PackageAlias, PackageInfo, is_installed, remove_installed};

impl Project {
    pub fn uninstall(&self, alias: &str) -> Result<PackageInfo> {
        let alias = PackageAlias::try_from(alias)?;
        let mut manifest = ProjectManifest::load(&self.manifest_path())?;
        let mut lock = self.load_lock()?.unwrap_or_default();
        let destination = self.package_dir(&alias);

        let declared = manifest.remove_dependency(&alias)?;
        let locked = lock.remove(alias.as_str());
        let installed = is_installed(&destination);
        if declared.is_none() && locked.is_none() && !installed {
            bail!("Package `{alias}` is not installed");
        }

        let pending_manifest = write_pending(&self.manifest_path(), &manifest.to_string())?;
        let pending_lock = if locked.is_some() {
            Some(write_pending(&self.lock_path(), &lock.to_string())?)
        } else {
            None
        };
        remove_installed(&destination)?;
        commit(&pending_manifest, &self.manifest_path())?;
        if let Some(pending_lock) = pending_lock {
            commit(&pending_lock, &self.lock_path())?;
        }
        Ok(locked.unwrap_or_else(|| PackageInfo {
            alias: alias.to_string(),
            version: declared.unwrap_or_default(),
            ..PackageInfo::default()
        }))
    }
}
