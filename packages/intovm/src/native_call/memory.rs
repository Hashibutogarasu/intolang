use std::ffi::{CStr, CString, c_char};

use super::layout::{FieldKind, FieldLayout};
use crate::value::Value;

/// Read the field at `layout` of the struct that starts at `base`.
///
/// # Safety
///
/// `base` must point to a live struct that has the layout the field was
/// computed from.
pub unsafe fn read_field(base: usize, layout: &FieldLayout) -> Value {
    let address = (base + layout.offset) as *const u8;
    unsafe {
        match &layout.kind {
            FieldKind::Int(1) => Value::Int(i64::from((address as *const i8).read_unaligned())),
            FieldKind::Int(2) => Value::Int(i64::from((address as *const i16).read_unaligned())),
            FieldKind::Int(4) => Value::Int(i64::from((address as *const i32).read_unaligned())),
            FieldKind::Int(_) => Value::Int((address as *const i64).read_unaligned()),
            FieldKind::Float32 => Value::Float(f64::from((address as *const f32).read_unaligned())),
            FieldKind::Float64 => Value::Float((address as *const f64).read_unaligned()),
            FieldKind::Text => {
                let text = (address as *const *const c_char).read_unaligned();
                Value::Str(if text.is_null() {
                    Vec::new()
                } else {
                    CStr::from_ptr(text).to_bytes().to_vec()
                })
            }
            FieldKind::Instance(class) => {
                let pointer = (address as *const usize).read_unaligned();
                if pointer == 0 {
                    Value::Int(0)
                } else {
                    Value::Native {
                        ptr: pointer,
                        class: class.clone(),
                    }
                }
            }
            FieldKind::Pointer => Value::Int((address as *const i64).read_unaligned()),
        }
    }
}

/// Write `value` into the field at `layout` of the struct that starts at
/// `base`. Text is kept alive in `strings` for as long as the struct is used.
///
/// # Safety
///
/// `base` must point to writable memory of the size of the struct.
pub unsafe fn write_field(
    base: usize,
    layout: &FieldLayout,
    value: &Value,
    strings: &mut Vec<CString>,
    native: &dyn Fn(&Value) -> usize,
) {
    let address = (base + layout.offset) as *mut u8;
    unsafe {
        match (&layout.kind, value) {
            (FieldKind::Int(1), Value::Int(number)) => {
                (address as *mut i8).write_unaligned(*number as i8)
            }
            (FieldKind::Int(2), Value::Int(number)) => {
                (address as *mut i16).write_unaligned(*number as i16)
            }
            (FieldKind::Int(4), Value::Int(number)) => {
                (address as *mut i32).write_unaligned(*number as i32)
            }
            (FieldKind::Int(_), Value::Int(number)) => {
                (address as *mut i64).write_unaligned(*number)
            }
            (FieldKind::Float32, Value::Float(number)) => {
                (address as *mut f32).write_unaligned(*number as f32)
            }
            (FieldKind::Float64, Value::Float(number)) => {
                (address as *mut f64).write_unaligned(*number)
            }
            (FieldKind::Text, Value::Str(bytes)) => {
                let text = CString::new(
                    bytes
                        .iter()
                        .copied()
                        .filter(|byte| *byte != 0)
                        .collect::<Vec<_>>(),
                )
                .unwrap_or_default();
                (address as *mut *const c_char).write_unaligned(text.as_ptr());
                strings.push(text);
            }
            (FieldKind::Instance(_) | FieldKind::Pointer, other) => {
                (address as *mut usize).write_unaligned(native(other))
            }
            _ => {}
        }
    }
}
