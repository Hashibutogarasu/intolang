use anyhow::{Context, Result};
use std::path::Path;

pub fn link_directory(source: &Path, link: &Path) -> Result<()> {
    if let Some(parent) = link.parent() {
        std::fs::create_dir_all(parent)?;
    }
    remove_installed(link)?;
    #[cfg(unix)]
    let linked = std::os::unix::fs::symlink(source, link);
    #[cfg(windows)]
    let linked = std::os::windows::fs::symlink_dir(source, link);
    linked.with_context(|| format!("Could not link {} to {}", link.display(), source.display()))
}

pub fn is_installed(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok()
}

pub fn remove_installed(path: &Path) -> Result<()> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(error).with_context(|| format!("Could not read {}", path.display()));
        }
    };
    let removed = if metadata.file_type().is_symlink() {
        std::fs::remove_file(path).or_else(|_| std::fs::remove_dir(path))
    } else {
        std::fs::remove_dir_all(path)
    };
    removed.with_context(|| format!("Could not remove {}", path.display()))
}
