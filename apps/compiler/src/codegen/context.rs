use std::collections::HashMap;
use std::ffi::CString;
use std::ptr;

use anyhow::Result;

use crate::llvm;
use llvm::core::*;
use llvm::*;

/// Storage for a lowered native value. Values live in stack slots so that loop
/// carried variables observe updates made in the loop body.
pub struct Slot {
    pub ptr: *mut LLVMValue,
    pub ty: *mut LLVMType,
}

pub type Slots = HashMap<String, Slot>;

/// Shared LLVM state threaded through every [`super::Codegen`] node.
pub struct CodegenContext {
    pub context: *mut LLVMContext,
    pub module: *mut LLVMModule,
    pub builder: *mut LLVMBuilder,
    pub empty: CString,
    pub named_types: HashMap<String, *mut LLVMType>,
    pub class_fields: HashMap<String, Vec<(String, *mut LLVMType)>>,
    pub printf_type: *mut LLVMType,
    pub printf: *mut LLVMValue,
    pub main: *mut LLVMValue,
    pub entry: *mut LLVMBasicBlock,
    pub slots: Slots,
    pub loop_blocks: Vec<(*mut LLVMBasicBlock, *mut LLVMBasicBlock)>,
    pub if_blocks: Vec<(*mut LLVMBasicBlock, *mut LLVMBasicBlock, bool)>,
    pub text: String,
}

impl CodegenContext {
    pub(crate) unsafe fn create(
        context: *mut LLVMContext,
        module: *mut LLVMModule,
        builder: *mut LLVMBuilder,
    ) -> Result<Self> {
        unsafe {
            let empty = CString::new("")?;
            let mut printf_params = vec![LLVMPointerType(LLVMInt8TypeInContext(context), 0)];
            let printf_type = LLVMFunctionType(
                LLVMVoidTypeInContext(context),
                printf_params.as_mut_ptr(),
                0,
                1,
            );
            let printf_name = CString::new("printf")?;
            let printf = LLVMAddFunction(module, printf_name.as_ptr(), printf_type);

            let main_type =
                LLVMFunctionType(LLVMInt64TypeInContext(context), ptr::null_mut(), 0, 0);
            let main_name = CString::new("main")?;
            let main = LLVMAddFunction(module, main_name.as_ptr(), main_type);
            let entry_name = CString::new("entry")?;
            let entry = LLVMAppendBasicBlockInContext(context, main, entry_name.as_ptr());
            LLVMPositionBuilderAtEnd(builder, entry);

            Ok(Self {
                context,
                module,
                builder,
                empty,
                named_types: HashMap::new(),
                class_fields: HashMap::new(),
                printf_type,
                printf,
                main,
                entry,
                slots: Slots::new(),
                loop_blocks: Vec::new(),
                if_blocks: Vec::new(),
                text: String::new(),
            })
        }
    }
}
