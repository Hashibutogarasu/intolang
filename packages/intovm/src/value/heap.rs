use super::Value;

/// A class field with the default `NATIVE_CLASS` gives it.
#[derive(Debug, Clone)]
pub struct FieldDef {
    pub name: String,
    pub ty: String,
    pub default: Option<(char, String)>,
}

#[derive(Debug, Clone, Default)]
pub struct ClassDef {
    pub fields: Vec<FieldDef>,
}

/// The storage behind a [`Value::Ref`].
#[derive(Debug, Clone)]
pub struct Instance {
    pub class: String,
    pub fields: Vec<Value>,
}
