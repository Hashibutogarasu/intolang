use anyhow::{Result, anyhow, bail};
use std::{collections::HashMap, path::Path};

use crate::imports;
use crate::validation;

#[derive(Debug, Default)]
pub struct TestSummary {
    pub passed: usize,
    pub failed: usize,
}

#[derive(Clone)]
enum Storage { Local, Now, Late }

#[derive(Clone)]
enum Value { Bool(bool), Object { storage: Storage, allocated: bool, in_init: bool } }

#[derive(Clone)]
struct Function { params: Vec<String>, body: Vec<String> }

pub(super) fn run(source: &str, path: &Path, packages_dir: &Path) -> Result<TestSummary> {
    validation::function_scopes(source, &path.display().to_string())?;
    let imported = imports::resolve(source, packages_dir)?;
    let package = imports::find_by_alias(packages_dir, "builtin")?
        .ok_or_else(|| anyhow!("Builtin package is not bundled"))?;
    let mut functions = HashMap::new();
    for (module, function) in [
        ("tests/assert", "assert"),
        ("tests/memory-check", "memoryExists"),
        ("tests/memory-check", "memoryNotExists"),
    ] {
        if !imported.contains(function) { bail!("{}: test module must import `{function}`", path.display()); }
        let source_path = package.join(format!("src/main/builtin/{module}.into"));
        let content = std::fs::read_to_string(&source_path)?;
        functions.extend(parse_functions(&content)?);
    }
    if !imported.contains("test") { bail!("{}: test module must import the test macro", path.display()); }
    let macro_path = package.join("src/main/builtin/tests/macros/test.into");
    let macro_source = std::fs::read_to_string(macro_path)?;
    let annotation = macro_source.lines().map(str::trim)
        .find(|line| line.starts_with("#[test]"))
        .ok_or_else(|| anyhow!("test macro does not define the #[test] annotation"))?;
    let test_annotation = annotation;
    let source_lines: Vec<String> = source.lines().map(str::to_owned).collect();

    let mut summary = TestSummary::default();
    let mut index = 0;
    while index < source_lines.len() {
        if source_lines[index].trim() != test_annotation { index += 1; continue; }
        index += 1;
        while index < source_lines.len() && !source_lines[index].contains("fun ") { index += 1; }
        if index == source_lines.len() { bail!("{}: #[test] is not followed by a function", path.display()); }
        let signature = source_lines[index].trim();
        let name = signature.split("fun ").nth(1).unwrap_or("<test>")
            .split('(').next().unwrap_or("<test>").trim().to_owned();
        let mut depth = 0i32;
        let mut started = false;
        let mut body = Vec::new();
        while index < source_lines.len() {
            let line = source_lines[index].as_str();
            if line.contains('{') { started = true; }
            if started { body.push((index + 1, line)); }
            depth += line.chars().filter(|c| *c == '{').count() as i32;
            depth -= line.chars().filter(|c| *c == '}').count() as i32;
            index += 1;
            if started && depth == 0 { break; }
        }
        match run_test_body(&body, path, &name, &functions) {
            Ok(()) => { summary.passed += 1; println!("test {name} ... ok"); }
            Err(error) => { summary.failed += 1; eprintln!("{error:#}"); }
        }
    }
    if summary.passed + summary.failed == 0 { bail!("{}: no #[test] functions found", path.display()); }
    Ok(summary)
}

fn parse_functions(source: &str) -> Result<HashMap<String, Function>> {
    let lines: Vec<_> = source.lines().collect();
    let mut parsed = HashMap::new();
    let mut index = 0;
    while index < lines.len() {
        let signature = lines[index].trim();
        if !signature.starts_with("pub fun ") { index += 1; continue; }
        let rest = signature.trim_start_matches("pub fun ");
        let name = rest.split('(').next().unwrap_or("").trim().to_owned();
        let params = rest.split_once('(').and_then(|(_, tail)| tail.split_once(')'))
            .map(|(params, _)| params.split(',').filter_map(|param| param.split(':').next())
                .map(str::trim).filter(|name| !name.is_empty()).map(str::to_owned).collect())
            .unwrap_or_default();
        let mut depth = 0i32;
        let mut started = false;
        let mut body = Vec::new();
        while index < lines.len() {
            let line = lines[index];
            if line.contains('{') { started = true; }
            depth += line.chars().filter(|c| *c == '{').count() as i32;
            depth -= line.chars().filter(|c| *c == '}').count() as i32;
            if started && index > 0 && depth > 0 { body.push(line.trim().to_owned()); }
            index += 1;
            if started && depth == 0 { break; }
        }
        parsed.insert(name, Function { params, body });
    }
    Ok(parsed)
}

fn run_test_body(lines: &[(usize, &str)], path: &Path, test: &str, functions: &HashMap<String, Function>) -> Result<()> {
    let mut env = HashMap::new();
    let mut in_init = false;
    for (line_no, raw) in lines {
        let code = raw.split_once("//").map(|(code, _)| code).unwrap_or(raw).trim();
        if code.is_empty() || code.starts_with("#[test]") || code.contains("fun ") { continue; }
        if code.starts_with("init") && code.contains('{') {
            in_init = true;
            for value in env.values_mut() {
                if let Value::Object { storage: Storage::Local, in_init, .. } = value { *in_init = true; }
            }
            continue;
        }
        if code == "}" {
            in_init = false;
            for value in env.values_mut() {
                if let Value::Object { storage: Storage::Local, in_init, .. } = value { *in_init = false; }
            }
            continue;
        }

        let declaration = code.trim_end_matches(';');
        let (storage, rest) = if let Some(rest) = declaration.strip_prefix("now into ") { (Storage::Now, rest) }
            else if let Some(rest) = declaration.strip_prefix("late into ") { (Storage::Late, rest) }
            else if let Some(rest) = declaration.strip_prefix("into ") { (Storage::Local, rest) }
            else { (Storage::Local, "") };
        if !rest.is_empty() {
            if let Some((name, _)) = rest.split_once('=') {
                let name = name.split(':').next().unwrap_or(name).trim();
                let allocated = matches!(storage, Storage::Now);
                env.insert(name.to_owned(), Value::Object { storage, allocated, in_init });
            }
            continue;
        }
        if let Some(rest) = code.strip_prefix("unload ") {
            let name = rest.trim_end_matches(';').trim();
            if let Some(Value::Object { allocated, .. }) = env.get_mut(name) { *allocated = false; }
            continue;
        }
        if code.starts_with("assert(") {
            let expr = code.strip_prefix("assert(").unwrap()
                .strip_suffix(");").or_else(|| code.strip_prefix("assert(").unwrap().strip_suffix(')'))
                .unwrap_or("").trim();
            let value = eval(expr, &mut env, functions, *line_no, path, test)?;
            call_function("assert", vec![value], functions, *line_no, path, test)?;
        }
    }
    Ok(())
}

fn eval(expr: &str, env: &mut HashMap<String, Value>, functions: &HashMap<String, Function>, line: usize, path: &Path, test: &str) -> Result<Value> {
    let expr = expr.trim();
    if let Some(rest) = expr.strip_prefix('!') {
        return match eval(rest, env, functions, line, path, test)? {
            Value::Bool(value) => Ok(Value::Bool(!value)),
            _ => bail!("{}:{line}: `!` expects Boolean in {test}", path.display()),
        };
    }
    if expr == "true" { return Ok(Value::Bool(true)); }
    if expr == "false" { return Ok(Value::Bool(false)); }
    if let Some((function, args)) = expr.split_once('(').and_then(|(f, tail)| tail.strip_suffix(')').map(|args| (f.trim(), args))) {
        let arguments = if args.trim().is_empty() { Vec::new() }
            else { args.split(',').map(|arg| eval(arg, env, functions, line, path, test)).collect::<Result<Vec<_>>>()? };
        return call_function(function, arguments, functions, line, path, test);
    }
    if let Some((object_name, property)) = expr.split_once('.') {
        if property == "isInMemory" {
            let value = env.get_mut(object_name.trim()).ok_or_else(|| anyhow!("{}:{line}: undefined variable `{object_name}` in {test}", path.display()))?;
            if let Value::Object { storage, allocated, in_init } = value {
                let exists = match storage {
                    Storage::Now => *allocated,
                    Storage::Late => { *allocated = true; true },
                    Storage::Local => *in_init,
                };
                return Ok(Value::Bool(exists));
            }
        }
    }
    env.get(expr).cloned().ok_or_else(|| anyhow!("{}:{line}: undefined variable `{expr}` in {test}", path.display()))
}

fn call_function(name: &str, args: Vec<Value>, functions: &HashMap<String, Function>, line: usize, path: &Path, test: &str) -> Result<Value> {
    let function = functions.get(name).ok_or_else(|| anyhow!("{}:{line}: function `{name}` has no Into definition", path.display()))?;
    if function.params.len() != args.len() { bail!("{}:{line}: wrong argument count for {name}", path.display()); }
    let mut env: HashMap<_, _> = function.params.iter().cloned().zip(args).collect();
    let mut active_if = Vec::new();
    for statement in &function.body {
        let statement = statement.trim();
        if statement.is_empty() { continue; }
        if let Some(condition) = statement.strip_prefix("if ").and_then(|s| s.strip_suffix('{')) {
            let condition = condition.trim();
            let value = if let Some((lhs, rhs)) = condition.split_once("==") {
                match (eval(lhs, &mut env, functions, line, path, test)?, eval(rhs, &mut env, functions, line, path, test)?) {
                    (Value::Bool(a), Value::Bool(b)) => a == b,
                    _ => bail!("{}:{line}: invalid Boolean condition in {name}", path.display()),
                }
            } else {
                match eval(condition, &mut env, functions, line, path, test)? {
                    Value::Bool(value) => value,
                    _ => bail!("{}:{line}: if condition must be Boolean in {name}", path.display()),
                }
            };
            active_if.push(value);
            continue;
        }
        if statement == "}" { active_if.pop(); continue; }
        let active = active_if.iter().all(|value| *value);
        if !active { continue; }
        if let Some(value) = statement.strip_prefix("throw ") {
            bail!("{}:{line}: Into assertion failed in {test}: {}", path.display(), value.trim_end_matches(';').trim_matches('"'));
        }
        if let Some(expr) = statement.strip_prefix("return ") {
            return eval(expr.trim_end_matches(';'), &mut env, functions, line, path, test);
        }
    }
    Ok(Value::Bool(true))
}
