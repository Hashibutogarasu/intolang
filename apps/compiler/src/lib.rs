extern crate llvm_sys as llvm;

mod codegen;
mod object;
mod resources;
mod target;

use anyhow::{Context, Result};
use std::{ffi::CString, path::Path};

/// Compile source output into an executable at `output`.
pub fn compile(message: &str, output: &Path) -> Result<()> {
    compile_with_packages(message, output, &resources::builtin_packages_dir())
}

/// Compile using a package directory assembled for the current project.
pub fn compile_with_packages(message: &str, output: &Path, packages_dir: &Path) -> Result<()> {
    unsafe {
        let context = llvm::core::LLVMContextCreate();
        let module_name = CString::new("intolang-module")?;
        let module = llvm::core::LLVMModuleCreateWithNameInContext(module_name.as_ptr(), context);
        let builder = llvm::core::LLVMCreateBuilderInContext(context);

        codegen::generate(context, module, builder, message)
            .context("Error while generating module")?;
        let machine =
            target::host_target_machine().context("Failed to initialize target machine")?;
        let object_path = output.with_extension("o");
        object::emit(module, machine, &object_path).context("Failed to emit object file")?;

        llvm::target_machine::LLVMDisposeTargetMachine(machine);
        llvm::core::LLVMDisposeBuilder(builder);
        llvm::core::LLVMDisposeModule(module);
        llvm::core::LLVMContextDispose(context);
    }

    let object_path = output.with_extension("o");
    let (native_objects, uses_cxx, libraries, library_paths) =
        compile_imported_native_sources(message, output, packages_dir)?;
    lld::link_objects(
        &object_path,
        &native_objects,
        output,
        uses_cxx,
        &libraries,
        &library_paths,
    )
    .context("embedded LLD failed to write executable")?;
    for object in native_objects {
        std::fs::remove_file(object).ok();
    }
    std::fs::remove_file(object_path).ok();
    Ok(())
}

fn compile_imported_native_sources(
    message: &str,
    output: &Path,
    packages_dir: &Path,
) -> Result<(Vec<std::path::PathBuf>, bool, Vec<String>, Vec<String>)> {
    let resource_root = packages_dir;
    let mut sources = Vec::new();
    for line in message.lines() {
        if let Some(call) = line.strip_prefix("\u{1e}NATIVE_LINK_ASSIGN\t") {
            let parts = call.split('\t').collect::<Vec<_>>();
            if parts.len() >= 4 {
                let package = package_dir(resource_root, parts[0]);
                let source = package.join(parts[2]);
                let language = parts[1];
                if !source.is_file() {
                    anyhow::bail!("Linked native source does not exist: {}", source.display());
                }
                sources.push((source, language == "cpp", package));
            }
        }
    }

    let mut unique_sources = std::collections::HashSet::new();
    sources.retain(|(source, _, _)| unique_sources.insert(source.clone()));

    let mut objects = Vec::new();
    let mut uses_cxx = false;
    let mut libraries = Vec::new();
    let mut library_paths = Vec::new();
    for (index, (source, cxx, package)) in sources.iter().enumerate() {
        uses_cxx |= *cxx;
        let dependencies = native_dependencies(package, source)?;
        for library in &dependencies {
            let path = pkg_config_flags(std::slice::from_ref(library), "--variable=libdir")?
                .into_iter()
                .next()
                .with_context(|| {
                    format!("pkg-config returned no library directory for `{library}`")
                })?;
            if !library_paths.contains(&path) {
                library_paths.push(path);
            }
        }
        for flag in pkg_config_flags(&dependencies, "--libs")? {
            if let Some(name) = flag.strip_prefix("-l") {
                if !name.is_empty() && !libraries.iter().any(|library| library == name) {
                    libraries.push(name.to_owned());
                }
            } else if let Some(path) = flag.strip_prefix("-L") {
                if !library_paths.iter().any(|existing| existing == path) {
                    library_paths.push(path.to_owned());
                }
            }
        }
        let object =
            output.with_file_name(format!(".intolang-native-{}-{index}.o", std::process::id()));
        let compiler = if *cxx { "CXX" } else { "CC" };
        let default_compiler = if *cxx { "c++" } else { "cc" };
        let command = std::env::var(compiler).unwrap_or_else(|_| default_compiler.into());
        let result = std::process::Command::new(command)
            .arg("-fPIC")
            .arg("-c")
            .args(pkg_config_flags(&dependencies, "--cflags")?)
            .arg(source)
            .arg("-o")
            .arg(&object)
            .output()
            .with_context(|| {
                format!(
                    "Could not compile native module source {}",
                    source.display()
                )
            })?;
        if !result.status.success() {
            anyhow::bail!(
                "Native module compilation failed for {}:\n{}",
                source.display(),
                String::from_utf8_lossy(&result.stderr)
            );
        }
        objects.push(object);
    }
    Ok((objects, uses_cxx, libraries, library_paths))
}

/// Locate a package directory by its manifest `alias`. The directory name is
/// not required to match the alias (the bundled builtin package lives in
/// `intolang` while its alias is `builtin`).
fn package_dir(packages_dir: &Path, alias: &str) -> std::path::PathBuf {
    if let Ok(entries) = std::fs::read_dir(packages_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let manifest = path.join("into-package.json");
            if let Ok(contents) = std::fs::read_to_string(&manifest) {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&contents) {
                    if value.get("alias").and_then(serde_json::Value::as_str) == Some(alias) {
                        return path;
                    }
                }
            }
        }
    }
    packages_dir.join(alias)
}

fn native_dependencies(package: &Path, source: &Path) -> Result<Vec<String>> {
    let manifest = package.join("into-package.json");
    let contents = std::fs::read_to_string(&manifest).with_context(|| {
        format!(
            "Could not read native package manifest {}",
            manifest.display()
        )
    })?;
    let value: serde_json::Value = serde_json::from_str(&contents)
        .with_context(|| format!("Invalid JSON in {}", manifest.display()))?;
    let relative = source
        .strip_prefix(package)
        .with_context(|| format!("Native source {} is outside its package", source.display()))?;
    let key = relative.to_string_lossy().replace('\\', "/");
    Ok(value
        .get("nativeDependencies")
        .and_then(|deps| deps.get(key))
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_owned)
        .collect())
}

fn pkg_config_flags(dependencies: &[String], flag: &str) -> Result<Vec<String>> {
    if dependencies.is_empty() {
        return Ok(Vec::new());
    }
    let output = std::process::Command::new("pkg-config")
        .arg(flag)
        .args(dependencies)
        .output()
        .context("Could not run pkg-config for declared native dependencies")?;
    if !output.status.success() {
        anyhow::bail!(
            "pkg-config failed for native dependencies {:?}: {}",
            dependencies,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .map(str::to_owned)
        .collect())
}

/// Compile exported Into functions into a host native shared library.
pub fn compile_library(functions: &[backend::NativeFunction], output: &Path) -> Result<()> {
    unsafe {
        let context = llvm::core::LLVMContextCreate();
        let module_name = CString::new("intolang-library")?;
        let module = llvm::core::LLVMModuleCreateWithNameInContext(module_name.as_ptr(), context);
        let builder = llvm::core::LLVMCreateBuilderInContext(context);

        codegen::generate_library(context, module, builder, functions)
            .context("Error while generating native library")?;
        let machine =
            target::host_target_machine().context("Failed to initialize target machine")?;
        let object_path = output.with_extension("o");
        object::emit(module, machine, &object_path)
            .context("Failed to emit library object file")?;

        llvm::target_machine::LLVMDisposeTargetMachine(machine);
        llvm::core::LLVMDisposeBuilder(builder);
        llvm::core::LLVMDisposeModule(module);
        llvm::core::LLVMContextDispose(context);
    }

    let object_path = output.with_extension("o");
    lld::link_shared(&object_path, output)
        .context("embedded LLD failed to write native library")?;
    std::fs::remove_file(object_path).ok();
    Ok(())
}

/// Directory containing builtin packages embedded into the compiler.
pub fn builtin_packages_dir() -> std::path::PathBuf {
    resources::builtin_packages_dir()
}

/// Directory containing the embedded hello_world project template.
pub fn hello_world_template_dir() -> std::path::PathBuf {
    resources::hello_world_template_dir()
}
