use anyhow::Result;
use std::ffi::OsStr;
use std::path::Path;

const EXCLUDED: [&str; 2] = [".git", "dist"];

pub(crate) fn is_excluded(name: &OsStr) -> bool {
    EXCLUDED.iter().any(|excluded| name == *excluded)
}

pub(crate) fn copy_package(source: &Path, destination: &Path) -> Result<()> {
    std::fs::create_dir_all(destination)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        if is_excluded(&entry.file_name()) {
            continue;
        }
        let target = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_package(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}
