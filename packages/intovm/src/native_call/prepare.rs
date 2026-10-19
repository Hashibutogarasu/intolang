use anyhow::{Context, Result, bail};
use native::SharedSource;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use super::dl::Library;
use crate::inex::Inex;

const LIBRARY_NAME: &str = "libinex-native.so";

fn cache_root() -> Result<PathBuf> {
    if let Some(directory) = std::env::var_os("INTO_CACHE_DIR") {
        return Ok(PathBuf::from(directory));
    }
    if let Some(directory) = std::env::var_os("XDG_CACHE_HOME") {
        return Ok(PathBuf::from(directory).join("into"));
    }
    let home = std::env::var_os("HOME").context("native calls need HOME or INTO_CACHE_DIR")?;
    Ok(PathBuf::from(home).join(".cache/into"))
}

fn fingerprint(inex: &Inex) -> String {
    let mut hasher = DefaultHasher::new();
    for file in &inex.files {
        file.path.hash(&mut hasher);
        file.contents.hash(&mut hasher);
    }
    for source in &inex.sources {
        source.alias.hash(&mut hasher);
        source.path.hash(&mut hasher);
        source.dependencies.hash(&mut hasher);
    }
    std::env::var("CC").unwrap_or_default().hash(&mut hasher);
    std::env::var("CXX").unwrap_or_default().hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn inside(path: &str) -> Result<&Path> {
    let path = Path::new(path);
    if path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
    {
        bail!("The bundled path {} leaves its directory", path.display());
    }
    Ok(path)
}

fn compile(inex: &Inex, work: &Path) -> Result<PathBuf> {
    for file in &inex.files {
        let target = work.join(inside(&file.path)?);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&target, &file.contents)
            .with_context(|| format!("Could not unpack {}", target.display()))?;
    }
    let mut sources = Vec::new();
    for source in &inex.sources {
        sources.push(SharedSource {
            path: work.join(&source.alias).join(inside(&source.path)?),
            language: source.language,
            dependencies: source.dependencies.clone(),
        });
    }
    let output = work.join(LIBRARY_NAME);
    native::compile_shared_library(&sources, &output)?;
    Ok(output)
}

fn build(inex: &Inex, directory: &Path, library: &Path) -> Result<()> {
    static BUILDS: AtomicU64 = AtomicU64::new(0);
    let work = directory.join(format!(
        "build-{}-{}",
        std::process::id(),
        BUILDS.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&work)?;
    let built = compile(inex, &work)
        .and_then(|output| std::fs::rename(output, library).map_err(Into::into));
    std::fs::remove_dir_all(&work).ok();
    built
}

/// Compile the native sources of `inex` once per content, then load them.
pub fn load(inex: &Inex) -> Result<Library> {
    let directory = cache_root()?.join("native").join(fingerprint(inex));
    std::fs::create_dir_all(&directory)?;
    let library = directory.join(LIBRARY_NAME);
    if library.is_file() {
        if let Ok(opened) = Library::open(&library) {
            return Ok(opened);
        }
        std::fs::remove_file(&library).ok();
    }
    build(inex, &directory, &library)?;
    Library::open(&library)
}
