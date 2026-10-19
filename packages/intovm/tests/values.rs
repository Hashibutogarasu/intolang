mod support;

use intovm::{Value, Vm, VmOptions};
use support::{record, run, run_error};
use types::MessageTag::*;

#[test]
fn classes_construct_with_their_defaults() {
    let message = [
        record(Class, &["Point", "x:Int32:n:7", "y:Int32:-:"]),
        record(Construct, &["p", "Point"]),
        record(FieldPrint, &["p", "Point", "0", "Int32", "1"]),
        record(FieldPrint, &["p", "Point", "1", "Int32", "1"]),
    ]
    .concat();
    assert_eq!(run(&message), "7\n0\n");
}

#[test]
fn copy_duplicates_the_instance_and_alias_shares_it() {
    let message = [
        record(Class, &["Box", "value:Int32:n:5"]),
        record(Construct, &["a", "Box"]),
        record(Copy, &["b", "a", "Box"]),
        record(Alias, &["c", "a"]),
        record(FieldPrint, &["b", "Box", "0", "Int32", "1"]),
    ]
    .concat();
    assert_eq!(run(&message), "5\n");
}

#[test]
fn print_const_prints_a_number_operand() {
    let message = [record(PrintConst, &["String", "n:7", "1"])].concat();
    assert_eq!(run(&message), "7\n");
}

#[test]
fn reading_an_unset_slot_is_an_error() {
    let message = record(Print, &["missing", "Int32", "1"]);
    assert!(run_error(&message).contains("`missing` has no storage"));
}

#[test]
fn unknown_native_functions_are_errors() {
    let message = record(
        LinkAssign,
        &["ps", "c", "src/x.c", "unknown_symbol", "Void", ""],
    );
    assert!(run_error(&message).contains("`unknown_symbol` is not available in the VM"));
}

#[test]
fn intrinsics_replace_native_functions() {
    let message = [
        record(
            LinkAssign,
            &["ps", "c", "src/x.c", "add_one", "Int32", "r", "i:41"],
        ),
        record(Print, &["r", "Int32", "1"]),
    ]
    .concat();
    let mut vm = Vm::new(VmOptions::default());
    vm.register(
        "add_one",
        std::rc::Rc::new(|_, args| Ok(Value::Int(args[0].int().unwrap() + 1))),
    );
    vm.load(&message).unwrap();
    vm.run().unwrap();
    assert_eq!(vm.stdout(), "42\n");
}
