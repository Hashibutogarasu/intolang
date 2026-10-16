use std::path::PathBuf;

pub(super) fn builtin_packages_dir() -> PathBuf {
    PathBuf::from(env!("COMPILER_BUNDLED_RES")).join("packages")
}

pub(super) fn hello_world_template_dir() -> PathBuf {
    PathBuf::from(env!("COMPILER_BUNDLED_RES")).join("templates/HelloWorld")
}
