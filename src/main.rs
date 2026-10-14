extern crate llvm_sys as llvm;
#[macro_use]
extern crate anyhow;

use std::ffi::CString;
use std::mem::MaybeUninit;

use llvm::analysis::{LLVMVerifierFailureAction, LLVMVerifyModule};
use llvm::core::*;
use llvm::target::{LLVM_InitializeNativeAsmPrinter, LLVM_InitializeNativeTarget};
use llvm::target_machine::*;
use llvm::*;

use anyhow::{Context, Result};

unsafe fn gen_code(ctx: *mut LLVMContext, m: *mut LLVMModule, b: *mut LLVMBuilder) -> Result<()> {
    let emp_str = CString::new("")?;
    let mut param_ty = vec![LLVMPointerType(LLVMInt8TypeInContext(ctx), 0)];
    let printf_ty = LLVMFunctionType(LLVMVoidTypeInContext(ctx), param_ty.as_mut_ptr(), 0, 1);
    let name = CString::new("printf")?;
    let printf_body = LLVMAddFunction(m, name.as_ptr(), printf_ty);

    let mut param_ty = vec![];
    let main_ty = LLVMFunctionType(LLVMInt64TypeInContext(ctx), param_ty.as_mut_ptr(), 0, 0);
    let name = CString::new("main")?;
    let main_body = LLVMAddFunction(m, name.as_ptr(), main_ty);

    let name = CString::new("entry")?;
    let bb_entry = LLVMAppendBasicBlockInContext(ctx, main_body, name.as_ptr());
    LLVMPositionBuilderAtEnd(b, bb_entry);

    let s = CString::new("Hello, World!\n")?;
    let mut arg = vec![LLVMBuildGlobalString(b, s.as_ptr(), emp_str.as_ptr())];
    LLVMBuildCall2(
        b,
        printf_ty,
        printf_body,
        arg.as_mut_ptr(),
        1,
        emp_str.as_ptr(),
    );

    LLVMBuildRet(b, LLVMConstInt(LLVMInt64TypeInContext(ctx), 0, 1));

    let mut err = MaybeUninit::uninit().assume_init();
    let is_err = LLVMVerifyModule(
        m,
        LLVMVerifierFailureAction::LLVMPrintMessageAction,
        &mut err,
    );
    if is_err != 0 {
        let err = CString::from_raw(err);
        let err_ = err.clone();
        LLVMDisposeMessage(err.into_raw());
        return Err(anyhow!("Failed to verify main module: {}", err_.to_str()?));
    }
    Ok(())
}

unsafe fn get_target_machine() -> Result<*mut LLVMOpaqueTargetMachine> {
    LLVM_InitializeNativeTarget();
    LLVM_InitializeNativeAsmPrinter();

    let cpu = LLVMGetHostCPUName();
    let feat = LLVMGetHostCPUFeatures();
    let triple = LLVMGetDefaultTargetTriple();

    let mut target = MaybeUninit::uninit().assume_init();
    let mut err = MaybeUninit::uninit().assume_init();

    let is_err = LLVMGetTargetFromTriple(triple, &mut target, &mut err);
    if is_err != 0 {
        let err = CString::from_raw(err);
        let err_ = err.clone();
        LLVMDisposeMessage(err.into_raw());
        return Err(anyhow!(
            "Failed to get target from triple: {}",
            err_.to_str()?
        ));
    }

    let tm = LLVMCreateTargetMachine(
        target,
        triple,
        cpu,
        feat,
        LLVMCodeGenOptLevel::LLVMCodeGenLevelDefault,
        LLVMRelocMode::LLVMRelocPIC,
        LLVMCodeModel::LLVMCodeModelDefault,
    );
    LLVMDisposeMessage(cpu);
    LLVMDisposeMessage(feat);
    LLVMDisposeMessage(triple);
    Ok(tm)
}

fn compile(cc: String) -> Result<()> {
    unsafe {
        let ctx = LLVMContextCreate();
        let mod_name = CString::new("test")?;
        let m = LLVMModuleCreateWithNameInContext(mod_name.as_ptr(), ctx);
        let b = LLVMCreateBuilderInContext(ctx);

        gen_code(ctx, m, b).context("Error while generating body")?;

        let tm = get_target_machine().context("Failed while getting target machine")?;

        let mut err = MaybeUninit::uninit().assume_init();
        let is_err = LLVMTargetMachineEmitToFile(
            tm,
            m,
            CString::new("test.o")?.into_raw(),
            LLVMCodeGenFileType::LLVMObjectFile,
            &mut err,
        );
        if is_err != 0 {
            let err = CString::from_raw(err);
            let err_ = err.clone();
            LLVMDisposeMessage(err.into_raw());
            return Err(anyhow!("Failed to emit object file: {}", err_.to_str()?));
        }
        LLVMDisposeTargetMachine(tm);
        LLVMDisposeBuilder(b);
        LLVMDisposeModule(m);
        LLVMContextDispose(ctx);
    }

    let ext = if cfg!(windows) { ".exe" } else { "" };
    let compiling = std::process::Command::new(cc)
        .args(vec!["test.o".into(), "-o".into(), format!("test{}", ext)])
        .output()?;

    let stderr = String::from_utf8(compiling.stderr)?;
    let status = compiling
        .status
        .code()
        .ok_or(anyhow!("Apparently the compiler was killed"))?;
    if status != 0 {
        return Err(anyhow!(
            "Compile failed with code {}\nstderr: {}",
            status,
            stderr
        ));
    }

    Ok(())
}

fn main() {
    let cc = std::env::var("CC").unwrap_or("gcc".into());
    compile(cc)
        .context("Error while compiling")
        .unwrap_or_else(|e| {
            eprintln!("{:?}", e);
        });
}
