mod assets;
mod compile;
mod embed;
mod manifest;
mod pkg_config;
mod shared;
mod sources;

pub use assets::required_assets;
pub use compile::{NativeBuild, compile_sources};
pub use embed::embed_bundle;
pub use manifest::native_dependencies;
pub use shared::{SharedSource, compile_shared_library};
pub use sources::{NativeSource, collect_sources};
