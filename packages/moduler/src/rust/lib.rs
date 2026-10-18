//! Module resolution for the Into language.
//!
//! This crate owns everything concerned with `import`/`export` statements:
//! locating packages by alias, following imports transitively, collecting the
//! public names a module exposes, and resolving `#[link]` metadata. A resolved
//! module is represented by [`Module`], which pairs the [`PackageInfo`] it came
//! from with the path of its source file.

mod exports;
mod library;
mod links;
mod module;
mod package;
mod spec;

pub use exports::{module_exports, public_items, resolve};
pub use library::{NativeFunction, lower_library};
pub use links::resolve_links;
pub use module::{LoadedModule, Module, collect_modules};
pub use package::{PackageInfo, find_by_alias};
pub use spec::{ExportSpec, ImportSpec};
