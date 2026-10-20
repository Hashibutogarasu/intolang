use errors::{Errors, Result};
use std::path::Path;

use super::package::Package;
use crate::manifest::MANIFEST_FILE;

impl Package {
    pub fn children(directory: &Path) -> Result<Vec<Package>> {
        let mut packages = Vec::new();
        for entry in std::fs::read_dir(directory).map_err(|error| Errors::read(directory, error))? {
            let path = entry?.path();
            if path.is_dir()
                && let Ok(package) = Package::load(&path)
            {
                packages.push(package);
            }
        }
        Ok(packages)
    }

    pub fn find_by_alias(directory: &Path, alias: &str) -> Result<Option<Package>> {
        Ok(Package::children(directory)?
            .into_iter()
            .find(|package| package.manifest.alias.as_deref() == Some(alias)))
    }

    pub fn enclosing(path: &Path) -> Option<Package> {
        path.ancestors()
            .skip(1)
            .find(|ancestor| ancestor.join(MANIFEST_FILE).is_file())
            .and_then(|root| Package::load(root).ok())
    }
}
