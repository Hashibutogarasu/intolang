use ast::{ClassDecl, Declaration, EventDecl, FieldDecl, FunctionDecl, MacroDecl, Param};
use types::Storage;

pub fn param(param: &Param) -> String {
    let mut text = param.name.clone();
    if let Some(ty) = &param.ty {
        text.push_str(&format!(": {ty}"));
        if param.optional {
            text.push('?');
        }
        if param.variadic {
            text.push_str("...");
        }
    }
    if let Some(default) = &param.default {
        text.push_str(&format!(" = {default}"));
    }
    text
}

fn params(params: &[Param]) -> String {
    params.iter().map(param).collect::<Vec<_>>().join(", ")
}

fn type_params(names: &[String]) -> String {
    if names.is_empty() {
        String::new()
    } else {
        format!("<{}>", names.join(", "))
    }
}

pub fn function(decl: &FunctionDecl) -> String {
    let mut text = String::new();
    for (flag, word) in [
        (decl.is_public, "pub "),
        (decl.is_static, "static "),
        (decl.is_abstract, "abstract "),
        (decl.is_default, "default "),
        (decl.is_override, "override "),
    ] {
        if flag {
            text.push_str(word);
        }
    }
    text.push_str(&format!(
        "fun {}{}({})",
        decl.name,
        type_params(&decl.type_params),
        params(&decl.params)
    ));
    if let Some(ty) = &decl.return_type {
        text.push_str(&format!(": {ty}"));
    }
    text
}

pub fn event(decl: &EventDecl) -> String {
    let mut text = String::new();
    for (flag, word) in [
        (decl.is_public, "pub "),
        (decl.is_static, "static "),
        (decl.is_default, "default "),
        (decl.is_override, "override "),
    ] {
        if flag {
            text.push_str(word);
        }
    }
    text.push_str(&format!(
        "event {}{}({})",
        decl.name,
        type_params(&decl.type_params),
        params(&decl.params)
    ));
    text
}

pub fn class(decl: &ClassDecl) -> String {
    let mut text = String::new();
    if decl.is_public {
        text.push_str("pub ");
    }
    if decl.is_abstract {
        text.push_str("abstract ");
    }
    text.push_str(&format!(
        "{}{}()",
        decl.name,
        type_params(&decl.type_params)
    ));
    if let Some(parent) = &decl.parent_type {
        text.push_str(&format!(": {parent}"));
    }
    text
}

pub fn field(decl: &FieldDecl) -> String {
    let mut text = String::from("pub ");
    if decl.is_static {
        text.push_str("static ");
    }
    text.push_str(&format!("into: {} {}", decl.ty, decl.name));
    if let Some(default) = &decl.default {
        text.push_str(&format!(" = {default}"));
    }
    text
}

pub fn mac(decl: &MacroDecl) -> String {
    let public = if decl.is_public { "pub " } else { "" };
    format!("{public}macro {}", decl.name)
}

pub fn local(decl: &Declaration, ty: Option<&str>) -> String {
    let mut text = String::new();
    if decl.constant {
        text.push_str("const ");
    }
    match decl.storage {
        Storage::Now => text.push_str("now "),
        Storage::Late => text.push_str("late "),
        Storage::Local => {}
    }
    text.push_str("into ");
    if decl.pointer {
        text.push('*');
    }
    text.push_str(&decl.name);
    if let Some(ty) = ty {
        text.push_str(&format!(": {ty}"));
    }
    text
}
