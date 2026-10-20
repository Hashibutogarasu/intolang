mod alias;
mod builtin;
mod hash;
mod link;
mod location;
mod package_info;

pub(crate) use location::relative_path;

pub use alias::PackageAlias;
pub use builtin::{BUILTIN_ALIAS, BUILTIN_SCHEME, is_builtin};
pub use hash::PackageHash;
pub use link::{is_installed, link_directory, remove_installed};
pub use package_info::PackageInfo;
