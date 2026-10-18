//! Module resolution for the Into language.
//!
//! This crate owns everything concerned with `import`/`export` statements:
//! locating packages by alias, following imports transitively, collecting the
//! public names a module exposes, and resolving `#[link]` metadata. A resolved
//! module is represented by [`Module`], which pairs the [`PackageInfo`] it came
//! from with the path of its source file. It also installs, restores and
//! removes the packages of a project ([`Project`]), recorded in a [`LockFile`].

mod exports;
mod library;
mod links;
mod lock;
mod module;
mod package;
mod project;
mod spec;

pub use exports::{module_exports, public_items, resolve};
pub use library::{NativeFunction, lower_library};
pub use links::resolve_links;
pub use lock::LockFile;
pub use module::{LoadedModule, Module, collect_modules};
pub use package::{
    PackageAlias, PackageHash, PackageInfo, find_by_alias, is_installed, link_directory,
    remove_installed,
};
pub use project::{Project, Restoration};
pub use spec::{ExportSpec, ImportSpec};
