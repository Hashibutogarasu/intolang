use errors::{Errors, Result};
use package::{MANIFEST_FILE, Manifest, Package};
use std::path::{Path, PathBuf};

pub fn package_dir(packages_dir: &Path, alias: &str) -> PathBuf {
    Package::find_by_alias(packages_dir, alias)
        .ok()
        .flatten()
        .map_or_else(|| packages_dir.join(alias), |package| package.root)
}

pub fn native_dependencies(package: &Path, source: &Path) -> Result<Vec<String>> {
    let manifest = Manifest::load(&package.join(MANIFEST_FILE))?;
    let relative =
        source
            .strip_prefix(package)
            .map_err(|_| Errors::NativeSourceOutsidePackage {
                path: source.display().to_string(),
            })?;
    let key = relative.to_string_lossy().replace('\\', "/");
    Ok(manifest.dependencies_of(&key).to_vec())
}
