use anyhow::{Context, Result};
use bundle::Entry;
use std::path::Path;

use super::model::Inex;

impl Inex {
    /// Write the files and directories of the bundle below `directory`.
    pub fn unpack(&self, directory: &Path) -> Result<()> {
        for (path, entry) in self.bundle.entries() {
            let target = directory.join(path.as_str());
            match entry {
                Entry::Directory => std::fs::create_dir_all(&target)
                    .with_context(|| format!("Could not create {}", target.display()))?,
                Entry::File { mode, .. } => {
                    if let Some(parent) = target.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::write(&target, self.bundle.read(path.as_str()).unwrap_or_default())
                        .with_context(|| format!("Could not unpack {}", target.display()))?;
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(*mode))?;
                    }
                    #[cfg(not(unix))]
                    let _ = mode;
                }
            }
        }
        Ok(())
    }
}
