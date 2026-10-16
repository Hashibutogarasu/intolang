use anyhow::{Result, anyhow};
use std::{
    collections::HashMap,
    ffi::{CStr, CString},
    ptr,
};

use crate::llvm;
use llvm::analysis::{LLVMVerifierFailureAction, LLVMVerifyModule};
use llvm::core::*;
use llvm::*;

/// Storage for a lowered native value. Values live in stack slots so that loop
/// carried variables observe updates made in the loop body.
struct Slot {
    ptr: *mut LLVMValue,
    ty: *mut LLVMType,
}

type Slots = HashMap<String, Slot>;

pub(super) unsafe fn generate(
    context: *mut LLVMContext,
    module: *mut LLVMModule,
    builder: *mut LLVMBuilder,
    message: &str,
) -> Result<()> {
    unsafe {
        let empty = CString::new("")?;
        let mut named_types = HashMap::new();
        let mut class_lines = Vec::new();
        for line in message.lines() {
            if let Some(spec) = line.strip_prefix("\u{1e}NATIVE_CLASS\t") {
                let name = spec.split('\t').next().unwrap_or_default();
                let name_c = CString::new(name)?;
                named_types.insert(
                    name.to_owned(),
                    LLVMStructCreateNamed(context, name_c.as_ptr()),
                );
                class_lines.push(spec);
            }
        }
        let mut class_fields = HashMap::new();
        for spec in class_lines {
            let mut parts = spec.split('\t');
            let name = parts.next().unwrap_or_default();
            let mut fields = Vec::new();
            let mut types = Vec::new();
            for part in parts {
                let (field, ty) = part.split_once(':').unwrap_or((part, "String"));
                let llvm_type = into_type(context, ty, &named_types);
                fields.push((field.to_owned(), llvm_type));
                types.push(llvm_type);
            }
            let mut field_types = types;
            LLVMStructSetBody(
                *named_types.get(name).unwrap(),
                field_types.as_mut_ptr(),
                field_types.len() as u32,
                0,
            );
            class_fields.insert(name.to_owned(), fields);
        }
        let mut printf_params = vec![LLVMPointerType(LLVMInt8TypeInContext(context), 0)];
        let printf_type = LLVMFunctionType(
            LLVMVoidTypeInContext(context),
            printf_params.as_mut_ptr(),
            0,
            1,
        );
        let printf_name = CString::new("printf")?;
        let printf = LLVMAddFunction(module, printf_name.as_ptr(), printf_type);

        let mut main_params = vec![];
        let main_type = LLVMFunctionType(
            LLVMInt64TypeInContext(context),
            main_params.as_mut_ptr(),
            0,
            0,
        );
        let main_name = CString::new("main")?;
        let main = LLVMAddFunction(module, main_name.as_ptr(), main_type);
        let entry_name = CString::new("entry")?;
        let entry = LLVMAppendBasicBlockInContext(context, main, entry_name.as_ptr());
        LLVMPositionBuilderAtEnd(builder, entry);

        let mut text = String::new();
        let mut slots = Slots::new();
        let mut loop_blocks = Vec::new();
        let mut if_blocks = Vec::new();
        for line in message.split_inclusive('\n') {
            if line.strip_prefix("\u{1e}NATIVE_CLASS\t").is_some() {
                continue;
            } else if let Some(call) = line.strip_prefix("\u{1e}NATIVE_IF_CALL\t") {
                emit_text(context, builder, printf_type, printf, &empty, &text)?;
                text.clear();
                let mut parts = call.trim_end_matches('\n').split('\t');
                let _package = parts.next();
                let _language = parts.next();
                let _source = parts.next();
                let target = CString::new(parts.next().unwrap_or_default())?;
                let return_type = parts.next().unwrap_or_default();
                let _assigned_to = parts.next();
                let (mut params, mut args) =
                    native_call_arguments(context, builder, &empty, &slots, parts)?;
                let result_type = into_type(context, return_type, &named_types);
                if result_type != LLVMInt1TypeInContext(context) {
                    anyhow::bail!("Native if condition must return Boolean");
                }
                let function_type =
                    LLVMFunctionType(result_type, params.as_mut_ptr(), args.len() as u32, 0);
                let existing = LLVMGetNamedFunction(module, target.as_ptr());
                let function = if existing.is_null() {
                    LLVMAddFunction(module, target.as_ptr(), function_type)
                } else {
                    existing
                };
                let condition = LLVMBuildCall2(
                    builder,
                    function_type,
                    function,
                    args.as_mut_ptr(),
                    args.len() as u32,
                    empty.as_ptr(),
                );
                let then_block =
                    LLVMAppendBasicBlockInContext(context, main, CString::new("if.then")?.as_ptr());
                let else_block =
                    LLVMAppendBasicBlockInContext(context, main, CString::new("if.else")?.as_ptr());
                let merge_block = LLVMAppendBasicBlockInContext(
                    context,
                    main,
                    CString::new("if.merge")?.as_ptr(),
                );
                LLVMBuildCondBr(builder, condition, then_block, else_block);
                if_blocks.push((else_block, merge_block, false));
                LLVMPositionBuilderAtEnd(builder, then_block);
            } else if let Some(flag) = line.strip_prefix("\u{1e}NATIVE_IF_CONST\t") {
                emit_text(context, builder, printf_type, printf, &empty, &text)?;
                text.clear();
                let value = flag.trim();
                let condition = LLVMConstInt(
                    LLVMInt1TypeInContext(context),
                    if value == "1" { 1 } else { 0 },
                    0,
                );
                let then_block =
                    LLVMAppendBasicBlockInContext(context, main, CString::new("if.then")?.as_ptr());
                let else_block =
                    LLVMAppendBasicBlockInContext(context, main, CString::new("if.else")?.as_ptr());
                let merge_block = LLVMAppendBasicBlockInContext(
                    context,
                    main,
                    CString::new("if.merge")?.as_ptr(),
                );
                LLVMBuildCondBr(builder, condition, then_block, else_block);
                if_blocks.push((else_block, merge_block, false));
                LLVMPositionBuilderAtEnd(builder, then_block);
            } else if let Some(call) = line.strip_prefix("\u{1e}NATIVE_IF_BEGIN\t") {
                emit_text(context, builder, printf_type, printf, &empty, &text)?;
                text.clear();
                let condition =
                    build_comparison(context, builder, &empty, &slots, &named_types, call)?;
                let then_block =
                    LLVMAppendBasicBlockInContext(context, main, CString::new("if.then")?.as_ptr());
                let else_block =
                    LLVMAppendBasicBlockInContext(context, main, CString::new("if.else")?.as_ptr());
                let merge_block = LLVMAppendBasicBlockInContext(
                    context,
                    main,
                    CString::new("if.merge")?.as_ptr(),
                );
                LLVMBuildCondBr(builder, condition, then_block, else_block);
                if_blocks.push((else_block, merge_block, false));
                LLVMPositionBuilderAtEnd(builder, then_block);
            } else if line.starts_with("\u{1e}NATIVE_IF_ELSE") {
                let (else_block, merge_block, has_else) = if_blocks
                    .last_mut()
                    .ok_or_else(|| anyhow!("`else` has no matching if"))?;
                if *has_else {
                    anyhow::bail!("An if block cannot contain multiple else branches");
                }
                LLVMBuildBr(builder, *merge_block);
                *has_else = true;
                LLVMPositionBuilderAtEnd(builder, *else_block);
            } else if line.starts_with("\u{1e}NATIVE_IF_END") {
                let (else_block, merge_block, has_else) = if_blocks
                    .pop()
                    .ok_or_else(|| anyhow!("if end has no matching if"))?;
                LLVMBuildBr(builder, merge_block);
                if !has_else {
                    LLVMPositionBuilderAtEnd(builder, else_block);
                    LLVMBuildBr(builder, merge_block);
                }
                LLVMPositionBuilderAtEnd(builder, merge_block);
            } else if line.starts_with("\u{1e}NATIVE_BREAK") {
                let (_, exit) = loop_blocks
                    .last()
                    .ok_or_else(|| anyhow!("Native break has no matching loop"))?;
                LLVMBuildBr(builder, *exit);
                let unreachable = LLVMAppendBasicBlockInContext(
                    context,
                    main,
                    CString::new("after.break")?.as_ptr(),
                );
                LLVMPositionBuilderAtEnd(builder, unreachable);
            } else if let Some(call) = line.strip_prefix("\u{1e}NATIVE_FIELD_PRINT\t") {
                emit_text(context, builder, printf_type, printf, &empty, &text)?;
                text.clear();
                let mut parts = call.trim_end_matches('\n').split('\t');
                let variable = parts.next().unwrap_or_default();
                let class_name = parts.next().unwrap_or_default();
                let field_index = parts.next().unwrap_or_default().parse::<u32>()?;
                let field_type = class_fields
                    .get(class_name)
                    .and_then(|fields| fields.get(field_index as usize))
                    .map(|(_, ty)| *ty)
                    .ok_or_else(|| anyhow!("Unknown field index for class `{class_name}`"))?;
                let _declared_field_type = parts.next();
                let newline = parts.next().unwrap_or_default() == "1";
                let object = load_slot(builder, &slots, variable, &empty)?;
                let struct_type = *named_types
                    .get(class_name)
                    .ok_or_else(|| anyhow!("Unknown native class `{class_name}`"))?;
                let field_pointer =
                    LLVMBuildStructGEP2(builder, struct_type, object, field_index, empty.as_ptr());
                let loaded = LLVMBuildLoad2(builder, field_type, field_pointer, empty.as_ptr());
                print_scalar(
                    context,
                    builder,
                    printf_type,
                    printf,
                    &empty,
                    field_type,
                    loaded,
                    newline,
                )?;
            } else if let Some(call) = line.strip_prefix("\u{1e}NATIVE_PRINT\t") {
                emit_text(context, builder, printf_type, printf, &empty, &text)?;
                text.clear();
                let mut parts = call.trim_end_matches('\n').split('\t');
                let variable = parts.next().unwrap_or_default();
                let return_type = parts.next().unwrap_or_default();
                let newline = parts.next().unwrap_or_default() == "1";
                let ty = into_type(context, return_type, &named_types);
                let value = load_slot(builder, &slots, variable, &empty)?;
                print_scalar(
                    context,
                    builder,
                    printf_type,
                    printf,
                    &empty,
                    ty,
                    value,
                    newline,
                )?;
            } else if let Some(call) = line.strip_prefix("\u{1e}NATIVE_PRINT_CONST\t") {
                emit_text(context, builder, printf_type, printf, &empty, &text)?;
                text.clear();
                let mut parts = call.trim_end_matches('\n').split('\t');
                let return_type = parts.next().unwrap_or_default();
                let operand = parts.next().unwrap_or_default();
                let newline = parts.next().unwrap_or_default() == "1";
                let ty = into_type(context, return_type, &named_types);
                let (_, value) = operand.split_once(':').unwrap_or(("", operand));
                let constant = const_for_type(context, ty, value)?;
                print_scalar(
                    context,
                    builder,
                    printf_type,
                    printf,
                    &empty,
                    ty,
                    constant,
                    newline,
                )?;
            } else if let Some(call) = line.strip_prefix("\u{1e}NATIVE_LOOP_BEGIN\t") {
                emit_text(context, builder, printf_type, printf, &empty, &text)?;
                text.clear();
                let mut parts = call.trim_end_matches('\n').split('\t');
                let _package = parts.next();
                let _language = parts.next();
                let _source = parts.next();
                let target = CString::new(parts.next().unwrap_or_default())?;
                let return_type = parts.next().unwrap_or_default();
                let _assigned_to = parts.next().unwrap_or_default();
                let condition_block = LLVMAppendBasicBlockInContext(
                    context,
                    main,
                    CString::new("while.condition")?.as_ptr(),
                );
                let body_block = LLVMAppendBasicBlockInContext(
                    context,
                    main,
                    CString::new("while.body")?.as_ptr(),
                );
                let exit_block = LLVMAppendBasicBlockInContext(
                    context,
                    main,
                    CString::new("while.exit")?.as_ptr(),
                );
                LLVMBuildBr(builder, condition_block);
                LLVMPositionBuilderAtEnd(builder, condition_block);
                let (mut params, function_args) =
                    native_call_arguments(context, builder, &empty, &slots, parts)?;
                let expected = into_type(context, return_type, &named_types);
                if expected != LLVMInt1TypeInContext(context) {
                    anyhow::bail!("Native loop condition must return Boolean");
                }
                let function_type =
                    LLVMFunctionType(expected, params.as_mut_ptr(), function_args.len() as u32, 0);
                let existing = LLVMGetNamedFunction(module, target.as_ptr());
                let function = if existing.is_null() {
                    LLVMAddFunction(module, target.as_ptr(), function_type)
                } else {
                    existing
                };
                let condition = LLVMBuildCall2(
                    builder,
                    function_type,
                    function,
                    function_args.as_ptr() as *mut _,
                    function_args.len() as u32,
                    empty.as_ptr(),
                );
                LLVMBuildCondBr(builder, condition, body_block, exit_block);
                loop_blocks.push((condition_block, exit_block));
                LLVMPositionBuilderAtEnd(builder, body_block);
            } else if let Some(call) = line.strip_prefix("\u{1e}NATIVE_WHILE_BEGIN\t") {
                emit_text(context, builder, printf_type, printf, &empty, &text)?;
                text.clear();
                let condition_block = LLVMAppendBasicBlockInContext(
                    context,
                    main,
                    CString::new("while.condition")?.as_ptr(),
                );
                let body_block = LLVMAppendBasicBlockInContext(
                    context,
                    main,
                    CString::new("while.body")?.as_ptr(),
                );
                let exit_block = LLVMAppendBasicBlockInContext(
                    context,
                    main,
                    CString::new("while.exit")?.as_ptr(),
                );
                LLVMBuildBr(builder, condition_block);
                LLVMPositionBuilderAtEnd(builder, condition_block);
                let condition =
                    build_comparison(context, builder, &empty, &slots, &named_types, call)?;
                LLVMBuildCondBr(builder, condition, body_block, exit_block);
                loop_blocks.push((condition_block, exit_block));
                LLVMPositionBuilderAtEnd(builder, body_block);
            } else if line.starts_with("\u{1e}NATIVE_LOOP_END") {
                let (condition, exit) = loop_blocks
                    .pop()
                    .ok_or_else(|| anyhow!("Native loop end has no matching loop"))?;
                LLVMBuildBr(builder, condition);
                LLVMPositionBuilderAtEnd(builder, exit);
            } else if let Some(call) = line.strip_prefix("\u{1e}NATIVE_ARITH\t") {
                let mut parts = call.trim_end_matches('\n').split('\t');
                let assigned = parts.next().unwrap_or_default().to_owned();
                let left_name = parts.next().unwrap_or_default();
                let operation = parts.next().unwrap_or_default();
                let operand = parts.next().unwrap_or_default();
                let return_type = parts.next().unwrap_or_default();
                let ty = into_type(context, return_type, &named_types);
                let left = load_slot(builder, &slots, left_name, &empty)?;
                let (kind, value) = operand.split_once(':').unwrap_or(("", operand));
                let right = if kind == "p" {
                    load_slot(builder, &slots, value, &empty)?
                } else {
                    const_for_type(context, ty, value)?
                };
                let is_float = matches!(
                    LLVMGetTypeKind(ty),
                    LLVMTypeKind::LLVMFloatTypeKind | LLVMTypeKind::LLVMDoubleTypeKind
                );
                let result = if is_float {
                    match operation {
                        "add" => LLVMBuildFAdd(builder, left, right, empty.as_ptr()),
                        "sub" => LLVMBuildFSub(builder, left, right, empty.as_ptr()),
                        "mul" => LLVMBuildFMul(builder, left, right, empty.as_ptr()),
                        "div" => LLVMBuildFDiv(builder, left, right, empty.as_ptr()),
                        _ => anyhow::bail!("Unsupported arithmetic operation `{operation}`"),
                    }
                } else {
                    match operation {
                        "add" => LLVMBuildAdd(builder, left, right, empty.as_ptr()),
                        "sub" => LLVMBuildSub(builder, left, right, empty.as_ptr()),
                        "mul" => LLVMBuildMul(builder, left, right, empty.as_ptr()),
                        "div" => LLVMBuildSDiv(builder, left, right, empty.as_ptr()),
                        _ => anyhow::bail!("Unsupported arithmetic operation `{operation}`"),
                    }
                };
                store_slot(builder, entry, &empty, &mut slots, &assigned, ty, result);
            } else if let Some(call) = line.strip_prefix("\u{1e}NATIVE_ASSIGN\t") {
                let mut parts = call.trim_end_matches('\n').split('\t');
                let target = parts.next().unwrap_or_default().to_owned();
                let return_type = parts.next().unwrap_or_default();
                let operand = parts.next().unwrap_or_default();
                let ty = into_type(context, return_type, &named_types);
                let (kind, value) = operand.split_once(':').unwrap_or(("", operand));
                let stored = if kind == "p" {
                    load_slot(builder, &slots, value, &empty)?
                } else if kind == "s" {
                    let decoded = decode_hex(value)?;
                    let value = CString::new(decoded)?;
                    LLVMBuildGlobalString(builder, value.as_ptr(), empty.as_ptr())
                } else {
                    const_for_type(context, ty, value)?
                };
                store_slot(builder, entry, &empty, &mut slots, &target, ty, stored);
            } else if let Some(call) = line.strip_prefix("\u{1e}NATIVE_LINK_ASSIGN\t") {
                emit_text(context, builder, printf_type, printf, &empty, &text)?;
                text.clear();
                let mut parts = call.trim_end_matches('\n').split('\t');
                let _package = parts.next().unwrap_or_default();
                let _language = parts.next().unwrap_or_default();
                let _source = parts.next().unwrap_or_default();
                let target = CString::new(parts.next().unwrap_or_default())?;
                let return_type = parts.next().unwrap_or_default();
                let assigned_to = parts.next().unwrap_or_default().to_owned();
                let (mut params, mut args) =
                    native_call_arguments(context, builder, &empty, &slots, parts)?;
                let result_type = if return_type == "Void" || return_type.is_empty() {
                    LLVMVoidTypeInContext(context)
                } else {
                    into_type(context, return_type, &named_types)
                };
                let function_type =
                    LLVMFunctionType(result_type, params.as_mut_ptr(), args.len() as u32, 0);
                let existing = LLVMGetNamedFunction(module, target.as_ptr());
                let function = if existing.is_null() {
                    LLVMAddFunction(module, target.as_ptr(), function_type)
                } else {
                    existing
                };
                let result = LLVMBuildCall2(
                    builder,
                    function_type,
                    function,
                    args.as_mut_ptr(),
                    args.len() as u32,
                    empty.as_ptr(),
                );
                if result_type != LLVMVoidTypeInContext(context) && !assigned_to.is_empty() {
                    store_slot(
                        builder,
                        entry,
                        &empty,
                        &mut slots,
                        &assigned_to,
                        result_type,
                        result,
                    );
                }
            } else if let Some(call) = line.strip_prefix("\u{1e}NATIVE\t") {
                emit_text(context, builder, printf_type, printf, &empty, &text)?;
                text.clear();
                let mut parts = call.trim_end_matches('\n').split('\t');
                let symbol = CString::new(parts.next().unwrap_or_default())?;
                let kind = parts.next().unwrap_or_default();
                let bytes = parts.next().unwrap_or_default();
                let argument = if kind == "s" {
                    let decoded = decode_hex(bytes)?;
                    let value = CString::new(decoded)?;
                    LLVMBuildGlobalString(builder, value.as_ptr(), empty.as_ptr())
                } else {
                    let value = bytes.parse::<i32>()?;
                    LLVMConstInt(LLVMInt32TypeInContext(context), value as u64, 1)
                };
                let argument_type = if kind == "s" {
                    LLVMPointerType(LLVMInt8TypeInContext(context), 0)
                } else {
                    LLVMInt32TypeInContext(context)
                };
                let mut params = vec![argument_type];
                let function_type =
                    LLVMFunctionType(LLVMVoidTypeInContext(context), params.as_mut_ptr(), 1, 0);
                let existing = LLVMGetNamedFunction(module, symbol.as_ptr());
                let function = if existing.is_null() {
                    LLVMAddFunction(module, symbol.as_ptr(), function_type)
                } else {
                    existing
                };
                let mut args = vec![argument];
                LLVMBuildCall2(
                    builder,
                    function_type,
                    function,
                    args.as_mut_ptr(),
                    1,
                    empty.as_ptr(),
                );
            } else {
                text.push_str(line);
            }
        }
        emit_text(context, builder, printf_type, printf, &empty, &text)?;
        LLVMBuildRet(builder, LLVMConstInt(LLVMInt64TypeInContext(context), 0, 1));

        let mut error = ptr::null_mut();
        if LLVMVerifyModule(
            module,
            LLVMVerifierFailureAction::LLVMPrintMessageAction,
            &mut error,
        ) != 0
        {
            let description = if error.is_null() {
                "unknown verifier error".into()
            } else {
                CStr::from_ptr(error).to_string_lossy().into_owned()
            };
            if !error.is_null() {
                LLVMDisposeMessage(error);
            }
            return Err(anyhow!("Failed to verify main module: {description}"));
        }
        Ok(())
    }
}

unsafe fn build_comparison(
    context: *mut LLVMContext,
    builder: *mut LLVMBuilder,
    empty: &CString,
    slots: &Slots,
    named_types: &HashMap<String, *mut LLVMType>,
    call: &str,
) -> Result<*mut LLVMValue> {
    unsafe {
        let mut parts = call.trim_end_matches('\n').split('\t');
        let left_name = parts.next().unwrap_or_default();
        let operator = parts.next().unwrap_or_default();
        let operand = parts.next().unwrap_or_default();
        let type_name = parts.next().unwrap_or_default();
        let ty = into_type(context, type_name, named_types);
        let left = load_slot(builder, slots, left_name, empty)?;
        let (kind, value) = operand.split_once(':').unwrap_or(("", operand));
        let right = if kind == "p" {
            load_slot(builder, slots, value, empty)?
        } else {
            const_for_type(context, ty, value)?
        };
        let is_float = matches!(
            LLVMGetTypeKind(ty),
            LLVMTypeKind::LLVMFloatTypeKind | LLVMTypeKind::LLVMDoubleTypeKind
        );
        let condition = if is_float {
            LLVMBuildFCmp(
                builder,
                real_predicate(operator)?,
                left,
                right,
                empty.as_ptr(),
            )
        } else {
            LLVMBuildICmp(
                builder,
                int_predicate(operator)?,
                left,
                right,
                empty.as_ptr(),
            )
        };
        Ok(condition)
    }
}

unsafe fn slot_ptr(
    builder: *mut LLVMBuilder,
    entry: *mut LLVMBasicBlock,
    empty: &CString,
    slots: &mut Slots,
    name: &str,
    ty: *mut LLVMType,
) -> *mut LLVMValue {
    unsafe {
        if let Some(slot) = slots.get(name) {
            return slot.ptr;
        }
        let current = LLVMGetInsertBlock(builder);
        let first = LLVMGetFirstInstruction(entry);
        if first.is_null() {
            LLVMPositionBuilderAtEnd(builder, entry);
        } else {
            LLVMPositionBuilderBefore(builder, first);
        }
        let ptr = LLVMBuildAlloca(builder, ty, empty.as_ptr());
        if !current.is_null() {
            LLVMPositionBuilderAtEnd(builder, current);
        }
        slots.insert(name.to_owned(), Slot { ptr, ty });
        ptr
    }
}

unsafe fn store_slot(
    builder: *mut LLVMBuilder,
    entry: *mut LLVMBasicBlock,
    empty: &CString,
    slots: &mut Slots,
    name: &str,
    ty: *mut LLVMType,
    value: *mut LLVMValue,
) {
    unsafe {
        let ptr = slot_ptr(builder, entry, empty, slots, name, ty);
        LLVMBuildStore(builder, value, ptr);
    }
}

unsafe fn load_slot(
    builder: *mut LLVMBuilder,
    slots: &Slots,
    name: &str,
    empty: &CString,
) -> Result<*mut LLVMValue> {
    unsafe {
        let slot = slots
            .get(name)
            .ok_or_else(|| anyhow!("Native value `{name}` has no storage"))?;
        Ok(LLVMBuildLoad2(builder, slot.ty, slot.ptr, empty.as_ptr()))
    }
}

unsafe fn const_for_type(
    _context: *mut LLVMContext,
    ty: *mut LLVMType,
    text: &str,
) -> Result<*mut LLVMValue> {
    unsafe {
        match LLVMGetTypeKind(ty) {
            LLVMTypeKind::LLVMFloatTypeKind | LLVMTypeKind::LLVMDoubleTypeKind => {
                let value = text.parse::<f64>()?;
                Ok(LLVMConstReal(ty, value))
            }
            _ => {
                let value = text.parse::<i64>()?;
                Ok(LLVMConstInt(ty, value as u64, 1))
            }
        }
    }
}

unsafe fn print_scalar(
    context: *mut LLVMContext,
    builder: *mut LLVMBuilder,
    printf_type: *mut LLVMType,
    printf: *mut LLVMValue,
    empty: &CString,
    ty: *mut LLVMType,
    value: *mut LLVMValue,
    newline: bool,
) -> Result<()> {
    unsafe {
        let (format, value) = match LLVMGetTypeKind(ty) {
            LLVMTypeKind::LLVMFloatTypeKind => {
                let promoted = LLVMBuildFPExt(
                    builder,
                    value,
                    LLVMDoubleTypeInContext(context),
                    empty.as_ptr(),
                );
                (if newline { "%f\n" } else { "%f" }, promoted)
            }
            LLVMTypeKind::LLVMDoubleTypeKind => (if newline { "%f\n" } else { "%f" }, value),
            LLVMTypeKind::LLVMIntegerTypeKind => {
                let width = LLVMGetIntTypeWidth(ty);
                if width > 32 {
                    (if newline { "%ld\n" } else { "%ld" }, value)
                } else {
                    let promoted = if width < 32 {
                        LLVMBuildSExt(
                            builder,
                            value,
                            LLVMInt32TypeInContext(context),
                            empty.as_ptr(),
                        )
                    } else {
                        value
                    };
                    (if newline { "%d\n" } else { "%d" }, promoted)
                }
            }
            _ => (if newline { "%s\n" } else { "%s" }, value),
        };
        let format = CString::new(format)?;
        let format_value = LLVMBuildGlobalString(builder, format.as_ptr(), empty.as_ptr());
        let mut args = vec![format_value, value];
        LLVMBuildCall2(
            builder,
            printf_type,
            printf,
            args.as_mut_ptr(),
            2,
            empty.as_ptr(),
        );
        Ok(())
    }
}

fn int_predicate(operator: &str) -> Result<llvm::LLVMIntPredicate> {
    Ok(match operator {
        "==" => llvm::LLVMIntPredicate::LLVMIntEQ,
        "!=" => llvm::LLVMIntPredicate::LLVMIntNE,
        ">" => llvm::LLVMIntPredicate::LLVMIntSGT,
        "<" => llvm::LLVMIntPredicate::LLVMIntSLT,
        ">=" => llvm::LLVMIntPredicate::LLVMIntSGE,
        "<=" => llvm::LLVMIntPredicate::LLVMIntSLE,
        _ => anyhow::bail!("Unsupported comparison operator `{operator}`"),
    })
}

fn real_predicate(operator: &str) -> Result<llvm::LLVMRealPredicate> {
    Ok(match operator {
        "==" => llvm::LLVMRealPredicate::LLVMRealOEQ,
        "!=" => llvm::LLVMRealPredicate::LLVMRealONE,
        ">" => llvm::LLVMRealPredicate::LLVMRealOGT,
        "<" => llvm::LLVMRealPredicate::LLVMRealOLT,
        ">=" => llvm::LLVMRealPredicate::LLVMRealOGE,
        "<=" => llvm::LLVMRealPredicate::LLVMRealOLE,
        _ => anyhow::bail!("Unsupported comparison operator `{operator}`"),
    })
}

unsafe fn native_call_arguments<'a>(
    context: *mut LLVMContext,
    builder: *mut LLVMBuilder,
    empty: &CString,
    slots: &Slots,
    args: impl Iterator<Item = &'a str>,
) -> Result<(Vec<*mut LLVMType>, Vec<*mut LLVMValue>)> {
    unsafe {
        let mut params = Vec::new();
        let mut values = Vec::new();
        for arg in args {
            let (kind, value) = arg.split_once(':').unwrap_or(("", ""));
            if kind == "s" {
                let decoded = decode_hex(value)?;
                let value = CString::new(decoded)?;
                params.push(LLVMPointerType(LLVMInt8TypeInContext(context), 0));
                values.push(LLVMBuildGlobalString(
                    builder,
                    value.as_ptr(),
                    empty.as_ptr(),
                ));
            } else if kind == "i" {
                let number = value.parse::<i32>()?;
                params.push(LLVMInt32TypeInContext(context));
                values.push(LLVMConstInt(
                    LLVMInt32TypeInContext(context),
                    number as u64,
                    1,
                ));
            } else if kind == "p" {
                let slot = slots
                    .get(value)
                    .ok_or_else(|| anyhow!("Native pointer argument `{value}` has no value"))?;
                params.push(slot.ty);
                values.push(LLVMBuildLoad2(builder, slot.ty, slot.ptr, empty.as_ptr()));
            } else {
                anyhow::bail!("Unsupported native link argument type `{kind}`");
            }
        }
        Ok((params, values))
    }
}

unsafe fn emit_text(
    context: *mut LLVMContext,
    builder: *mut LLVMBuilder,
    printf_type: *mut LLVMType,
    printf: *mut LLVMValue,
    empty: &CString,
    text: &str,
) -> Result<()> {
    if text.is_empty() {
        return Ok(());
    }
    unsafe {
        let message = CString::new(text)?;
        let mut args = vec![LLVMBuildGlobalString(
            builder,
            message.as_ptr(),
            empty.as_ptr(),
        )];
        LLVMBuildCall2(
            builder,
            printf_type,
            printf,
            args.as_mut_ptr(),
            1,
            empty.as_ptr(),
        );
    }
    let _ = context;
    Ok(())
}

fn decode_hex(value: &str) -> Result<Vec<u8>> {
    if value.len() % 2 != 0 {
        anyhow::bail!("Invalid native string argument encoding");
    }
    (0..value.len())
        .step_by(2)
        .map(|index| Ok(u8::from_str_radix(&value[index..index + 2], 16)?))
        .collect()
}

unsafe fn into_type(
    context: *mut LLVMContext,
    name: &str,
    named_types: &HashMap<String, *mut LLVMType>,
) -> *mut LLVMType {
    unsafe {
        match name {
            "String" => LLVMPointerType(LLVMInt8TypeInContext(context), 0),
            "Path" => LLVMPointerType(LLVMInt8TypeInContext(context), 0),
            "Pointer" => LLVMPointerType(LLVMInt8TypeInContext(context), 0),
            "Boolean" => LLVMInt1TypeInContext(context),
            "Int8" => LLVMInt8TypeInContext(context),
            "Int16" => LLVMInt16TypeInContext(context),
            "Int32" => LLVMInt32TypeInContext(context),
            "Int64" => LLVMInt64TypeInContext(context),
            "Float32" => LLVMFloatTypeInContext(context),
            "Float64" => LLVMDoubleTypeInContext(context),
            other => named_types
                .get(other)
                .map(|ty| LLVMPointerType(*ty, 0))
                .unwrap_or_else(|| LLVMPointerType(LLVMInt8TypeInContext(context), 0)),
        }
    }
}

pub(super) unsafe fn generate_library(
    context: *mut LLVMContext,
    module: *mut LLVMModule,
    builder: *mut LLVMBuilder,
    functions: &[backend::NativeFunction],
) -> Result<()> {
    unsafe {
        let empty = CString::new("")?;
        let mut printf_params = vec![LLVMPointerType(LLVMInt8TypeInContext(context), 0)];
        let printf_type = LLVMFunctionType(
            LLVMInt32TypeInContext(context),
            printf_params.as_mut_ptr(),
            0,
            1,
        );
        let printf_name = CString::new("printf")?;
        let printf = LLVMAddFunction(module, printf_name.as_ptr(), printf_type);
        for exported in functions {
            let function_name = CString::new(exported.name.as_bytes())?;
            let function_type =
                LLVMFunctionType(LLVMVoidTypeInContext(context), ptr::null_mut(), 0, 0);
            let function = LLVMAddFunction(module, function_name.as_ptr(), function_type);
            let block_name = CString::new("entry")?;
            let block = LLVMAppendBasicBlockInContext(context, function, block_name.as_ptr());
            LLVMPositionBuilderAtEnd(builder, block);
            if !exported.output.is_empty() {
                let format_string = CString::new("%s")?;
                let format_value =
                    LLVMBuildGlobalString(builder, format_string.as_ptr(), empty.as_ptr());
                let output = CString::new(exported.output.as_bytes())?;
                let output_value = LLVMBuildGlobalString(builder, output.as_ptr(), empty.as_ptr());
                let mut args = vec![format_value, output_value];
                LLVMBuildCall2(
                    builder,
                    printf_type,
                    printf,
                    args.as_mut_ptr(),
                    2,
                    empty.as_ptr(),
                );
            }
            LLVMBuildRetVoid(builder);
        }

        let mut error = ptr::null_mut();
        if LLVMVerifyModule(
            module,
            LLVMVerifierFailureAction::LLVMPrintMessageAction,
            &mut error,
        ) != 0
        {
            let description = if error.is_null() {
                "unknown verifier error".into()
            } else {
                CStr::from_ptr(error).to_string_lossy().into_owned()
            };
            if !error.is_null() {
                LLVMDisposeMessage(error);
            }
            return Err(anyhow!(
                "Failed to verify native library module: {description}"
            ));
        }
        Ok(())
    }
}
