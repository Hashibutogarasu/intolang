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
