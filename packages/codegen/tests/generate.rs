use anyhow::Result;
use llvm_sys::core::*;
use moduler::NativeFunction;
use parser::Record;
use std::ffi::{CStr, CString};
use types::MessageTag;

/// Run `build` against a fresh module and return the module's IR.
fn module_ir(
    build: impl Fn(
        *mut llvm_sys::LLVMContext,
        *mut llvm_sys::LLVMModule,
        *mut llvm_sys::LLVMBuilder,
    ) -> Result<()>,
) -> Result<String> {
    unsafe {
        let context = LLVMContextCreate();
        let name = CString::new("codegen-test")?;
        let module = LLVMModuleCreateWithNameInContext(name.as_ptr(), context);
        let builder = LLVMCreateBuilderInContext(context);
        let result = build(context, module, builder);
        let text = LLVMPrintModuleToString(module);
        let ir = CStr::from_ptr(text).to_string_lossy().into_owned();
        LLVMDisposeMessage(text);
        LLVMDisposeBuilder(builder);
        LLVMDisposeModule(module);
        LLVMContextDispose(context);
        result.map(|()| ir)
    }
}

fn generate(message: &str) -> Result<String> {
    module_ir(|context, module, builder| unsafe {
        codegen::generate(context, module, builder, message)
    })
}

fn record(tag: MessageTag, fields: &[&str]) -> String {
    fields
        .iter()
        .fold(Record::new(tag), |record, field| record.field(field))
        .to_string()
}

#[test]
fn generates_main_with_stored_values() {
    let message = [
        record(MessageTag::Assign, &["count", "Int32", "n:5"]),
        record(MessageTag::Print, &["count", "Int32", "1"]),
    ]
    .concat();
    let ir = generate(&message).unwrap();
    assert!(ir.contains("define i64 @main()"), "{ir}");
    assert!(ir.contains("store i32 5"), "{ir}");
    assert!(ir.contains("@printf"), "{ir}");
}

#[test]
fn prints_literal_text_lines() {
    let ir = generate("hello\n").unwrap();
    assert!(ir.contains("hello"), "{ir}");
}

#[test]
fn generates_loops_and_branches() {
    let message = [
        record(MessageTag::Assign, &["i", "Int32", "n:0"]),
        record(MessageTag::WhileBegin, &["i", "<", "n:3", "Int32"]),
        record(MessageTag::IfBegin, &["i", "==", "n:1", "Int32"]),
        record(MessageTag::Break, &[]),
        record(MessageTag::IfElse, &[]),
        record(MessageTag::Arith, &["i", "i", "add", "n:1", "Int32"]),
        record(MessageTag::IfEnd, &[]),
        record(MessageTag::LoopEnd, &[]),
    ]
    .concat();
    let ir = generate(&message).unwrap();
    assert!(ir.contains("while.condition"), "{ir}");
    assert!(ir.contains("if.then"), "{ir}");
    assert!(ir.contains("add i32"), "{ir}");
}

#[test]
fn declares_classes_and_constructs_them() {
    let message = [
        record(MessageTag::Class, &["Config", "size:Int32:n:7"]),
        record(MessageTag::Construct, &["config", "Config"]),
        record(
            MessageTag::FieldPrint,
            &["config", "Config", "0", "Int32", "1"],
        ),
    ]
    .concat();
    let ir = generate(&message).unwrap();
    assert!(ir.contains("%Config = type { i32 }"), "{ir}");
}

#[test]
fn aliases_share_the_source_storage() {
    let message = [
        record(MessageTag::Class, &["Config", "size:Int32:n:7"]),
        record(MessageTag::Construct, &["config", "Config"]),
        record(MessageTag::Alias, &["moved", "config"]),
        record(
            MessageTag::FieldPrint,
            &["moved", "Config", "0", "Int32", "1"],
        ),
    ]
    .concat();
    let ir = generate(&message).unwrap();
    assert!(ir.contains("%Config = type { i32 }"), "{ir}");
    assert_eq!(ir.matches("alloca %Config").count(), 1, "{ir}");
}

#[test]
fn copies_instances_into_new_storage() {
    let message = [
        record(MessageTag::Class, &["Config", "size:Int32:n:7"]),
        record(MessageTag::Construct, &["config", "Config"]),
        record(MessageTag::Copy, &["copied", "config", "Config"]),
        record(
            MessageTag::FieldPrint,
            &["copied", "Config", "0", "Int32", "1"],
        ),
    ]
    .concat();
    let ir = generate(&message).unwrap();
    assert_eq!(ir.matches("alloca %Config").count(), 2, "{ir}");
}

fn handler(name: &str, captures: &str, params: &[&str], body: &[String]) -> String {
    let mut fields = vec![name, captures];
    fields.extend_from_slice(params);
    [
        vec![record(MessageTag::HandlerBegin, &fields)],
        body.to_vec(),
        vec![record(MessageTag::HandlerEnd, &[])],
    ]
    .concat()
    .concat()
}

#[test]
fn generates_handler_functions_and_event_tables() {
    let message = [
        handler(
            "handler_one",
            "",
            &["value:Int32"],
            &[record(MessageTag::Print, &["value", "Int32", "1"])],
        ),
        record(MessageTag::EventOn, &["example", "", "handler_one"]),
        record(MessageTag::Emit, &["example", "i:3"]),
    ]
    .concat();
    let ir = generate(&message).unwrap();
    assert!(ir.contains("define void @handler_one(i32"), "{ir}");
    assert!(ir.contains("@__event_example_handlers"), "{ir}");
    assert!(ir.contains("call void %"), "{ir}");
}

#[test]
fn registrations_can_be_removed_one_by_one_or_all_at_once() {
    let message = [
        handler("handler_one", "", &[], &[]),
        record(MessageTag::EventOn, &["example", "listener", "handler_one"]),
        record(MessageTag::EventOff, &["example", "listener"]),
        record(MessageTag::EventDestroy, &["example"]),
    ]
    .concat();
    let ir = generate(&message).unwrap();
    assert!(ir.contains("@__event_example_ids"), "{ir}");
    assert!(ir.contains("store ptr null"), "{ir}");
}

#[test]
fn handlers_reach_captured_values_through_globals() {
    let message = [
        record(MessageTag::Assign, &["total", "Int32", "n:0"]),
        handler(
            "handler_one",
            "total",
            &[],
            &[record(
                MessageTag::Arith,
                &["total", "total", "add", "n:1", "Int32"],
            )],
        ),
        record(MessageTag::EventOn, &["example", "", "handler_one"]),
        record(MessageTag::Emit, &["example"]),
        record(MessageTag::Print, &["total", "Int32", "1"]),
    ]
    .concat();
    let ir = generate(&message).unwrap();
    assert!(ir.contains("@__global_total"), "{ir}");
}

#[test]
fn rejects_handler_ends_without_a_start_and_unknown_handlers() {
    let error = generate(&record(MessageTag::HandlerEnd, &[])).unwrap_err();
    assert!(error.to_string().contains("no matching handler"), "{error}");
    let error = generate(&record(MessageTag::EventOn, &["example", "", "missing"])).unwrap_err();
    assert!(
        error.to_string().contains("Unknown event handler"),
        "{error}"
    );
}

#[test]
fn rejects_copying_unknown_classes() {
    let error = generate(&record(MessageTag::Copy, &["a", "b", "Missing"])).unwrap_err();
    assert!(
        error.to_string().contains("Unknown native class"),
        "{error}"
    );
}

#[test]
fn calls_native_functions_with_encoded_arguments() {
    let message = record(
        MessageTag::LinkAssign,
        &[
            "pkg",
            "c",
            "src/ffi/a.c",
            "greet",
            "Void",
            "",
            "s:6162",
            "i:2",
        ],
    );
    let ir = generate(&message).unwrap();
    assert!(ir.contains("call void @greet(ptr"), "{ir}");
}

#[test]
fn rejects_unbalanced_blocks() {
    let error = generate(&record(MessageTag::IfEnd, &[])).unwrap_err();
    assert!(error.to_string().contains("no matching if"), "{error}");
    let error = generate(&record(MessageTag::Break, &[])).unwrap_err();
    assert!(error.to_string().contains("no matching loop"), "{error}");
}

#[test]
fn rejects_malformed_records() {
    let error = generate(&record(MessageTag::LinkAssign, &["pkg"])).unwrap_err();
    assert!(error.to_string().contains("Malformed"), "{error}");
}

#[test]
fn generates_exported_library_functions() {
    let functions = [NativeFunction {
        name: "say_hello".into(),
        output: "hello\n".into(),
    }];
    let ir = module_ir(|context, module, builder| unsafe {
        codegen::generate_library(context, module, builder, &functions)
    })
    .unwrap();
    assert!(ir.contains("define void @say_hello()"), "{ir}");
}
