#![allow(dead_code)]

use intovm::{Vm, VmOptions};
use parser::Record;
use types::MessageTag;

pub fn record(tag: MessageTag, fields: &[&str]) -> String {
    fields
        .iter()
        .fold(Record::new(tag), |record, field| record.field(field))
        .to_string()
}

/// The `s:` operand of a string.
pub fn text(value: &str) -> String {
    let hex: String = value.bytes().map(|byte| format!("{byte:02x}")).collect();
    format!("s:{hex}")
}

/// A `NATIVE_LINK_ASSIGN` of a `ps` function.
pub fn ps_call(symbol: &str, return_type: &str, assigned_to: &str, args: &[&str]) -> String {
    let mut fields = vec![
        "ps",
        "c",
        "src/ffi/ps/process.c",
        symbol,
        return_type,
        assigned_to,
    ];
    fields.extend_from_slice(args);
    record(MessageTag::LinkAssign, &fields)
}

pub fn vm_of(message: &str, options: VmOptions) -> Vm {
    let mut vm = Vm::new(options);
    vm.load(message).unwrap();
    vm
}

pub fn run(message: &str) -> String {
    let mut vm = Vm::new(VmOptions::default());
    vm.load(message).unwrap();
    vm.run().unwrap();
    vm.stdout()
}

pub fn run_error(message: &str) -> String {
    let mut vm = Vm::new(VmOptions::default());
    vm.load(message).unwrap();
    format!("{:#}", vm.run().unwrap_err())
}
