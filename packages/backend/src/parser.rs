use anyhow::{Result, anyhow};
use std::{collections::HashMap, path::Path};

use crate::{imports, lexer::tokenize};

/// Parse the currently supported statement subset and lower it to output.
pub(super) fn lower(source: &str, packages_dir: Option<&Path>) -> Result<String> {
    lower_named(source, packages_dir, "<source>")
}

struct Variable {
    value: String,
    memory_backed: bool,
    allocated: bool,
    lazy: bool,
    unloaded: bool,
    declared_in_init: bool,
}

fn diagnostic(path: &str, line: usize, scope: &str, message: &str) -> anyhow::Error {
    anyhow!("{message}\n\nInto language backtrace:\n  0: {scope} at {path}:{line}")
}

pub(super) fn lower_named(source: &str, packages_dir: Option<&Path>, source_path: &str) -> Result<String> {
    let _tokens = tokenize(source)?;
    if let Some(packages_dir) = packages_dir { imports::resolve(source, packages_dir)?; }

    let mut values: HashMap<String, Variable> = HashMap::new();
    let mut output = String::new();
    let mut in_init = false;
    let mut init_has_run = false;

    for (index, raw) in source.lines().enumerate() {
        let line_number = index + 1;
        let (code, _) = raw.split_once("//").unwrap_or((raw, ""));
        let line = code.trim();
        if line.starts_with("import ") || line.is_empty() { continue; }
        if line.starts_with("init") && line.contains('{') { in_init = true; init_has_run = true; continue; }
        if line == "}" { in_init = false; continue; }

        let declaration = line.trim_end_matches(';');
        let (prefix, decl) = if let Some(rest) = declaration.strip_prefix("const now into ") { ("now", rest) }
            else if let Some(rest) = declaration.strip_prefix("now into ") { ("now", rest) }
            else if let Some(rest) = declaration.strip_prefix("late into ") { ("late", rest) }
            else if let Some(rest) = declaration.strip_prefix("into ") { ("local", rest) }
            else { ("", "") };
        if !prefix.is_empty() {
            if let Some((name, rhs)) = decl.split_once('=') {
                let value = rhs.trim().trim_matches('"').to_owned();
                let memory = prefix != "local";
                values.insert(name.trim().to_owned(), Variable {
                    value,
                    memory_backed: memory,
                    allocated: prefix == "now",
                    lazy: prefix == "late",
                    unloaded: false,
                    declared_in_init: in_init,
                });
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("unload ") {
            let name = rest.trim_end_matches(';').trim();
            if let Some(variable) = values.get_mut(name) {
                variable.allocated = false;
                variable.unloaded = true;
            }
            continue;
        }
        let Some((is_println, rest)) = line.strip_prefix("println(").map(|r| (true, r))
            .or_else(|| line.strip_prefix("print(").map(|r| (false, r))) else { continue; };
        let arg = rest.split(')').next().unwrap_or("").trim();
        let (value, newline) = if arg.starts_with('"') && arg.ends_with('"') {
            (arg[1..arg.len()-1].to_owned(), is_println)
        } else if let Some(variable) = values.get_mut(arg) {
            if (!variable.memory_backed && !in_init) || (variable.declared_in_init && !in_init) {
                let scope = if init_has_run { "module scope (after init)" } else { "module scope" };
                return Err(diagnostic(source_path, line_number, scope,
                    &format!("Variable `{arg}` is out of scope here")));
            }
            if variable.memory_backed && !variable.allocated && variable.lazy && !variable.unloaded {
                variable.allocated = true;
            }
            if variable.memory_backed && !variable.allocated {
                return Err(diagnostic(source_path, line_number, if in_init { "init" } else { "module" },
                    &format!("Variable `{arg}` was unloaded")));
            }
            (variable.value.clone(), is_println)
        } else {
            let scope = if in_init { "init" } else { "module" };
            return Err(diagnostic(source_path, line_number, scope,
                &format!("Undefined variable `{arg}`")));
        };
        output.push_str(&value);
        if newline { output.push('\n'); }
    }
    Ok(output)
}
