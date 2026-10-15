use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub(super) fn run(path: PathBuf, max_depth: Option<usize>) -> Result<()> {
    let mut files = Vec::new();
    if path.is_dir() {
        collect_into_files(&path, &mut files, max_depth, 0)?;
        files.sort();
        if files.is_empty() {
            anyhow::bail!("No .into files found under {}", path.display());
        }
    } else if path.extension().is_some_and(|extension| extension == "into") {
        files.push(path);
    } else {
        anyhow::bail!("Format mode expects an .into file or directory path");
    }

    for file in files {
        let source = std::fs::read_to_string(&file)
            .with_context(|| format!("Could not read Into source: {}", file.display()))?;
        let formatted = backend::format_source(&source);
        std::fs::write(&file, formatted)
            .with_context(|| format!("Could not write formatted Into source: {}", file.display()))?;
        println!("Formatted {}", file.display());
    }
    Ok(())
}

fn collect_into_files(
    root: &Path,
    files: &mut Vec<PathBuf>,
    max_depth: Option<usize>,
    depth: usize,
) -> Result<()> {
    if !root.exists() { return Ok(()); }
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() && max_depth.is_none_or(|limit| depth < limit) {
            collect_into_files(&path, files, max_depth, depth + 1)?;
        } else if path.extension().is_some_and(|extension| extension == "into") {
            files.push(path);
        }
    }
    Ok(())
}
