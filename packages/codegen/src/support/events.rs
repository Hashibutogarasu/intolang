use errors::{Errors, Result};
use std::ffi::CString;
use std::mem;

use crate::llvm;
use llvm::core::*;
use llvm::*;

use crate::CodegenContext;
use crate::context::{EventTable, SavedFunction, Slot, Slots};

/// How many handlers one event keeps; registrations beyond it are ignored.
pub(crate) const EVENT_CAPACITY: u64 = 64;

impl CodegenContext {
    /// A name no other generated block or slot uses.
    pub(crate) fn label(&mut self, prefix: &str) -> String {
        self.labels += 1;
        format!("{prefix}.{}", self.labels)
    }

    /// The handler table globals of `event`, created on first use.
    pub(crate) unsafe fn event_table(&mut self, event: &str) -> Result<EventTable> {
        if let Some(table) = self.events.get(event) {
            return Ok(*table);
        }
        unsafe {
            let pointer = LLVMPointerType(LLVMInt8TypeInContext(self.context), 0);
            let integer = LLVMInt32TypeInContext(self.context);
            let handlers_type = LLVMArrayType2(pointer, EVENT_CAPACITY);
            let ids_type = LLVMArrayType2(integer, EVENT_CAPACITY);
            let global = |name: String, ty: *mut LLVMType| -> Result<*mut LLVMValue> {
                let name = CString::new(name)?;
                let global = LLVMAddGlobal(self.module, ty, name.as_ptr());
                LLVMSetInitializer(global, LLVMConstNull(ty));
                Ok(global)
            };
            let handlers = global(format!("__event_{event}_handlers"), handlers_type)?;
            let ids = global(format!("__event_{event}_ids"), ids_type)?;
            let count = global(format!("__event_{event}_count"), integer)?;
            let next = global(format!("__event_{event}_next"), integer)?;
            LLVMSetInitializer(next, LLVMConstInt(integer, 1, 0));
            let table = EventTable {
                handlers,
                handlers_type,
                ids,
                ids_type,
                count,
                next,
            };
            self.events.insert(event.to_owned(), table);
            Ok(table)
        }
    }

    /// The address of entry `index` of the array global `array`.
    pub(crate) unsafe fn array_entry(
        &self,
        array_type: *mut LLVMType,
        array: *mut LLVMValue,
        index: *mut LLVMValue,
    ) -> *mut LLVMValue {
        unsafe {
            let zero = LLVMConstInt(LLVMInt32TypeInContext(self.context), 0, 0);
            let mut indices = [zero, index];
            LLVMBuildGEP2(
                self.builder,
                array_type,
                array,
                indices.as_mut_ptr(),
                2,
                self.empty.as_ptr(),
            )
        }
    }

    /// Generate `body(index)` for every `index` below `count`.
    pub(crate) unsafe fn counted_loop(
        &mut self,
        count: *mut LLVMValue,
        body: impl FnOnce(&mut Self, *mut LLVMValue) -> Result<()>,
    ) -> Result<()> {
        unsafe {
            let integer = LLVMInt32TypeInContext(self.context);
            let counter = self.label("__loop_index");
            let index = self.slot_ptr(&counter, integer);
            LLVMBuildStore(self.builder, LLVMConstInt(integer, 0, 0), index);
            let condition_label = self.label("loop.condition");
            let body_label = self.label("loop.body");
            let done_label = self.label("loop.done");
            let condition = self.append_block(&condition_label)?;
            let inside = self.append_block(&body_label)?;
            let done = self.append_block(&done_label)?;
            LLVMBuildBr(self.builder, condition);

            LLVMPositionBuilderAtEnd(self.builder, condition);
            let current = LLVMBuildLoad2(self.builder, integer, index, self.empty.as_ptr());
            let below = LLVMBuildICmp(
                self.builder,
                LLVMIntPredicate::LLVMIntSLT,
                current,
                count,
                self.empty.as_ptr(),
            );
            LLVMBuildCondBr(self.builder, below, inside, done);

            LLVMPositionBuilderAtEnd(self.builder, inside);
            let current = LLVMBuildLoad2(self.builder, integer, index, self.empty.as_ptr());
            body(self, current)?;
            let current = LLVMBuildLoad2(self.builder, integer, index, self.empty.as_ptr());
            let step = LLVMBuildAdd(
                self.builder,
                current,
                LLVMConstInt(integer, 1, 0),
                self.empty.as_ptr(),
            );
            LLVMBuildStore(self.builder, step, index);
            LLVMBuildBr(self.builder, condition);

            LLVMPositionBuilderAtEnd(self.builder, done);
            Ok(())
        }
    }

    /// Move the value held by the slot `name` into a global so that handler
    /// functions can reach it.
    pub(crate) unsafe fn promote_to_global(&mut self, name: &str) -> Result<()> {
        if self.globals.contains(name) {
            return Ok(());
        }
        unsafe {
            let slot = *self.slots.get(name).ok_or_else(|| {
                errors::Error::from(Errors::CapturedValueNoStorage {
                    name: name.to_string(),
                })
            })?;
            let global_name = CString::new(format!("__global_{name}"))?;
            let global = LLVMAddGlobal(self.module, slot.ty, global_name.as_ptr());
            LLVMSetInitializer(global, LLVMConstNull(slot.ty));
            LLVMReplaceAllUsesWith(slot.ptr, global);
            LLVMInstructionEraseFromParent(slot.ptr);
            self.slots.insert(
                name.to_owned(),
                Slot {
                    ptr: global,
                    ty: slot.ty,
                },
            );
            self.globals.insert(name.to_owned());
        }
        Ok(())
    }

    /// Start generating the function `name`, which takes `params` and
    /// reaches the `captures` through globals.
    pub(crate) unsafe fn begin_function(
        &mut self,
        name: &str,
        params: &[(String, String)],
        captures: &[String],
        return_type: Option<&str>,
    ) -> Result<()> {
        unsafe {
            self.flush_text()?;
            for capture in captures {
                self.promote_to_global(capture)?;
            }
            let mut types: Vec<*mut LLVMType> =
                params.iter().map(|(_, ty)| self.into_type(ty)).collect();
            let returns = return_type
                .filter(|ty| !matches!(*ty, "" | "Void"))
                .map(|ty| self.into_type(ty));
            self.returns.push(returns);
            let function_type = LLVMFunctionType(
                returns.unwrap_or_else(|| LLVMVoidTypeInContext(self.context)),
                types.as_mut_ptr(),
                types.len() as u32,
                0,
            );
            let function_name = CString::new(name)?;
            let function = LLVMAddFunction(self.module, function_name.as_ptr(), function_type);
            let entry_name = CString::new("entry")?;
            let entry = LLVMAppendBasicBlockInContext(self.context, function, entry_name.as_ptr());

            let shared: Slots = self
                .slots
                .iter()
                .filter(|(slot, _)| self.globals.contains(*slot))
                .map(|(slot, value)| (slot.clone(), *value))
                .collect();
            let saved = SavedFunction {
                main: self.main,
                entry: self.entry,
                block: LLVMGetInsertBlock(self.builder),
                slots: mem::replace(&mut self.slots, shared),
                loop_blocks: mem::take(&mut self.loop_blocks),
                if_blocks: mem::take(&mut self.if_blocks),
            };
            self.functions.push(saved);
            self.main = function;
            self.entry = entry;
            LLVMPositionBuilderAtEnd(self.builder, entry);
            for (index, ((param, _), ty)) in params.iter().zip(types).enumerate() {
                let value = LLVMGetParam(function, index as u32);
                self.store_slot(param, ty, value);
            }
        }
        Ok(())
    }

    /// Finish the function started by [`Self::begin_function`] and return to
    /// the one that was being generated.
    pub(crate) unsafe fn end_function(&mut self) -> Result<()> {
        unsafe {
            self.flush_text()?;
            match self.returns.pop().flatten() {
                Some(ty) => LLVMBuildRet(self.builder, LLVMConstNull(ty)),
                None => LLVMBuildRetVoid(self.builder),
            };
            let saved = self
                .functions
                .pop()
                .ok_or_else(|| errors::Error::from(Errors::HandlerEndWithoutHandler))?;
            self.main = saved.main;
            self.entry = saved.entry;
            self.slots = saved.slots;
            self.loop_blocks = saved.loop_blocks;
            self.if_blocks = saved.if_blocks;
            LLVMPositionBuilderAtEnd(self.builder, saved.block);
        }
        Ok(())
    }
}
