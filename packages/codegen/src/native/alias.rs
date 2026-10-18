use anyhow::{Error, Result};
use parser::Record;

use crate::{Codegen, CodegenContext, CodegenResult};

/// `move` and `borrow`: the target slot holds the pointer the source holds.
pub struct NativeAlias {
    target: String,
    source: String,
}

impl TryFrom<&Record> for NativeAlias {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        Ok(Self {
            target: record.get(0).to_owned(),
            source: record.get(1).to_owned(),
        })
    }
}

impl Codegen for NativeAlias {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            ctx.flush_text()?;
            let ty = ctx
                .slots
                .get(&self.source)
                .map(|slot| slot.ty)
                .ok_or_else(|| anyhow::anyhow!("Native value `{}` has no storage", self.source))?;
            let value = ctx.load_slot(&self.source)?;
            ctx.store_slot(&self.target, ty, value);
            Ok(CodegenResult::unit())
        }
    }
}
