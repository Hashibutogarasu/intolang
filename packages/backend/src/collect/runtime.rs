use ast::Path;
use native::NativeLink;
use std::collections::{HashMap, HashSet};

use crate::context::UserFunction;
use crate::expr::{called_paths, names_callback, returns_early};

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
            let targets = called_paths(&function.body)
                .iter()
                .flat_map(|path| callee_keys(functions, links, function, path))
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

/// The functions a call from `caller` may run.
fn callee_keys(
    functions: &HashMap<String, UserFunction>,
    links: &HashMap<String, NativeLink>,
    caller: &UserFunction,
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
