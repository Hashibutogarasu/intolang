use anyhow::{Result, anyhow, bail};
use std::ffi::CString;

use super::vm::Vm;
use crate::native_call::{
    MAX_ARGUMENTS, Returned, Returns, StructLayout, invoke, read_field, write_field,
};
use crate::value::Value;

struct Materialized {
    index: usize,
    class: String,
    buffer: Vec<u64>,
}

fn returns_of(return_type: &str) -> Returns {
    match return_type {
        "Float32" => Returns::Float32,
        "Float64" => Returns::Float64,
        _ => Returns::Integer,
    }
}

fn raw_of(value: &Value) -> usize {
    match value {
        Value::Int(number) => *number as usize,
        Value::Native { ptr, .. } => *ptr,
        _ => 0,
    }
}

impl Vm {
    /// The layout of `class` as native code sees it.
    pub(super) fn layout_of(&self, class: &str) -> StructLayout {
        StructLayout::of(class, &self.program.classes)
    }

    /// Read the field `index` of an instance the VM or native code owns.
    pub(super) fn read_instance_field(&self, value: &Value, index: usize) -> Result<Value> {
        match value {
            Value::Ref(reference) => self
                .instance(*reference)?
                .fields
                .get(index)
                .cloned()
                .ok_or_else(|| anyhow!("Unknown field index {index}")),
            Value::Native { ptr, class } => {
                let layout = self.layout_of(class);
                let field = layout
                    .fields
                    .get(index)
                    .ok_or_else(|| anyhow!("Unknown field index {index} of `{class}`"))?;
                Ok(unsafe { read_field(*ptr, field) })
            }
            _ => bail!("The value holds no instance"),
        }
    }

    /// The class an instance the VM or native code owns belongs to.
    pub(super) fn class_of(&self, value: &Value) -> Result<String> {
        match value {
            Value::Ref(reference) => Ok(self.instance(*reference)?.class.clone()),
            Value::Native { class, .. } => Ok(class.clone()),
            _ => bail!("The value holds no instance"),
        }
    }

    fn to_raw(
        &self,
        value: &Value,
        strings: &mut Vec<CString>,
        buffers: &mut Vec<Materialized>,
    ) -> Result<usize> {
        match value {
            Value::Int(number) => Ok(*number as usize),
            Value::Native { ptr, .. } => Ok(*ptr),
            Value::Unit => Ok(0),
            Value::Float(_) => bail!("Floating point arguments are not supported in native calls"),
            Value::Str(bytes) => {
                let text = CString::new(
                    bytes
                        .iter()
                        .copied()
                        .filter(|byte| *byte != 0)
                        .collect::<Vec<_>>(),
                )?;
                let pointer = text.as_ptr() as usize;
                strings.push(text);
                Ok(pointer)
            }
            Value::Ref(reference) => {
                let instance = self.instance(*reference)?.clone();
                let layout = self.layout_of(&instance.class);
                let mut buffer = vec![0u64; layout.size / 8 + 1];
                let base = buffer.as_mut_ptr() as usize;
                for (field, content) in layout.fields.iter().zip(&instance.fields) {
                    unsafe { write_field(base, field, content, strings, &raw_of) };
                }
                buffers.push(Materialized {
                    index: *reference,
                    class: instance.class,
                    buffer,
                });
                Ok(base)
            }
        }
    }

    fn from_returned(&self, returned: Returned, return_type: &str) -> Value {
        let class = return_type.strip_suffix('&').unwrap_or(return_type);
        match returned {
            Returned::Float(number) => Value::Float(number),
            Returned::Integer(raw) => match class {
                "" | "Void" => Value::Unit,
                "Int8" => Value::Int(i64::from(raw as i8)),
                "Int16" => Value::Int(i64::from(raw as i16)),
                "Int32" => Value::Int(i64::from(raw as i32)),
                "Boolean" => Value::Int(i64::from(raw as u8 != 0)),
                "String" | "Path" => {
                    if raw == 0 {
                        Value::Str(Vec::new())
                    } else {
                        let text =
                            unsafe { std::ffi::CStr::from_ptr(raw as *const std::ffi::c_char) };
                        Value::Str(text.to_bytes().to_vec())
                    }
                }
                other if raw != 0 && self.program.classes.contains_key(other) => Value::Native {
                    ptr: raw,
                    class: other.to_owned(),
                },
                _ => Value::Int(raw as i64),
            },
        }
    }

    /// Call `symbol` in the native library of the program, if it defines it.
    pub(super) fn call_native(
        &mut self,
        symbol: &str,
        args: &[Value],
        return_type: &str,
    ) -> Result<Option<Value>> {
        let Some(library) = self.native.clone() else {
            return Ok(None);
        };
        let Some(function) = library.symbol(symbol)? else {
            return Ok(None);
        };
        if args.len() > MAX_ARGUMENTS {
            bail!("`{symbol}` takes more than {MAX_ARGUMENTS} arguments");
        }
        let mut strings = Vec::new();
        let mut buffers = Vec::new();
        let raw = args
            .iter()
            .map(|argument| self.to_raw(argument, &mut strings, &mut buffers))
            .collect::<Result<Vec<_>>>()?;
        let returned = unsafe { invoke(function, &raw, returns_of(return_type)) }
            .ok_or_else(|| anyhow!("`{symbol}` takes more than {MAX_ARGUMENTS} arguments"))?;
        for text in strings {
            let _ = text.into_raw();
        }
        for materialized in &buffers {
            let layout = self.layout_of(&materialized.class);
            let base = materialized.buffer.as_ptr() as usize;
            for (position, field) in layout.fields.iter().enumerate() {
                let value = unsafe { read_field(base, field) };
                if let Some(instance) = self.heap.get_mut(materialized.index)
                    && let Some(slot) = instance.fields.get_mut(position)
                {
                    *slot = value;
                }
            }
        }
        Ok(Some(self.from_returned(returned, return_type)))
    }
}
