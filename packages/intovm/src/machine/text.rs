use errors::{Errors, Result};
use std::rc::Rc;

use super::Vm;
use crate::value::Value;

fn bytes(args: &[Value], index: usize) -> Result<&[u8]> {
    match args.get(index) {
        Some(Value::Str(bytes)) => Ok(bytes),
        Some(other) => Err(errors::Error::from(Errors::NotTextValue {
            value: format!("{:?}", other),
        })),
        None => Err(errors::Error::from(Errors::MissingNativeArgument {
            index: index.to_string(),
        })),
    }
}

fn int(args: &[Value], index: usize) -> Result<i64> {
    args.get(index).and_then(Value::int).ok_or_else(|| {
        errors::Error::from(Errors::MissingIntegerArgument {
            index: index.to_string(),
        })
    })
}

fn flag(value: bool) -> Result<Value> {
    Ok(Value::Int(i64::from(value)))
}

fn encode(code: i64) -> Vec<u8> {
    u32::try_from(code)
        .ok()
        .and_then(char::from_u32)
        .unwrap_or(char::REPLACEMENT_CHARACTER)
        .to_string()
        .into_bytes()
}

fn find(text: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    text.windows(needle.len())
        .position(|window| window == needle)
}

/// The text functions of `string.c`, run on the bytes of the text so that
/// the VM and the compiled code agree.
pub(crate) fn install(vm: &mut Vm) {
    vm.register(
        "into_string_concat",
        Rc::new(|_, args| {
            let mut joined = bytes(args, 0)?.to_vec();
            joined.extend_from_slice(bytes(args, 1)?);
            Ok(Value::Str(joined))
        }),
    );
    vm.register(
        "into_string_from_char",
        Rc::new(|_, args| Ok(Value::Str(encode(int(args, 0)?)))),
    );
    vm.register(
        "into_string_equals",
        Rc::new(|_, args| flag(bytes(args, 0)? == bytes(args, 1)?)),
    );
    vm.register(
        "into_string_length",
        Rc::new(|_, args| Ok(Value::Int(bytes(args, 0)?.len() as i64))),
    );
    vm.register(
        "into_string_at",
        Rc::new(|_, args| {
            let text = bytes(args, 0)?;
            let index = int(args, 1)?;
            let code = usize::try_from(index)
                .ok()
                .and_then(|index| text.get(index))
                .copied()
                .unwrap_or(0);
            Ok(Value::Int(i64::from(code)))
        }),
    );
    vm.register(
        "into_string_substring",
        Rc::new(|_, args| {
            let text = bytes(args, 0)?;
            let start = int(args, 1)?.max(0) as usize;
            let end = (int(args, 2)?.max(0) as usize).min(text.len());
            let part = if start >= end {
                &[][..]
            } else {
                &text[start..end]
            };
            Ok(Value::Str(part.to_vec()))
        }),
    );
    vm.register(
        "into_string_index_of",
        Rc::new(|_, args| {
            let found = find(bytes(args, 0)?, bytes(args, 1)?);
            Ok(Value::Int(found.map_or(-1, |index| index as i64)))
        }),
    );
    vm.register(
        "into_string_starts_with",
        Rc::new(|_, args| flag(bytes(args, 0)?.starts_with(bytes(args, 1)?))),
    );
    vm.register(
        "into_string_ends_with",
        Rc::new(|_, args| flag(bytes(args, 0)?.ends_with(bytes(args, 1)?))),
    );
    vm.register(
        "into_console_write",
        Rc::new(|vm, args| {
            let text = bytes(args, 0)?.to_vec();
            vm.write(&text);
            Ok(Value::Unit)
        }),
    );
    vm.register(
        "into_console_read",
        Rc::new(|vm, _| Ok(Value::Str(vm.read_line()))),
    );
    vm.register(
        "into_string_to_lower",
        Rc::new(|_, args| Ok(Value::Str(bytes(args, 0)?.to_ascii_lowercase()))),
    );
    vm.register(
        "into_string_to_upper",
        Rc::new(|_, args| Ok(Value::Str(bytes(args, 0)?.to_ascii_uppercase()))),
    );
}
