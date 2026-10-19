use anyhow::{Context, Result};
use intovm::{Inex, InexFile, InexHeader, InexSource, Vm, VmOptions};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The extension of the Into executables the VM runs.
pub(super) const EXTENSION: &str = "inex";

/// Whether `path` names an `.inex` file.
pub(super) fn is_inex(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension == EXTENSION)
}

/// Package the lowered `message` of a project with what its native calls
/// need, found in the package environment `packages`.
pub(super) fn assemble(header: InexHeader, message: String, packages: &Path) -> Result<Inex> {
    let native = native::collect_sources(&message, packages)?;
    let mut sources = Vec::new();
    let mut directories = BTreeSet::new();
    for source in &native {
        sources.push(InexSource {
            alias: source.alias.clone(),
            language: source.language,
            path: source.relative.clone(),
            dependencies: native::native_dependencies(&source.package, &source.path)?,
        });
        directories.insert((source.alias.clone(), source.package.join("src/ffi")));
    }
    let mut files = Vec::new();
    for (alias, directory) in directories {
        bundle(&alias, &directory, Path::new("src/ffi"), &mut files)?;
    }
    Ok(Inex {
        header,
        message,
        sources,
        files,
    })
}

/// Add every file under `directory`, as `alias/relative/...`.
fn bundle(alias: &str, directory: &Path, relative: &Path, files: &mut Vec<InexFile>) -> Result<()> {
    if !directory.is_dir() {
        return Ok(());
    }
    let mut entries = std::fs::read_dir(directory)?.collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let relative = relative.join(entry.file_name());
        if path.is_dir() {
            bundle(alias, &path, &relative, files)?;
        } else {
            files.push(InexFile {
                path: format!("{alias}/{}", relative.to_string_lossy().replace('\\', "/")),
                contents: std::fs::read(&path)
                    .with_context(|| format!("Could not read {}", path.display()))?,
            });
        }
    }
    Ok(())
}

/// Write `inex` to `output`, executable where the platform has such a mode.
pub(super) fn write(inex: &Inex, output: &Path) -> Result<()> {
    std::fs::write(output, Vec::from(inex))
        .with_context(|| format!("Could not write {}", output.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(output, std::fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}

/// Read an `.inex` file and run it.
pub(super) fn run_file(path: &PathBuf) -> Result<()> {
    let bytes =
        std::fs::read(path).with_context(|| format!("Could not read {}", path.display()))?;
    let inex = Inex::try_from(bytes.as_slice())
        .with_context(|| format!("Could not load {}", path.display()))?;
    execute(&inex)
}

/// Run the program of `inex` in the VM as the application, so it works with
/// the real process, signals and time, and end this process with the code the
/// program ends with.
pub(super) fn execute(inex: &Inex) -> Result<()> {
    let mut vm = Vm::new(VmOptions {
        echo_stdout: true,
        real_processes: true,
        stdin: None,
        ..VmOptions::default()
    });
    vm.load_inex(inex)?;
    vm.run()?;
    match vm.exit_status() {
        0 => Ok(()),
        status => std::process::exit(status),
    }
}
