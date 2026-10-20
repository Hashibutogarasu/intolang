use errors::Result;

use crate::llvm;
use llvm::LLVMValue;

use crate::CodegenContext;

/// A node of the lowered Into message that knows how to emit its LLVM code.
///
/// Every statement tag produced by the backend (`NATIVE`, `NATIVE_PRINT`,
/// `NATIVE_IF_BEGIN`, ...) is represented by one type implementing this trait,
/// so code generation is driven by walking the AST instead of matching on raw
/// message lines.
pub trait Codegen {
    /// Generate the LLVM for this node, returning what the node produced.
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult>;
}

/// The structure produced by generating a single AST node.
#[derive(Default)]
pub struct CodegenResult {
    /// The LLVM value produced by the node, when it yields one (for example a
    /// comparison used as an `if`/`while` condition).
    value: Option<*mut LLVMValue>,
}

impl CodegenResult {
    /// A node that only mutates the module or the pending text buffer.
    pub fn unit() -> Self {
        Self { value: None }
    }

    /// A node that produced an SSA value.
    pub fn value(value: *mut LLVMValue) -> Self {
        Self { value: Some(value) }
    }

    /// The produced value, if any.
    pub fn into_value(self) -> Option<*mut LLVMValue> {
        self.value
    }
}
