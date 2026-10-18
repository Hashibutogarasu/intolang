use anyhow::Result;

use crate::llvm;

use crate::Codegen;
use crate::CodegenContext;
use crate::program;

/// Generate `main` from a lowered message.
///
/// # Safety
///
/// `context`, `module` and `builder` must be valid LLVM objects that belong
/// together.
pub unsafe fn generate(
    context: *mut llvm::LLVMContext,
    module: *mut llvm::LLVMModule,
    builder: *mut llvm::LLVMBuilder,
    message: &str,
) -> Result<()> {
    unsafe {
        let mut ctx = CodegenContext::create(context, module, builder)?;
        let program = program::parse(message)?;
        for class in &program.classes {
            class.declare(&mut ctx)?;
        }
        for class in &program.classes {
            class.generate(&mut ctx)?;
        }
        for node in &program.statements {
            let result = node.generate(&mut ctx)?;
            let _ = result.into_value();
        }
        ctx.finish()
    }
}
