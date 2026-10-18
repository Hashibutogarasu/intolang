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
    let native = native::compile_sources(message, output, packages_dir)?;
    lld::link_objects(
        &object_path,
        &native.objects,
        output,
        native.uses_cxx,
        &native.libraries,
        &native.library_paths,
    )
    .context("embedded LLD failed to write executable")?;
    for object in native.objects {
        std::fs::remove_file(object).ok();
    }
    std::fs::remove_file(object_path).ok();
    Ok(())
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
