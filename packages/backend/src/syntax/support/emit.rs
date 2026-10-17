use std::collections::HashMap;

use crate::syntax::context::Variable;
use crate::syntax::support::text::decode_string_literal;
use crate::types::Type;
use moduler::{Module, NativeLink};

pub fn arithmetic_operand(
    right: &str,
    values: &HashMap<String, Variable>,
) -> Option<(String, String)> {
    let right = right.trim();
    if let Ok(number) = right.parse::<i64>() {
        return Some(("n".into(), number.to_string()));
    }
    if let Ok(number) = right.parse::<f64>() {
        return Some(("n".into(), number.to_string()));
    }
    if let Some(variable) = values.get(right) {
        if variable.native && variable.ty.is_numeric() {
            return Some(("p".into(), right.to_owned()));
        }
    }
    None
}

pub fn comparison_operand(
    right: &str,
    values: &HashMap<String, Variable>,
) -> Option<(String, String)> {
    let right = right.trim();
    if let Ok(number) = right.parse::<i64>() {
        return Some(("n".into(), number.to_string()));
    }
    if let Ok(number) = right.parse::<f64>() {
        return Some(("n".into(), number.to_string()));
    }
    if let Some(variable) = values.get(right) {
        if variable.native && variable.ty.is_numeric() {
            return Some(("p".into(), right.to_owned()));
        }
    }
    None
}

pub fn emit_comparison(
    output: &mut String,
    tag: &str,
    left: &str,
    operator: &str,
    operand: &(String, String),
    ty: &Type,
) {
    output.push('\u{1e}');
    output.push_str(tag);
    output.push('\t');
    output.push_str(left);
    output.push('\t');
    output.push_str(operator);
    output.push('\t');
    output.push_str(&operand.0);
    output.push(':');
    output.push_str(&operand.1);
    output.push('\t');
    output.push_str(&ty.name());
    output.push('\n');
}

pub fn emit_assign(output: &mut String, target: &str, ty: &Type, kind: &str, value: &str) {
    output.push('\u{1e}');
    output.push_str("NATIVE_ASSIGN\t");
    output.push_str(target);
    output.push('\t');
    output.push_str(&ty.name());
    output.push('\t');
    output.push_str(kind);
    output.push(':');
    output.push_str(value);
    output.push('\n');
}

pub fn emit_print(output: &mut String, name: &str, ty: &Type, newline: bool) {
    output.push('\u{1e}');
    output.push_str("NATIVE_PRINT\t");
    output.push_str(name);
    output.push('\t');
    output.push_str(&ty.name());
    output.push('\t');
    output.push_str(if newline { "1" } else { "0" });
    output.push('\n');
}

pub fn emit_print_const(output: &mut String, ty: &Type, value: &str, newline: bool) {
    output.push('\u{1e}');
    output.push_str("NATIVE_PRINT_CONST\t");
    output.push_str(&ty.name());
    output.push('\t');
    output.push_str("n:");
    output.push_str(value);
    output.push('\t');
    output.push_str(if newline { "1" } else { "0" });
    output.push('\n');
}

/// Encode `value` as lowercase hexadecimal, one pair of digits per byte.
pub fn to_hex(value: &str) -> String {
    let mut hex = String::with_capacity(value.len() * 2);
    for byte in value.bytes() {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

/// Emit the `PackageInfo` and `Module` class layouts the module machinery uses.
pub fn emit_module_classes(output: &mut String) {
    output.push('\u{1e}');
    output.push_str("NATIVE_CLASS\tPackageInfo");
    for field in [
        "alias:String",
        "name:String",
        "version:String",
        "licence:String",
        "root:Path",
    ] {
        output.push('\t');
        output.push_str(field);
        output.push_str(":-:");
    }
    output.push('\n');
    output.push('\u{1e}');
    output.push_str("NATIVE_CLASS\tModule\tpackage:PackageInfo:-:\tsourcePath:Path:-:\n");
}

/// Emit a `NATIVE_MODULE` record describing a resolved module instance.
pub fn emit_module(output: &mut String, index: usize, module: &Module) {
    output.push('\u{1e}');
    output.push_str("NATIVE_MODULE\t");
    output.push_str(&index.to_string());
    let root = module.package.root.to_string_lossy();
    let source_path = module.source_path.to_string_lossy();
    for field in [
        module.name.as_str(),
        module.package.alias.as_str(),
        module.package.name.as_str(),
        module.package.version.as_str(),
        module.package.licence.as_str(),
        root.as_ref(),
        source_path.as_ref(),
    ] {
        output.push('\t');
        output.push_str(&to_hex(field));
    }
    output.push('\n');
}

pub fn emit_link_call(
    output: &mut String,
    link: &NativeLink,
    args: &str,
    enums: &HashMap<String, HashMap<String, i32>>,
    values: &HashMap<String, Variable>,
    assigned_to: &str,
    has_receiver: bool,
) -> std::result::Result<(), String> {
    let mut provided = args
        .split(',')
        .map(str::trim)
        .filter(|arg| !arg.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if linked_argument_count(&provided, has_receiver) > link.params.len() {
        return Err(format!(
            "Function `{}` expects {} argument(s) but received {}",
            link.target,
            link.params.len(),
            linked_argument_count(&provided, has_receiver)
        ));
    }
    for index in linked_argument_count(&provided, has_receiver)..link.params.len() {
        let param = &link.params[index];
        match &param.default {
            Some(default) => provided.push(default.clone()),
            None => {
                return Err(format!("Missing required argument `{}`", param.name));
            }
        }
    }
    output.push('\u{1e}');
    output.push_str("NATIVE_LINK_ASSIGN\t");
    output.push_str(&link.package_alias);
    output.push('\t');
    output.push_str(&link.language);
    output.push('\t');
    output.push_str(&link.source_path);
    output.push('\t');
    output.push_str(&link.target);
    output.push('\t');
    output.push_str(&link.return_type);
    output.push('\t');
    output.push_str(assigned_to);
    for arg in provided {
        let arg = arg.as_str();
        let (kind, value) = if arg.starts_with('"') && arg.ends_with('"') {
            ("s", decode_string_literal(&arg[1..arg.len() - 1]))
        } else if let Ok(number) = arg.parse::<i32>() {
            ("i", number.to_string())
        } else if let Some((enum_name, variant)) = arg.split_once('.') {
            let value = enums
                .get(enum_name)
                .and_then(|variants| variants.get(variant))
                .ok_or_else(|| format!("Undefined enum variant `{arg}`"))?;
            ("i", value.to_string())
        } else if let Some(variable) = values.get(arg) {
            if variable.native {
                ("p", arg.to_owned())
            } else {
                ("s", variable.value.clone())
            }
        } else {
            return Err(format!("Unsupported native argument `{arg}`"));
        };
        output.push('\t');
        output.push_str(kind);
        output.push(':');
        if kind == "i" || kind == "p" {
            output.push_str(&value);
        } else {
            for byte in value.bytes() {
                output.push_str(&format!("{byte:02x}"));
            }
        }
    }
    output.push('\n');
    Ok(())
}

pub fn linked_argument_count(provided: &[String], has_receiver: bool) -> usize {
    provided
        .len()
        .saturating_sub(if has_receiver { 1 } else { 0 })
}
