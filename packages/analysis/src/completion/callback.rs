use ipc::{CompletionItem, CompletionKind, SymbolKind};

use super::context::{CompletionContext, Site};
use super::provider::CompletionProvider;
use crate::index::Scope;

const EVENT_HANDLER: &str = "on";
const SINGLE_CALLBACK: &str = "Callback";
const PAIR_CALLBACK: &str = "BiCallback";
const PAIR_NAMES: [&str; 2] = ["key", "value"];
const FALLBACK_NAME: &str = "value";

pub struct CallbackProvider;

impl CompletionProvider for CallbackProvider {
    fn candidates(&self, context: &CompletionContext<'_>) -> Vec<CompletionItem> {
        let (Site::Word { call: Some(call) }, Some(analysis)) = (&context.site, context.analysis)
        else {
            return Vec::new();
        };
        if !context.prefix.is_empty() {
            return Vec::new();
        }
        let scope = Scope::new(analysis);
        let Some(names) = handler_names(&scope, &call.callee, call.index, context.line) else {
            return Vec::new();
        };
        let names = distinct(names);
        let tab_stops: Vec<String> = names
            .iter()
            .enumerate()
            .map(|(number, name)| format!("${{{}:{name}}}", number + 1))
            .collect();
        vec![CompletionItem {
            text: format!("({}) => {{\n\t$0\n}}", tab_stops.join(", ")),
            snippet: true,
            detail: Some("Callback".to_owned()),
            ..context.item(
                format!("({}) => {{ }}", names.join(", ")),
                CompletionKind::Snippet,
            )
        }]
    }
}

/// The parameter names of the callback the argument `index` of `callee` takes.
fn handler_names(
    scope: &Scope<'_>,
    callee: &[String],
    index: usize,
    line: usize,
) -> Option<Vec<String>> {
    let (method, receiver) = callee.split_last()?;
    if method == EVENT_HANDLER
        && index == 0
        && !receiver.is_empty()
        && let Some(names) = event_names(scope, receiver, line)
    {
        return Some(names);
    }
    let symbol = if receiver.is_empty() {
        scope.named(method, line)?.1
    } else {
        let class = scope.type_of_path(receiver, line)?;
        scope.member_of(&class, method)?.1
    };
    callback_names(symbol.params.get(index)?.ty.as_deref()?)
}

/// The parameter names of the event that `receiver` names.
fn event_names(scope: &Scope<'_>, receiver: &[String], line: usize) -> Option<Vec<String>> {
    let (name, owner) = receiver.split_last()?;
    let symbol = if owner.is_empty() {
        scope.named(name, line)?.1
    } else {
        let class = scope.type_of_path(owner, line)?;
        scope.member_of(&class, name)?.1
    };
    (symbol.kind == SymbolKind::Event).then(|| {
        symbol
            .params
            .iter()
            .map(|param| param.name.clone())
            .collect()
    })
}

/// The names a callback of the declared type `ty` takes, from its arguments.
fn callback_names(ty: &str) -> Option<Vec<String>> {
    let (name, arguments) = match ty.split_once('<') {
        Some((name, rest)) => (name.trim(), rest.trim_end().trim_end_matches('>')),
        None => (ty.trim(), ""),
    };
    match name {
        PAIR_CALLBACK => Some(PAIR_NAMES.map(str::to_owned).to_vec()),
        SINGLE_CALLBACK => Some(vec![argument_name(arguments)]),
        _ => None,
    }
}

/// The name for a value of the type `argument`: its class in lower camel case.
fn argument_name(argument: &str) -> String {
    let class = argument
        .split('<')
        .next()
        .unwrap_or_default()
        .trim()
        .trim_end_matches(['&', '*']);
    let mut characters = class.chars();
    match characters.next() {
        Some(first) => first.to_lowercase().chain(characters).collect(),
        None => FALLBACK_NAME.to_owned(),
    }
}

/// `names` with a number added to the repeated ones.
fn distinct(names: Vec<String>) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    for name in names {
        let mut candidate = name.clone();
        let mut number = 1;
        while seen.contains(&candidate) {
            number += 1;
            candidate = format!("{name}{number}");
        }
        seen.push(candidate);
    }
    seen
}
