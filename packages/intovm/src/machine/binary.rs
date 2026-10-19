use anyhow::{Result, anyhow};
use std::ffi::c_void;
use std::rc::Rc;

use super::Vm;
use super::memory_access::address_of;
use crate::value::Value;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn calloc(count: usize, size: usize) -> *mut c_void;
    fn free(pointer: *mut c_void);
}

/// The layout `binary.c` gives a `Binary`, so that native code can take the
/// instances the VM makes.
#[repr(C)]
struct Binary {
    data: *mut u8,
    length: i64,
}

fn path(args: &[Value], index: usize) -> Result<std::path::PathBuf> {
    match args.get(index) {
        Some(Value::Str(bytes)) => Ok(String::from_utf8_lossy(bytes).into_owned().into()),
        other => Err(anyhow!("{other:?} is not a path")),
    }
}

fn binary(args: &[Value]) -> Result<*mut Binary> {
    Ok(args.first().map_or(Ok(0), address_of)? as *mut Binary)
}

fn contents<'a>(binary: *mut Binary) -> &'a [u8] {
    if binary.is_null() {
        return &[];
    }
    unsafe {
        let binary = &*binary;
        if binary.data.is_null() || binary.length <= 0 {
            &[]
        } else {
            std::slice::from_raw_parts(binary.data, binary.length as usize)
        }
    }
}

/// A `Binary` that holds a copy of `bytes`, or a null pointer when there is no
/// memory for it.
pub(super) fn make(bytes: &[u8]) -> *mut c_void {
    make_binary(bytes) as *mut c_void
}

fn make_binary(bytes: &[u8]) -> *mut Binary {
    unsafe {
        let binary = calloc(1, std::mem::size_of::<Binary>()) as *mut Binary;
        if binary.is_null() {
            return binary;
        }
        if !bytes.is_empty() {
            let data = malloc(bytes.len()) as *mut u8;
            if data.is_null() {
                free(binary as *mut c_void);
                return std::ptr::null_mut();
            }
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), data, bytes.len());
            (*binary).data = data;
            (*binary).length = bytes.len() as i64;
        }
        binary
    }
}

/// The `Binary` functions of `binary.c`, on structures the VM allocates in
/// the same layout.
pub(crate) fn install(vm: &mut Vm) {
    vm.register(
        "into_binary_read",
        Rc::new(|_, args| {
            let pointer = std::fs::read(path(args, 0)?)
                .map_or(std::ptr::null_mut(), |bytes| make_binary(&bytes));
            Ok(Value::Int(pointer as i64))
        }),
    );
    vm.register(
        "into_binary_write",
        Rc::new(|_, args| {
            let binary = binary(args)?;
            let written =
                !binary.is_null() && std::fs::write(path(args, 1)?, contents(binary)).is_ok();
            Ok(Value::Int(i64::from(written)))
        }),
    );
    vm.register(
        "into_binary_length",
        Rc::new(|_, args| Ok(Value::Int(contents(binary(args)?).len() as i64))),
    );
    vm.register(
        "into_binary_at",
        Rc::new(|_, args| {
            let index = args.get(1).and_then(Value::int).unwrap_or(-1);
            let byte = usize::try_from(index)
                .ok()
                .and_then(|index| contents(binary(args).ok()?).get(index).copied())
                .unwrap_or(0);
            Ok(Value::Int(i64::from(byte)))
        }),
    );
    vm.register(
        "into_binary_text",
        Rc::new(|_, args| Ok(Value::Str(contents(binary(args)?).to_vec()))),
    );
    vm.register(
        "into_binary_destroy",
        Rc::new(|_, args| {
            let binary = binary(args)?;
            if !binary.is_null() {
                unsafe {
                    free((*binary).data as *mut c_void);
                    free(binary as *mut c_void);
                }
            }
            Ok(Value::Unit)
        }),
    );
}
