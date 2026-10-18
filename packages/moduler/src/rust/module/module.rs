use std::path::PathBuf;

use crate::package::PackageInfo;

/// A module reached from an entry source, directly or transitively.
///
/// Mirrors the Into `Module` class: it holds the [`PackageInfo`] it belongs to
/// and the path of the module source file (`sourcePath: Path`).
#[derive(Debug, Clone)]
pub struct Module {
    pub package: PackageInfo,
    pub source_path: PathBuf,
    pub alias: String,
    pub name: String,
}
