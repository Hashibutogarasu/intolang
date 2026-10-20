use errors::{Error, Result};
use native::NativeArg;
use parser::Record;

use crate::llvm;
use llvm::LLVMIntPredicate;
use llvm::core::*;

use crate::{Codegen, CodegenContext, CodegenResult};

/// `NATIVE_EMIT`: call every registered handler of an event with the given
/// arguments. Fields: the event, then one native argument per field. Handlers
/// registered while the event is emitted wait for the next emit.
pub struct NativeEmit {
    event: String,
    args: Vec<NativeArg>,
}

impl TryFrom<&Record> for NativeEmit {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        let args = record
            .fields()
            .iter()
            .skip(1)
            .map(|field| field.parse())
            .collect::<std::result::Result<Vec<_>, errors::Error>>()?;
        Ok(Self {
            event: record.get(0).to_owned(),
            args,
        })
    }
}

impl Codegen for NativeEmit {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            ctx.flush_text()?;
            let table = ctx.event_table(&self.event)?;
            let integer = LLVMInt32TypeInContext(ctx.context);
            let pointer = LLVMPointerType(LLVMInt8TypeInContext(ctx.context), 0);
            let (mut params, mut values) = ctx.native_call_arguments(&self.args)?;
            let function_type = LLVMFunctionType(
                LLVMVoidTypeInContext(ctx.context),
                params.as_mut_ptr(),
                params.len() as u32,
                0,
            );
            let count = LLVMBuildLoad2(ctx.builder, integer, table.count, ctx.empty.as_ptr());
            ctx.counted_loop(count, |ctx, index| {
                let entry = ctx.array_entry(table.handlers_type, table.handlers, index);
                let handler = LLVMBuildLoad2(ctx.builder, pointer, entry, ctx.empty.as_ptr());
                let present = LLVMBuildICmp(
                    ctx.builder,
                    LLVMIntPredicate::LLVMIntNE,
                    handler,
                    LLVMConstNull(pointer),
                    ctx.empty.as_ptr(),
                );
                let call_label = ctx.label("emit.call");
                let skip_label = ctx.label("emit.skip");
                let call = ctx.append_block(&call_label)?;
                let skip = ctx.append_block(&skip_label)?;
                LLVMBuildCondBr(ctx.builder, present, call, skip);
                LLVMPositionBuilderAtEnd(ctx.builder, call);
                LLVMBuildCall2(
                    ctx.builder,
                    function_type,
                    handler,
                    values.as_mut_ptr(),
                    values.len() as u32,
                    ctx.empty.as_ptr(),
                );
                LLVMBuildBr(ctx.builder, skip);
                LLVMPositionBuilderAtEnd(ctx.builder, skip);
                Ok(())
            })?;
            Ok(CodegenResult::unit())
        }
    }
}
