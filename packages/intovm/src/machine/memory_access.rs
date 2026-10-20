use errors::{Errors, Result};
use native::{MemoryLoad, MemoryStore};
use std::alloc::Layout;
use std::rc::Rc;

use super::vm::Vm;
use crate::value::Value;

/// The address a value holds: a pointer a native call returned, or an
/// integer.
pub(super) fn address_of(value: &Value) -> Result<usize> {
    match value {
        Value::Int(address) => Ok(*address as usize),
        Value::Native { ptr, .. } => Ok(*ptr),
        other => Err(errors::Error::from(Errors::NotAPointer {
            value: format!("{:?}", other),
        })),
    }
}

/// How many bytes an item of the type takes in memory.
fn size_of(ty: &str) -> Result<usize> {
    Ok(match ty {
        "Int8" | "Boolean" => 1,
        "Int16" => 2,
        "Int32" | "Char" | "Float32" => 4,
        "Int64" | "Float64" | "Pointer" => 8,
        other => {
            return Err(errors::Error::from(Errors::ItemNotReadable {
                ty: other.to_string(),
            }));
        }
    })
}

/// The memory functions of `memory.c`, on blocks the VM keeps track of, so
/// that a program can use memory without a native library.
pub(crate) fn install(vm: &mut Vm) {
    vm.register(
        "into_memory_allocate",
        Rc::new(|vm, args| {
            let bytes = args.first().and_then(Value::int).unwrap_or(0);
            if bytes <= 0 {
                return Ok(Value::Int(0));
            }
            let layout = Layout::from_size_align(bytes as usize, 16)?;
            let pointer = unsafe { std::alloc::alloc_zeroed(layout) } as usize;
            if pointer == 0 {
                return Ok(Value::Int(0));
            }
            vm.allocations.insert(pointer, layout);
            Ok(Value::Int(pointer as i64))
        }),
    );
    vm.register(
        "into_memory_free",
        Rc::new(|vm, args| {
            let pointer = args.first().map_or(Ok(0), address_of)?;
            if let Some(layout) = vm.allocations.remove(&pointer) {
                unsafe { std::alloc::dealloc(pointer as *mut u8, layout) };
            }
            Ok(Value::Unit)
        }),
    );
    vm.register(
        "into_memory_copy",
        Rc::new(|_, args| {
            let (target, source) = (address_of(&args[0])?, address_of(&args[1])?);
            let bytes = args.get(2).and_then(Value::int).unwrap_or(0);
            if target != 0 && source != 0 && bytes > 0 {
                unsafe { std::ptr::copy(source as *const u8, target as *mut u8, bytes as usize) };
            }
            Ok(Value::Unit)
        }),
    );
    vm.register(
        "into_memory_fill",
        Rc::new(|_, args| {
            let target = address_of(&args[0])?;
            let value = args.get(1).and_then(Value::int).unwrap_or(0) as u8;
            let bytes = args.get(2).and_then(Value::int).unwrap_or(0);
            if target != 0 && bytes > 0 {
                unsafe { std::ptr::write_bytes(target as *mut u8, value, bytes as usize) };
            }
            Ok(Value::Unit)
        }),
    );
}

impl Vm {
    /// Release the blocks the program allocated and did not free.
    pub(super) fn release_allocations(&mut self) {
        for (pointer, layout) in self.allocations.drain() {
            unsafe { std::alloc::dealloc(pointer as *mut u8, layout) };
        }
    }

    /// Read the item of memory a `NATIVE_LOAD` names.
    pub(super) fn memory_load(&mut self, load: &MemoryLoad) -> Result<()> {
        let base = address_of(&self.load_slot(&load.pointer)?)?;
        if base == 0 {
            return Err(errors::Error::from(Errors::NullPointer {
                pointer: load.pointer.to_string(),
            }));
        }
        let index = self
            .operand(&load.index, "Int64")?
            .int()
            .ok_or_else(|| errors::Error::from(Errors::IndexNotNumber))?;
        let address = base.wrapping_add((index as isize * size_of(&load.ty)? as isize) as usize);
        let value = unsafe {
            match load.ty.as_str() {
                "Int8" => Value::Int(i64::from((address as *const i8).read_unaligned())),
                "Boolean" => Value::Int(i64::from((address as *const u8).read_unaligned() != 0)),
                "Int16" => Value::Int(i64::from((address as *const i16).read_unaligned())),
                "Int32" | "Char" => Value::Int(i64::from((address as *const i32).read_unaligned())),
                "Int64" => Value::Int((address as *const i64).read_unaligned()),
                "Float32" => Value::Float(f64::from((address as *const f32).read_unaligned())),
                "Float64" => Value::Float((address as *const f64).read_unaligned()),
                _ => Value::Int((address as *const i64).read_unaligned()),
            }
        };
        self.store(&load.target, value);
        Ok(())
    }

    /// Write the item of memory a `NATIVE_STORE` names.
    pub(super) fn memory_store(&mut self, store: &MemoryStore) -> Result<()> {
        let base = address_of(&self.load_slot(&store.pointer)?)?;
        if base == 0 {
            return Err(errors::Error::from(Errors::NullPointer {
                pointer: store.pointer.to_string(),
            }));
        }
        let index = self
            .operand(&store.index, "Int64")?
            .int()
            .ok_or_else(|| errors::Error::from(Errors::IndexNotNumber))?;
        let value = self.operand(&store.value, &store.ty)?;
        let address = base.wrapping_add((index as isize * size_of(&store.ty)? as isize) as usize);
        unsafe {
            match (store.ty.as_str(), &value) {
                ("Int8" | "Boolean", Value::Int(number)) => {
                    (address as *mut i8).write_unaligned(*number as i8)
                }
                ("Int16", Value::Int(number)) => {
                    (address as *mut i16).write_unaligned(*number as i16)
                }
                ("Int32" | "Char", Value::Int(number)) => {
                    (address as *mut i32).write_unaligned(*number as i32)
                }
                ("Int64", Value::Int(number)) => (address as *mut i64).write_unaligned(*number),
                ("Float32", other) => {
                    (address as *mut f32).write_unaligned(other.float().ok_or_else(|| {
                        errors::Error::from(Errors::NotANumber {
                            value: format!("{:?}", other),
                        })
                    })? as f32)
                }
                ("Float64", other) => {
                    (address as *mut f64).write_unaligned(other.float().ok_or_else(|| {
                        errors::Error::from(Errors::NotANumber {
                            value: format!("{:?}", other),
                        })
                    })?)
                }
                ("Pointer", other) => (address as *mut usize).write_unaligned(address_of(other)?),
                (ty, other) => {
                    return Err(errors::Error::from(Errors::ItemNotWritable {
                        value: format!("{:?}", other),
                        ty: ty.to_string(),
                    }));
                }
            }
        }
        Ok(())
    }
}
