use bundle::Bundle;
use errors::{Context, Errors, Result};
use intovm::{Inex, Vm, VmOptions};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The extension of the Into executables the VM runs.
pub(super) const EXTENSION: &str = "inex";

use package::MANIFEST_FILE as MANIFEST;

const FILE_MODE: u32 = 0o644;

/// Whether `path` names an `.inex` file.
pub(super) fn is_inex(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension == EXTENSION)
}

/// The directories and files of the project in `directory` that the `require`
/// and `asset` calls of the lowered `message` name.
pub(super) fn resources(directory: &Path, message: &str) -> Result<Bundle> {
    let mut bundle = Bundle::new();
    for required in native::required_assets(message)? {
        if bundle.get(&required).is_some() {
            continue;
        }
        let resources = directory.join(&required);
        if resources.is_dir() {
            bundle.add_tree(&resources, &required)?;
        } else if resources.is_file() {
            add_file(&mut bundle, &required, &resources)?;
        } else {
            return Err(Errors::BundleResourceMissing { path: required }.into());
        }
    }
    Ok(bundle)
}

/// Package the project in `directory` as an executable: its manifest, its
/// sources, the directories its `require` calls name, the manifests and native
/// sources of the packages the lowered `message` calls into, found in the
/// package environment `packages`, and the message itself as `intoc`.
pub(super) fn assemble(
    directory: &Path,
    packages: &Path,
    message: String,
    generator: String,
) -> Result<Inex> {
    let mut bundle = resources(directory, &message)?;
    add_file(&mut bundle, MANIFEST, &directory.join(MANIFEST))?;
    let sources = directory.join("src");
    if sources.is_dir() {
        bundle.add_tree_skipping(&sources, "src", &|relative| {
            relative == "rust" || relative.starts_with("rust/")
        })?;
    }
    let mut mounted = BTreeSet::new();
    for source in native::collect_sources(&message, packages)? {
        if !mounted.insert(source.alias.clone()) {
            continue;
        }
        let root = format!(".into/packages/{}", source.alias);
        add_file(
            &mut bundle,
            &format!("{root}/{MANIFEST}"),
            &source.package.join(MANIFEST),
        )?;
        let foreign = source.package.join("src/ffi");
        if foreign.is_dir() {
            bundle.add_tree(&foreign, &format!("{root}/src/ffi"))?;
        }
    }
    bundle.add_file("intoc", message.as_bytes(), FILE_MODE)?;
    Ok(Inex { generator, bundle })
}

fn add_file(bundle: &mut Bundle, target: &str, source: &Path) -> Result<()> {
    let contents = std::fs::read(source).map_err(|error| Errors::read(source, error))?;
    Ok(bundle.add_file(target, contents, FILE_MODE)?)
}

/// Write `inex` to `output`, executable where the platform has such a mode.
pub(super) fn write(inex: &Inex, output: &Path) -> Result<()> {
    std::fs::write(output, Vec::from(inex)).map_err(|error| Errors::write(output, error))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(output, std::fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}

/// Read an `.inex` file and run it.
pub(super) fn run_file(path: &PathBuf) -> Result<()> {
    let bytes = std::fs::read(path).map_err(|error| Errors::read(path, error))?;
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
