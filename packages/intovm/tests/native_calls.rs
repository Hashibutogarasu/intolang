mod support;

use bundle::Bundle;
use intovm::{Inex, Vm, VmOptions};
use support::record;
use types::MessageTag::*;

const SOURCE: &str = r#"
#include <stdint.h>
#include <stdio.h>

typedef struct {
    int32_t id;
    const char *label;
} item;

typedef struct {
    int32_t value;
} counter;

int32_t add_numbers(int32_t a, int32_t b) { return a + b; }

const char *greet(const char *name) {
    static char buffer[64];
    snprintf(buffer, sizeof buffer, "Hello, %s!", name);
    return buffer;
}

item *make_item(void) {
    static item made = {7, "seven"};
    return &made;
}

void bump(counter *target) { target->value += 5; }

int32_t is_even(int32_t number) { return number % 2 == 0; }
"#;

fn program(message: String, source: &str) -> Inex {
    let mut bundle = Bundle::new();
    bundle
        .add_file(
            ".into/packages/t/into-package.json",
            r#"{"name":"T","alias":"t","version":"0.1.0","LICENCE":"MIT"}"#,
            0o644,
        )
        .unwrap();
    bundle
        .add_file(".into/packages/t/src/ffi/math.c", source, 0o644)
        .unwrap();
    bundle.add_file("intoc", message, 0o644).unwrap();
    Inex {
        generator: "test".to_owned(),
        bundle,
    }
}

fn call(symbol: &str, return_type: &str, assigned_to: &str, args: &[&str]) -> String {
    let mut fields = vec!["t", "c", "src/ffi/math.c", symbol, return_type, assigned_to];
    fields.extend_from_slice(args);
    record(LinkAssign, &fields)
}

fn run(message: String) -> anyhow::Result<String> {
    let mut vm = Vm::new(VmOptions::default());
    vm.load_inex(&program(message, SOURCE))?;
    vm.run()?;
    Ok(vm.stdout())
}

#[test]
fn integers_go_in_and_out() {
    let message = [
        call("add_numbers", "Int32", "sum", &["i:2", "i:40"]),
        record(Print, &["sum", "Int32", "1"]),
    ]
    .concat();
    assert_eq!(run(message).unwrap(), "42\n");
}

#[test]
fn text_goes_in_and_out() {
    let message = [
        call("greet", "String", "greeting", &["s:496e746f"]),
        record(Print, &["greeting", "String", "1"]),
    ]
    .concat();
    assert_eq!(run(message).unwrap(), "Hello, Into!\n");
}

#[test]
fn a_boolean_result_is_read_from_the_low_byte() {
    let message = [
        call("is_even", "Boolean", "even", &["i:10"]),
        call("is_even", "Boolean", "odd", &["i:7"]),
        record(Print, &["even", "Int32", "1"]),
        record(Print, &["odd", "Int32", "1"]),
    ]
    .concat();
    assert_eq!(run(message).unwrap(), "1\n0\n");
}

#[test]
fn an_instance_native_code_allocated_is_read_in_place() {
    let message = [
        record(Class, &["item", "id:Int32:-:", "label:String:-:"]),
        call("make_item", "item", "made", &[]),
        record(FieldPrint, &["made", "item", "0", "Int32", "1"]),
        record(FieldPrint, &["made", "item", "1", "String", "1"]),
    ]
    .concat();
    assert_eq!(run(message).unwrap(), "7\nseven\n");
}

#[test]
fn a_field_of_a_native_instance_is_a_call_argument() {
    let message = [
        record(Class, &["item", "id:Int32:-:", "label:String:-:"]),
        call("make_item", "item", "made", &[]),
        call("add_numbers", "Int32", "sum", &["f:made:item:0", "i:3"]),
        record(Print, &["sum", "Int32", "1"]),
    ]
    .concat();
    assert_eq!(run(message).unwrap(), "10\n");
}

#[test]
fn native_code_may_change_an_instance_the_vm_made() {
    let message = [
        record(Class, &["counter", "value:Int32:n:7"]),
        record(Construct, &["c", "counter"]),
        call("bump", "Void", "", &["p:c"]),
        record(FieldPrint, &["c", "counter", "0", "Int32", "1"]),
    ]
    .concat();
    assert_eq!(run(message).unwrap(), "12\n");
}

#[test]
fn a_function_the_library_lacks_is_reported() {
    let message = call("missing_function", "Void", "", &[]);
    let error = run(message).unwrap_err();
    assert!(
        format!("{error:#}").contains("`missing_function` is not available in the VM"),
        "{error:#}"
    );
}

#[test]
fn a_source_that_does_not_compile_is_reported() {
    let mut vm = Vm::new(VmOptions::default());
    let error = vm
        .load_inex(&program(
            call("add_numbers", "Int32", "sum", &["i:2", "i:40"]),
            "int broken( {",
        ))
        .unwrap_err();
    assert!(
        format!("{error:#}").contains("compilation failed"),
        "{error:#}"
    );
}

#[test]
fn intrinsics_win_over_the_native_library() {
    let message = [
        call("add_numbers", "Int32", "sum", &["i:2", "i:40"]),
        record(Print, &["sum", "Int32", "1"]),
    ]
    .concat();
    let mut vm = Vm::new(VmOptions::default());
    vm.load_inex(&program(message, SOURCE)).unwrap();
    vm.register(
        "add_numbers",
        std::rc::Rc::new(|_, _| Ok(intovm::Value::Int(-1))),
    );
    vm.run().unwrap();
    assert_eq!(vm.stdout(), "-1\n");
}
