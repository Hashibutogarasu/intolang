use errors::{Errors, Result};
use std::path::Path;

use super::package::Package;
use crate::manifest::MANIFEST_FILE;
use crate::vfs::{Vfs, current};

impl Package {
    pub fn children(directory: &Path) -> Result<Vec<Package>> {
        Package::children_in(&*current(), directory)
    }

    pub fn children_in(files: &dyn Vfs, directory: &Path) -> Result<Vec<Package>> {
        let mut packages = Vec::new();
        for path in files
            .read_dir(directory)
            .map_err(|error| Errors::read(directory, error))?
        {
            if files.is_dir(&path)
                && let Ok(package) = Package::load_in(files, &path)
            {
                packages.push(package);
            }
        }
        Ok(packages)
    }

    pub fn find_by_alias(directory: &Path, alias: &str) -> Result<Option<Package>> {
        Package::find_by_alias_in(&*current(), directory, alias)
    }

    pub fn find_by_alias_in(
        files: &dyn Vfs,
        directory: &Path,
        alias: &str,
    ) -> Result<Option<Package>> {
        Ok(Package::children_in(files, directory)?
            .into_iter()
            .find(|package| package.manifest.alias.as_deref() == Some(alias)))
    }

    pub fn enclosing(path: &Path) -> Option<Package> {
        Package::enclosing_in(&*current(), path)
    }

    pub fn enclosing_in(files: &dyn Vfs, path: &Path) -> Option<Package> {
        path.ancestors()
            .skip(1)
            .find(|ancestor| files.is_file(&ancestor.join(MANIFEST_FILE)))
            .and_then(|root| Package::load_in(files, root).ok())
    }
}
