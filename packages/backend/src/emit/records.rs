use ast::Expr;
use moduler::Module;
use native::{Comparison, NativeArg, Operand};
use parser::{Hex, Record};
use types::{Literal, MessageTag, Type};

use crate::context::FieldDef;

fn flag(newline: bool) -> &'static str {
    if newline { "1" } else { "0" }
}

fn push(output: &mut String, record: Record) {
    output.push_str(&record.to_string());
}

/// A record with no fields, such as `NATIVE_BREAK`.
pub fn bare(output: &mut String, tag: MessageTag) {
    push(output, Record::new(tag));
}

pub fn assign(output: &mut String, target: &str, ty: &Type, kind: &str, value: &str) {
    push(
        output,
        Record::new(MessageTag::Assign)
            .field(target)
            .field(ty)
            .field(format!("{kind}:{value}")),
    );
}

pub fn arith(
    output: &mut String,
    assigned_to: &str,
    left: &str,
    operation: &str,
    operand: &Operand,
    ty: &Type,
) {
    push(
        output,
        Record::new(MessageTag::Arith)
            .field(assigned_to)
            .field(left)
            .field(operation)
            .field(operand)
            .field(ty),
    );
}

pub fn print(output: &mut String, name: &str, ty: &Type, newline: bool) {
    push(
        output,
        Record::new(MessageTag::Print)
            .field(name)
            .field(ty)
            .field(flag(newline)),
    );
}

pub fn print_const(output: &mut String, ty: &Type, value: &str, newline: bool) {
    push(
        output,
        Record::new(MessageTag::PrintConst)
            .field(ty)
            .field(format!("n:{value}"))
            .field(flag(newline)),
    );
}

pub fn field_print(
    output: &mut String,
    base: &str,
    class: &str,
    index: usize,
    field_type: &str,
    newline: bool,
) {
    push(
        output,
        Record::new(MessageTag::FieldPrint)
            .field(base)
            .field(class)
            .field(index)
            .field(field_type)
            .field(flag(newline)),
    );
}

pub fn comparison(output: &mut String, tag: MessageTag, comparison: &Comparison) {
    push(output, comparison.to_record(tag));
}

pub fn if_const(output: &mut String, value: bool) {
    push(output, Record::new(MessageTag::IfConst).field(flag(value)));
}

pub fn construct(output: &mut String, target: &str, class: &str) {
    push(
        output,
        Record::new(MessageTag::Construct)
            .field(target)
            .field(class),
    );
}

/// A call to an imported function that is lowered to a plain `NATIVE` record.
pub fn native_call(output: &mut String, symbol: &str, argument: &NativeArg) {
    let (kind, value) = match argument {
        NativeArg::Int(number) => ("i", number.to_string()),
        NativeArg::Str(bytes) => ("s", Hex::from(bytes.as_slice()).to_string()),
        NativeArg::Slot(name) => ("p", name.clone()),
    };
    push(
        output,
        Record::new(MessageTag::Native)
            .field(symbol)
            .field(kind)
            .field(value),
    );
}

/// Program output that is known at compile time.
pub fn text(output: &mut String, text: &str, newline: bool) {
    output.push_str(text);
    if newline {
        output.push('\n');
    }
}

/// Emit the `PackageInfo` and `Module` class layouts the module machinery uses.
pub fn module_classes(output: &mut String) {
    let mut package = Record::new(MessageTag::Class).field("PackageInfo");
    for field in [
        "alias:String",
        "name:String",
        "version:String",
        "licence:String",
        "root:Path",
    ] {
        package = package.field(format!("{field}:-:"));
    }
    push(output, package);
    push(
        output,
        Record::new(MessageTag::Class)
            .field("Module")
            .field("package:PackageInfo:-:")
            .field("sourcePath:Path:-:"),
    );
}

/// Emit a `NATIVE_MODULE` record describing a resolved module instance.
pub fn module(output: &mut String, index: usize, module: &Module) {
    let root = module.package.root.to_string_lossy();
    let source_path = module.source_path.to_string_lossy();
    let mut record = Record::new(MessageTag::Module).field(index);
    for field in [
        module.name.as_str(),
        module.package.alias.as_str(),
        module.package.name.as_str(),
        module.package.version.as_str(),
        module.package.licence.as_str(),
        root.as_ref(),
        source_path.as_ref(),
    ] {
        record = record.field(Hex::from(field));
    }
    push(output, record);
}

/// Emit a `NATIVE_CLASS` record with the instance fields of a class.
pub fn class(output: &mut String, name: &str, fields: &[FieldDef]) {
    let mut record = Record::new(MessageTag::Class).field(name);
    for field in fields.iter().filter(|field| !field.is_static) {
        let (kind, value) = encode_default(field.default.as_ref());
        record = record.field(format!("{}:{}:{kind}:{value}", field.name, field.ty));
    }
    push(output, record);
}

/// Encode a class field default as `(kind, value)` for the compiler message.
///
/// Kinds are `n` (numeric literal), `b` (boolean), `s` (hex-encoded string) and
/// `-` (no default, zero-initialised).
fn encode_default(default: Option<&Expr>) -> (char, String) {
    match default {
        None => ('-', String::new()),
        Some(Expr::Literal(Literal::String(value))) => ('s', Hex::from(value.as_str()).to_string()),
        Some(Expr::Literal(Literal::Bool(value))) => ('b', flag(*value).to_owned()),
        Some(Expr::Literal(Literal::Int(value))) => ('n', value.to_string()),
        Some(Expr::Literal(Literal::Float(value))) => ('n', value.to_string()),
        Some(other) => ('n', other.to_string()),
    }
}
