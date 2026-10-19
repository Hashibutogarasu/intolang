use anyhow::{Context, Result};
use native::SharedSource;
use parser::Record;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use types::MessageTag;

use super::dl::Library;
use crate::inex::Inex;

const LIBRARY_NAME: &str = "libinex-native.so";

/// Where the packages the native sources belong to are in the unpacked
/// bundle.
const PACKAGES: &str = ".into/packages";

/// The directory the VM keeps what it builds and unpacks for programs.
pub(crate) fn cache_root() -> Result<PathBuf> {
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
    inex.bundle.digest().hash(&mut hasher);
    std::env::var("CC").unwrap_or_default().hash(&mut hasher);
    std::env::var("CXX").unwrap_or_default().hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Whether the program calls native functions.
pub fn links(message: &str) -> bool {
    message
        .lines()
        .filter_map(|line| line.parse::<Record>().ok())
        .any(|record| record.tag() == MessageTag::LinkAssign)
}

fn compile(inex: &Inex, work: &Path) -> Result<PathBuf> {
    inex.unpack(work)?;
    let packages = work.join(PACKAGES);
    let mut sources = Vec::new();
    for source in native::collect_sources(inex.intoc()?, &packages)? {
        sources.push(SharedSource {
            dependencies: native::native_dependencies(&source.package, &source.path)?,
            path: source.path,
            language: source.language,
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
