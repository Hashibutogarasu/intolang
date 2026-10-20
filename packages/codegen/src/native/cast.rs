use errors::{Error, Errors, Result};
use native::CastAssign;
use parser::Record;

use crate::llvm;
use llvm::LLVMTypeKind;
use llvm::core::*;

use crate::{Codegen, CodegenContext, CodegenResult};

/// `NATIVE_CAST`: convert a scalar slot to another scalar type.
pub struct NativeCast {
    cast: CastAssign,
}

impl TryFrom<&Record> for NativeCast {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        Ok(Self {
            cast: CastAssign::try_from(record)?,
        })
    }
}

impl Codegen for NativeCast {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            let from = ctx.into_type(&self.cast.from);
            let to = ctx.into_type(&self.cast.to);
            let value = ctx.load_slot(&self.cast.source)?;
            let is_float = |ty| {
                matches!(
                    LLVMGetTypeKind(ty),
                    LLVMTypeKind::LLVMFloatTypeKind | LLVMTypeKind::LLVMDoubleTypeKind
                )
            };
            let unsigned = self.cast.from == "Boolean";
            let name = ctx.empty.as_ptr();
            let result = match (is_float(from), is_float(to)) {
                (false, false) => {
                    LLVMBuildIntCast2(ctx.builder, value, to, i32::from(!unsigned), name)
                }
                (false, true) if unsigned => LLVMBuildUIToFP(ctx.builder, value, to, name),
                (false, true) => LLVMBuildSIToFP(ctx.builder, value, to, name),
                (true, false) => LLVMBuildFPToSI(ctx.builder, value, to, name),
                (true, true) => LLVMBuildFPCast(ctx.builder, value, to, name),
            };
            if result.is_null() {
                return Err(errors::Error::from(Errors::CannotCast {
                    from: self.cast.from.to_string(),
                    to: self.cast.to.to_string(),
                }));
            }
            ctx.store_slot(&self.cast.target, to, result);
            Ok(CodegenResult::unit())
        }
    }
}
