mod manifest;
mod package;
mod vfs;

pub use manifest::{BuiltConfig, MANIFEST_FILE, Manifest};
pub use package::Package;
pub use vfs::{
    MemoryFs, RealFs, Vfs, current, is_dir, is_file, is_real, read_dir, read_to_string, with_vfs,
};
