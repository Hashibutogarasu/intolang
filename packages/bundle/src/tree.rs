use errors::{Errors, Result};
use std::path::Path;

use crate::Bundle;

/// How deep a directory tree may nest before it is taken for a loop.
const MAX_DEPTH: usize = 64;

/// The directory a package is linked into, relative to a project.
const PACKAGES: &str = ".into/packages";

impl Bundle {
    /// Add the files and directories under `source` below the path `mount`,
    /// in the order of their names; an empty `mount` is the root. Links are
    /// followed.
    pub fn add_tree(&mut self, source: &Path, mount: &str) -> Result<()> {
        self.add_tree_skipping(source, mount, &|_| false)
    }

    /// Like [`Bundle::add_tree`], leaving out the paths, relative to
    /// `source`, that `skip` names.
    pub fn add_tree_skipping(
        &mut self,
        source: &Path,
        mount: &str,
        skip: &dyn Fn(&str) -> bool,
    ) -> Result<()> {
        if !mount.is_empty() {
            self.add_directory(mount)?;
        }
        self.walk(source, mount, "", skip, 0)
    }

    /// Add the language-facing files of the package in `directory` as the
    /// package `alias` of a project: its manifest and sources, without its
    /// Rust crate, its build outputs and its history.
    pub fn mount_package(&mut self, alias: &str, directory: &Path) -> Result<()> {
        self.add_tree_skipping(directory, &format!("{PACKAGES}/{alias}"), &|relative| {
            matches!(relative, "target" | "dist" | ".git") || relative.starts_with("src/rust")
        })
    }

    fn walk(
        &mut self,
        directory: &Path,
        mount: &str,
        relative: &str,
        skip: &dyn Fn(&str) -> bool,
        depth: usize,
    ) -> Result<()> {
        if depth > MAX_DEPTH {
            return Err(Errors::BundleTooDeep {
                path: directory.display().to_string(),
            }
            .into());
        }
        let mut names = std::fs::read_dir(directory)
            .map_err(|error| Errors::read(directory, error))?
            .map(|entry| entry.map(|entry| entry.file_name().to_string_lossy().into_owned()))
            .collect::<std::io::Result<Vec<_>>>()
            .map_err(|error| Errors::read(directory, error))?;
        names.sort();
        for name in names {
            let child = if relative.is_empty() {
                name.clone()
            } else {
                format!("{relative}/{name}")
            };
            if skip(&child) {
                continue;
            }
            let target = if mount.is_empty() {
                child.clone()
            } else {
                format!("{mount}/{child}")
            };
            let path = directory.join(&name);
            let metadata = std::fs::metadata(&path).map_err(|error| Errors::read(&path, error))?;
            if metadata.is_dir() {
                self.add_directory(&target)?;
                self.walk(&path, mount, &child, skip, depth + 1)?;
            } else {
                let contents = std::fs::read(&path).map_err(|error| Errors::read(&path, error))?;
                self.add_file(&target, contents, mode_of(&metadata))?;
            }
        }
        Ok(())
    }
}

#[cfg(unix)]
fn mode_of(metadata: &std::fs::Metadata) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o777
}

#[cfg(not(unix))]
fn mode_of(_: &std::fs::Metadata) -> u32 {
    0o644
}
