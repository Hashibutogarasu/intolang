use ast::Expr;
use moduler::Module;
use native::{
    CastAssign, CompareAssign, Comparison, FieldAssign, FieldLoad, FunctionCall,
    FunctionDefinition, FunctionRef, FunctionReturn, IndirectCall, NativeArg, Operand,
    OwnershipChange, StateCheck,
};
use parser::{Hex, Record};
use types::{ComparisonOperator, Literal, MessageTag, Type};

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

pub fn compare_assign(output: &mut String, compare: &CompareAssign) {
    push(output, compare.to_record());
}

pub fn cast_assign(output: &mut String, cast: &CastAssign) {
    push(output, cast.to_record());
}

pub fn field_assign(output: &mut String, assign: &FieldAssign) {
    push(output, assign.to_record());
}

pub fn field_load(output: &mut String, load: &FieldLoad) {
    push(output, load.to_record());
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

/// Make `target` refer to the storage `source` holds, as `move` and `borrow`
/// do: no memory is copied.
pub fn alias(output: &mut String, target: &str, source: &str) {
    push(
        output,
        Record::new(MessageTag::Alias).field(target).field(source),
    );
}

/// Copy the whole instance `source` holds into new storage for `target`.
pub fn copy(output: &mut String, target: &str, source: &str, class: &str) {
    push(
        output,
        Record::new(MessageTag::Copy)
            .field(target)
            .field(source)
            .field(class),
    );
}

/// The hidden `Int32` slot telling whether the scope that declared `name`
/// still has to destroy it.
pub fn drop_flag_name(name: &str) -> String {
    format!("__drop_{name}")
}

pub fn drop_flag(output: &mut String, name: &str, owned: bool) {
    assign(
        output,
        &drop_flag_name(name),
        &Type::Int32,
        "n",
        flag(owned),
    );
}

/// Open an `if` that runs only while the drop flag of `name` is set.
pub fn drop_guard(output: &mut String, name: &str) {
    let condition = Comparison {
        left: drop_flag_name(name),
        operator: ComparisonOperator::Equal,
        operand: Operand::Number("1".to_owned()),
        type_name: Type::Int32.name(),
    };
    comparison(output, MessageTag::IfBegin, &condition);
}

/// Open the function a handler body is generated into.
pub fn handler_begin(
    output: &mut String,
    name: &str,
    captures: &[String],
    params: &[(String, Type)],
) {
    let mut record = Record::new(MessageTag::HandlerBegin)
        .field(name)
        .field(captures.join(","));
    for (param, ty) in params {
        record = record.field(format!("{param}:{ty}"));
    }
    push(output, record);
}

pub fn handler_end(output: &mut String) {
    bare(output, MessageTag::HandlerEnd);
}

/// Register the handler function `handler` with `event`; the registration
/// number goes to the slot `listener` unless it is empty.
pub fn event_on(output: &mut String, event: &str, listener: &str, handler: &str) {
    push(
        output,
        Record::new(MessageTag::EventOn)
            .field(event)
            .field(listener)
            .field(handler),
    );
}

/// Open the function `definition`; `NATIVE_HANDLER_END` closes it.
pub fn function_begin(output: &mut String, definition: &FunctionDefinition) {
    push(output, definition.to_record());
}

pub fn function_call(output: &mut String, call: &FunctionCall) {
    push(output, call.to_record());
}

pub fn function_return(output: &mut String, result: &FunctionReturn) {
    push(output, result.to_record());
}

pub fn function_ref(output: &mut String, reference: &FunctionRef) {
    push(output, reference.to_record());
}

pub fn call_indirect(output: &mut String, call: &IndirectCall) {
    push(output, call.to_record());
}

pub fn event_emit(output: &mut String, event: &str, args: &[NativeArg]) {
    let mut record = Record::new(MessageTag::Emit).field(event);
    for arg in args {
        record = record.field(arg);
    }
    push(output, record);
}

pub fn event_off(output: &mut String, event: &str, listener: &str) {
    push(
        output,
        Record::new(MessageTag::EventOff)
            .field(event)
            .field(listener),
    );
}

pub fn event_destroy(output: &mut String, event: &str) {
    push(output, Record::new(MessageTag::EventDestroy).field(event));
}

/// Tell the VM what happened to a binding or to the instance it holds.
pub fn ownership(output: &mut String, change: &OwnershipChange) {
    push(output, change.to_record());
}

/// Open an `if` on a property of a binding, such as `isOwned`.
pub fn if_state(output: &mut String, check: &StateCheck) {
    push(output, check.to_record());
}

pub fn throw(output: &mut String, message: &str) {
    push(
        output,
        Record::new(MessageTag::Throw).field(Hex::from(message)),
    );
}

/// Start the thread running the function `function`; its task goes to the
/// slot `task`.
pub fn thread_run(output: &mut String, task: &str, function: &str) {
    push(
        output,
        Record::new(MessageTag::ThreadRun)
            .field(task)
            .field(function),
    );
}

pub fn await_task(output: &mut String, task: &str) {
    push(output, Record::new(MessageTag::Await).field(task));
}

/// A call to an imported function that is lowered to a plain `NATIVE` record.
pub fn native_call(output: &mut String, symbol: &str, argument: &NativeArg) {
    let (kind, value) = match argument {
        NativeArg::Int(number) => ("i", number.to_string()),
        NativeArg::Str(bytes) => ("s", Hex::from(bytes.as_slice()).to_string()),
        NativeArg::Slot(name) => ("p", name.clone()),
        NativeArg::Field { slot, class, index } => ("f", format!("{slot}:{class}:{index}")),
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
/// Kinds are `n` (numeric literal), `b` (boolean), `s` (hex-encoded string),
/// `c` (a new instance of the named class) and `-` (no default,
/// zero-initialised).
fn encode_default(default: Option<&Expr>) -> (char, String) {
    match default {
        None => ('-', String::new()),
        Some(Expr::Literal(Literal::String(value))) => ('s', Hex::from(value.as_str()).to_string()),
        Some(Expr::Literal(Literal::Bool(value))) => ('b', flag(*value).to_owned()),
        Some(Expr::Literal(Literal::Char(value))) => ('n', u32::from(*value).to_string()),
        Some(Expr::Call { callee, args }) if args.is_empty() && callee.as_ident().is_some() => {
            ('c', callee.first().to_owned())
        }
        Some(Expr::Literal(Literal::Int(value))) => ('n', value.to_string()),
        Some(Expr::Literal(Literal::Float(value))) => ('n', value.to_string()),
        Some(other) => ('n', other.to_string()),
    }
}
