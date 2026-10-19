mod compile;
mod manifest;
mod pkg_config;
mod shared;
mod sources;

pub use compile::{NativeBuild, compile_sources};
pub use manifest::native_dependencies;
pub use shared::{SharedSource, compile_shared_library};
pub use sources::{NativeSource, collect_sources};
