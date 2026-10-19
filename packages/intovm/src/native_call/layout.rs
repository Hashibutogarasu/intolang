use std::collections::HashMap;

use crate::value::ClassDef;

/// How a field is stored in the C struct of its class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldKind {
    /// An integer of this many bytes.
    Int(usize),
    Float32,
    Float64,
    /// A pointer to NUL-terminated text.
    Text,
    /// A pointer to an instance of the class.
    Instance(String),
    /// Any other pointer.
    Pointer,
}

#[derive(Debug, Clone)]
pub struct FieldLayout {
    pub offset: usize,
    pub kind: FieldKind,
}

/// The C layout of a class: its fields in order, each at its natural
/// alignment.
#[derive(Debug, Clone, Default)]
pub struct StructLayout {
    pub fields: Vec<FieldLayout>,
    pub size: usize,
}

fn kind_of(ty: &str, classes: &HashMap<String, ClassDef>) -> (FieldKind, usize) {
    match ty {
        "Int8" | "Boolean" => (FieldKind::Int(1), 1),
        "Int16" => (FieldKind::Int(2), 2),
        "Int32" => (FieldKind::Int(4), 4),
        "Int64" => (FieldKind::Int(8), 8),
        "Float32" => (FieldKind::Float32, 4),
        "Float64" => (FieldKind::Float64, 8),
        "String" | "Path" => (FieldKind::Text, 8),
        other => {
            let class = other.strip_suffix('&').unwrap_or(other);
            if classes.contains_key(class) {
                (FieldKind::Instance(class.to_owned()), 8)
            } else {
                (FieldKind::Pointer, 8)
            }
        }
    }
}

impl StructLayout {
    pub fn of(class: &str, classes: &HashMap<String, ClassDef>) -> Self {
        let Some(definition) = classes.get(class) else {
            return Self::default();
        };
        let mut layout = Self::default();
        let mut alignment = 1;
        for field in &definition.fields {
            let (kind, size) = kind_of(&field.ty, classes);
            let offset = layout.size.next_multiple_of(size);
            layout.fields.push(FieldLayout { offset, kind });
            layout.size = offset + size;
            alignment = alignment.max(size);
        }
        layout.size = layout.size.next_multiple_of(alignment);
        layout
    }
}
