use errors::{Errors, Result};
use std::rc::Rc;

use super::Vm;
use crate::value::Value;

fn handle(args: &[Value]) -> Result<usize> {
    match args.first() {
        Some(Value::Int(handle)) if *handle > 0 => Ok(*handle as usize - 1),
        other => Err(errors::Error::from(Errors::NotAList {
            value: format!("{:?}", other),
        })),
    }
}

fn index(args: &[Value]) -> Result<Option<usize>> {
    let number = args
        .get(1)
        .and_then(Value::int)
        .ok_or_else(|| errors::Error::from(Errors::ListMissingIndex))?;
    Ok(usize::try_from(number).ok())
}

fn item(args: &[Value], position: usize) -> Result<Value> {
    args.get(position)
        .cloned()
        .ok_or_else(|| errors::Error::from(Errors::ListMissingItem))
}

fn items<'a>(vm: &'a mut Vm, args: &[Value]) -> Result<&'a mut Vec<Value>> {
    let position = handle(args)?;
    vm.lists
        .get_mut(position)
        .ok_or_else(|| errors::Error::from(Errors::ListFreed))
}

/// The list functions of `list.c`, on lists that hold the values the program
/// works with, so that a list can hold instances the VM owns as well.
pub(crate) fn install(vm: &mut Vm) {
    vm.register(
        "into_list_new",
        Rc::new(|vm, _| {
            vm.lists.push(Vec::new());
            Ok(Value::Int(vm.lists.len() as i64))
        }),
    );
    for kind in ["int", "float", "pointer"] {
        let zero = if kind == "float" {
            Value::Float(0.0)
        } else {
            Value::Int(0)
        };
        vm.register(
            &format!("into_list_push_{kind}"),
            Rc::new(|vm, args| {
                let value = item(args, 1)?;
                items(vm, args)?.push(value);
                Ok(Value::Unit)
            }),
        );
        vm.register(
            &format!("into_list_get_{kind}"),
            Rc::new(move |vm, args| {
                let position = index(args)?;
                let items = items(vm, args)?;
                Ok(position
                    .and_then(|position| items.get(position))
                    .cloned()
                    .unwrap_or_else(|| zero.clone()))
            }),
        );
        vm.register(
            &format!("into_list_set_{kind}"),
            Rc::new(|vm, args| {
                let position = index(args)?;
                let value = item(args, 2)?;
                let items = items(vm, args)?;
                if let Some(slot) = position.and_then(|position| items.get_mut(position)) {
                    *slot = value;
                }
                Ok(Value::Unit)
            }),
        );
    }
    vm.register(
        "into_list_length",
        Rc::new(|vm, args| Ok(Value::Int(items(vm, args)?.len() as i64))),
    );
    vm.register(
        "into_list_free",
        Rc::new(|vm, args| {
            items(vm, args)?.clear();
            Ok(Value::Unit)
        }),
    );
}
