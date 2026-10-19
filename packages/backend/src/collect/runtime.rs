use ast::{Expr, Path, Stmt, StmtKind};
use native::NativeLink;
use std::collections::{HashMap, HashSet};

use crate::context::UserFunction;
use crate::expr::{base_type_name, called_paths, names_callback, returns_early};

/// The functions that are called at run time instead of being inlined: the
/// ones that can reach themselves through calls, and the ones that return
/// before their last statement.
pub fn runtime_functions(
    functions: &HashMap<String, UserFunction>,
    links: &HashMap<String, NativeLink>,
) -> HashSet<String> {
    let edges: HashMap<&str, HashSet<String>> = functions
        .iter()
        .map(|(key, function)| {
            let locals = local_classes(function, functions, links);
            let targets = called_paths(&function.body)
                .iter()
                .flat_map(|path| callee_keys(functions, links, function, &locals, path))
                .collect();
            (key.as_str(), targets)
        })
        .collect();
    functions
        .iter()
        .filter(|(key, function)| {
            takes_callback(function)
                || returns_early(&function.body)
                || reaches(&edges, key.as_str(), key.as_str())
        })
        .map(|(key, _)| key.clone())
        .collect()
}

/// Whether a parameter of the function is a function, so that a lambda can be
/// passed to it.
fn takes_callback(function: &UserFunction) -> bool {
    function
        .params
        .iter()
        .filter_map(|param| param.ty.as_ref())
        .any(|ty| names_callback(ty.as_str()))
}

/// The class each name in `function` holds an instance of, as far as the
/// text of the function tells: its parameters, and the bindings declared with
/// a type or with a value that is a construction or a call with a known result.
fn local_classes(
    function: &UserFunction,
    functions: &HashMap<String, UserFunction>,
    links: &HashMap<String, NativeLink>,
) -> HashMap<String, String> {
    let mut locals = HashMap::new();
    for param in &function.params {
        if let Some(ty) = &param.ty {
            locals.insert(param.name.clone(), base_type_name(ty.as_str()).to_owned());
        }
    }
    declared_classes(&function.body, functions, links, &mut locals);
    locals
}

fn declared_classes(
    body: &[Stmt],
    functions: &HashMap<String, UserFunction>,
    links: &HashMap<String, NativeLink>,
    locals: &mut HashMap<String, String>,
) {
    for stmt in body {
        match &stmt.kind {
            StmtKind::Declaration(declaration) => {
                let class = declaration
                    .annotation
                    .as_ref()
                    .map(|ty| base_type_name(ty.as_str()).to_owned())
                    .or_else(|| {
                        declaration
                            .value
                            .as_ref()
                            .and_then(|value| result_class(value, functions, links))
                    });
                if let Some(class) = class {
                    locals.insert(declaration.name.clone(), class);
                }
            }
            StmtKind::If {
                then_body,
                else_body,
                ..
            } => {
                declared_classes(then_body, functions, links, locals);
                declared_classes(
                    else_body.as_deref().unwrap_or_default(),
                    functions,
                    links,
                    locals,
                );
            }
            StmtKind::While { body, .. } | StmtKind::Init(body) => {
                declared_classes(body, functions, links, locals);
            }
            _ => {}
        }
    }
}

/// The class a call evaluates to: the class a construction builds, or the
/// result type a function or a native function declares.
fn result_class(
    value: &Expr,
    functions: &HashMap<String, UserFunction>,
    links: &HashMap<String, NativeLink>,
) -> Option<String> {
    let Expr::Call { callee, .. } = value else {
        return None;
    };
    match callee.segments() {
        [name] => functions
            .get(name)
            .and_then(|function| function.return_type.clone())
            .map(|ty| base_type_name(&ty).to_owned())
            .or_else(|| {
                name.starts_with(|first: char| first.is_uppercase())
                    .then(|| name.clone())
            }),
        [class, method] => {
            let key = format!("{class}.{method}");
            functions
                .get(&key)
                .and_then(|function| function.return_type.clone())
                .or_else(|| links.get(&key).map(|link| link.return_type.clone()))
                .map(|ty| base_type_name(&ty).to_owned())
        }
        _ => None,
    }
}

/// The functions a call from `caller` may run.
fn callee_keys(
    functions: &HashMap<String, UserFunction>,
    links: &HashMap<String, NativeLink>,
    caller: &UserFunction,
    locals: &HashMap<String, String>,
    path: &Path,
) -> Vec<String> {
    match path.segments() {
        [name] => functions
            .contains_key(name)
            .then(|| name.clone())
            .into_iter()
            .collect(),
        [receiver, method] => {
            let own = (receiver == "this")
                .then(|| caller.receiver.as_ref())
                .flatten()
                .map(|class| format!("{class}.{method}"));
            if let Some(key) = own.filter(|key| functions.contains_key(key)) {
                return vec![key];
            }
            let static_key = format!("{receiver}.{method}");
            if functions.contains_key(&static_key) {
                return vec![static_key];
            }
            if links.contains_key(&static_key) {
                return Vec::new();
            }
            if let Some(class) = locals.get(receiver) {
                let key = format!("{class}.{method}");
                if functions.contains_key(&key) {
                    return vec![key];
                }
                if links.contains_key(&key) {
                    return Vec::new();
                }
            }
            let suffix = format!(".{method}");
            functions
                .keys()
                .filter(|key| key.ends_with(&suffix))
                .cloned()
                .collect()
        }
        _ => Vec::new(),
    }
}

/// Whether `to` can be reached from `from` through at least one call.
fn reaches(edges: &HashMap<&str, HashSet<String>>, from: &str, to: &str) -> bool {
    let mut pending: Vec<&str> = edges
        .get(from)
        .into_iter()
        .flatten()
        .map(String::as_str)
        .collect();
    let mut seen: HashSet<&str> = HashSet::new();
    while let Some(next) = pending.pop() {
        if next == to {
            return true;
        }
        if !seen.insert(next) {
            continue;
        }
        pending.extend(edges.get(next).into_iter().flatten().map(String::as_str));
    }
    false
}
