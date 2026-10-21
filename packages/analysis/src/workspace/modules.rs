use std::path::Path;

use package::Package;

pub const SOURCE_ROOT: &str = "src/main";
pub const SOURCE_EXTENSION: &str = "into";

pub struct ModuleEntry {
    pub name: String,
    pub directory: bool,
}

/// The aliases of the packages installed under `packages_dir`.
pub fn package_aliases(packages_dir: &Path) -> Vec<String> {
    let mut aliases: Vec<String> = Package::children(packages_dir)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|package| package.manifest.alias)
        .collect();
    aliases.sort();
    aliases
}

/// The directories and modules under `directory` of the sources of the package
/// `alias`, sorted by name; a module is named without its extension.
pub fn module_entries(packages_dir: &Path, alias: &str, directory: &str) -> Vec<ModuleEntry> {
    let Ok(Some(found)) = Package::find_by_alias(packages_dir, alias) else {
        return Vec::new();
    };
    let Ok(paths) = package::read_dir(&found.join(SOURCE_ROOT).join(directory)) else {
        return Vec::new();
    };
    let mut entries: Vec<ModuleEntry> = paths
        .into_iter()
        .filter_map(|path| {
            let directory = package::is_dir(&path);
            if !directory && path.extension().is_none_or(|ext| ext != SOURCE_EXTENSION) {
                return None;
            }
            let name = if directory {
                path.file_name()?.to_str()?.to_owned()
            } else {
                path.file_stem()?.to_str()?.to_owned()
            };
            Some(ModuleEntry { name, directory })
        })
        .collect();
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    entries
}
