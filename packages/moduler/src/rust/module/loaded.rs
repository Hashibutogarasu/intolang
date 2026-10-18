use ast::Program;

use super::module::Module;
use crate::package::PackageInfo;

/// A module together with the source text that was read for it.
pub struct LoadedModule {
    pub module: Module,
    pub contents: String,
    pub program: Program,
}

impl LoadedModule {
    /// The package the module belongs to.
    pub fn package(&self) -> &PackageInfo {
        &self.module.package
    }
}
