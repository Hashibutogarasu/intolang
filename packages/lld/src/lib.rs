use anyhow::{Context, Result, bail};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Link an LLVM-generated object with LLD selected at build time.
pub fn link(object: &Path, output: &Path) -> Result<()> {
    link_with_args(object, &[], output, false, &[], &[], false)
}

/// Link an LLVM-generated object as a native shared library.
pub fn link_shared(object: &Path, output: &Path) -> Result<()> {
    link_with_args(object, &[], output, false, &[], &[], true)
}

/// Link a generated object with helper objects and optional system libraries.
pub fn link_objects(
    object: &Path,
    extra_objects: &[std::path::PathBuf],
    output: &Path,
    cxx: bool,
    libraries: &[String],
    library_paths: &[String],
) -> Result<()> {
    link_with_args(
        object,
        extra_objects,
        output,
        cxx,
        libraries,
        library_paths,
        false,
    )
}

/// The C++ runtime libraries the executable loads. The driver finds them, but
/// linking with LLD in place of the driver's own linker leaves out the
/// run-path that points at them, so a program started outside the build
/// environment could not load them.
const RUNTIME_LIBRARIES: [&str; 2] = ["libstdc++.so.6", "libgcc_s.so.1"];

pub fn runtime_directories(driver: &str) -> Vec<String> {
    let mut directories: Vec<String> = Vec::new();
    for library in RUNTIME_LIBRARIES {
        let Ok(output) = std::process::Command::new(driver)
            .arg(format!("-print-file-name={library}"))
            .output()
        else {
            continue;
        };
        let printed = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        let found = Path::new(&printed);
        if found.is_absolute()
            && let Some(directory) = found.parent()
        {
            let directory = directory.to_string_lossy().into_owned();
            if !directories.contains(&directory) {
                directories.push(directory);
            }
        }
    }
    directories
}

fn link_with_args(
    object: &Path,
    extra_objects: &[std::path::PathBuf],
    output: &Path,
    cxx: bool,
    libraries: &[String],
    library_paths: &[String],
    shared: bool,
) -> Result<()> {
    static NEXT_LINK: AtomicUsize = AtomicUsize::new(0);
    let lld_dir = std::env::temp_dir().join(format!(
        "intolang-lld-{}-{}",
        std::process::id(),
        NEXT_LINK.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&lld_dir)?;
    let lld_path = lld_dir.join("ld.lld");
    if let Some(binary) = option_env!("LLD_EXTERNAL_BINARY") {
        #[cfg(unix)]
        std::os::unix::fs::symlink(binary, &lld_path)?;
        #[cfg(not(unix))]
        std::fs::copy(binary, &lld_path)?;
    } else {
        std::fs::write(&lld_path, include_bytes!(env!("LLD_EMBEDDED_BINARY")))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&lld_path, std::fs::Permissions::from_mode(0o755))?;
        }
    }
    let prefix = format!("-B{}/", lld_dir.display());
    let linker = if cxx { "CXX" } else { "CC" };
    let default_driver = if cxx { "c++" } else { "cc" };
    let driver = std::env::var(linker).unwrap_or_else(|_| default_driver.into());
    let mut command = std::process::Command::new(&driver);
    command.arg(prefix).arg("-fuse-ld=lld");
    if shared {
        command.arg("-shared");
    }
    command.arg(object);
    for extra in extra_objects {
        command.arg(extra);
    }
    for library in libraries {
        command.arg(format!("-l{library}"));
    }
    for path in library_paths {
        command.arg(format!("-L{path}"));
        command.arg(format!("-Wl,-rpath,{path}"));
    }
    if cxx {
        for directory in runtime_directories(&driver) {
            command.arg(format!("-Wl,-rpath,{directory}"));
        }
    }
    let result = command
        .arg("-o")
        .arg(output)
        .output()
        .context("could not start C compiler driver")?;
    std::fs::remove_dir_all(lld_dir).ok();
    if !result.status.success() {
        bail!(
            "LLD link failed ({}):\n{}",
            result.status,
            String::from_utf8_lossy(&result.stderr)
        );
    }
    Ok(())
}
