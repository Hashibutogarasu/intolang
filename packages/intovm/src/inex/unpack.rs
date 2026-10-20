use bundle::Entry;
use errors::{Errors, Result};
use std::path::Path;

use super::model::Inex;

impl Inex {
    /// Write the files and directories of the bundle below `directory`.
    pub fn unpack(&self, directory: &Path) -> Result<()> {
        for (path, entry) in self.bundle.entries() {
            let target = directory.join(path.as_str());
            match entry {
                Entry::Directory => std::fs::create_dir_all(&target)
                    .map_err(|error| Errors::file_action("create", &target, error))?,
                Entry::File { mode, .. } => {
                    if let Some(parent) = target.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::write(&target, self.bundle.read(path.as_str()).unwrap_or_default())
                        .map_err(|error| Errors::file_action("unpack", &target, error))?;
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
