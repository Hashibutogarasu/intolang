use ast::{EventDecl, FunctionDecl, Program};
use errors::{Error, Errors, Result};
use std::collections::HashMap;

use crate::context::{ClassInfo, Classes, EVENT, LISTENER, MemberInfo, OBJECT, TASK, THREAD};
use crate::diagnostic::diagnostic;

/// The return type that stands for "the class of the receiver".
pub const SELF_TYPE: &str = "Self";

/// The built-in root class every class without a parent inherits from.
pub fn builtin_classes() -> Classes {
    let default = |name: &str, return_type: &str| MemberInfo {
        name: name.to_owned(),
        line: 0,
        is_static: false,
        is_abstract: false,
        is_default: true,
        is_override: false,
        type_params: Vec::new(),
        params: Vec::new(),
        return_type: Some(return_type.to_owned()),
        has_body: false,
        is_native: true,
    };
    let on_cast = MemberInfo {
        name: "onCast".to_owned(),
        line: 0,
        is_static: false,
        is_abstract: false,
        is_default: true,
        is_override: false,
        type_params: vec!["From".to_owned(), "To".to_owned()],
        params: vec!["From".to_owned(), "To".to_owned()],
        return_type: None,
        has_body: true,
        is_native: false,
    };
    let object = ClassInfo {
        name: OBJECT.to_owned(),
        line: 0,
        is_abstract: true,
        type_params: Vec::new(),
        parent: None,
        parent_type: None,
        methods: vec![
            default("toString", "String"),
            default("hash", "Int32"),
            default("destroy", "Void"),
            default("copy", SELF_TYPE),
            default("move", SELF_TYPE),
            default("borrow", SELF_TYPE),
        ],
        events: vec![on_cast],
    };
    let plain = |name: &str| ClassInfo {
        name: name.to_owned(),
        line: 0,
        is_abstract: false,
        type_params: Vec::new(),
        parent: None,
        parent_type: None,
        methods: Vec::new(),
        events: Vec::new(),
    };
    Classes::from([
        (OBJECT.to_owned(), object),
        (EVENT.to_owned(), plain(EVENT)),
        (LISTENER.to_owned(), plain(LISTENER)),
        (THREAD.to_owned(), plain(THREAD)),
        (TASK.to_owned(), plain(TASK)),
    ])
}

/// Collect the inheritance view of every class declared in `program`.
pub fn collect_class_infos(program: &Program) -> Vec<ClassInfo> {
    program
        .classes()
        .map(|class| ClassInfo {
            name: class.name.clone(),
            line: class.line,
            is_abstract: class.is_abstract,
            type_params: class.type_params.clone(),
            parent: class.parent.clone(),
            parent_type: class.parent_type.as_ref().map(ToString::to_string),
            methods: class.methods.iter().map(method_info).collect(),
            events: class.events.iter().map(event_info).collect(),
        })
        .collect()
}

fn method_info(function: &FunctionDecl) -> MemberInfo {
    MemberInfo {
        name: function.name.clone(),
        line: function.line,
        is_static: function.is_static,
        is_abstract: function.is_abstract,
        is_default: function.is_default,
        is_override: function.is_override,
        type_params: function.type_params.clone(),
        params: function
            .params
            .iter()
            .map(|param| {
                param
                    .ty
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_default()
            })
            .collect(),
        return_type: function.return_type.as_ref().map(ToString::to_string),
        has_body: function.body.is_some(),
        is_native: function.language.is_some(),
    }
}

fn event_info(event: &EventDecl) -> MemberInfo {
    MemberInfo {
        name: event.name.clone(),
        line: event.line,
        is_static: event.is_static,
        is_abstract: false,
        is_default: event.is_default,
        is_override: event.is_override,
        type_params: event.type_params.clone(),
        params: event
            .params
            .iter()
            .map(|param| {
                param
                    .ty
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_default()
            })
            .collect(),
        return_type: None,
        has_body: true,
        is_native: false,
    }
}

/// The parent chain of `name`, nearest first, ending with `Object`.
pub fn ancestors<'a>(classes: &'a Classes, name: &str) -> Result<Vec<&'a ClassInfo>> {
    let mut chain: Vec<&ClassInfo> = Vec::new();
    let mut current = name.to_owned();
    loop {
        let class = classes.get(&current).ok_or_else(|| Errors::UnknownClass {
            name: current.clone(),
        })?;
        if chain.iter().any(|seen| seen.name == class.name) {
            return Err(Errors::ClassInheritsItself {
                name: name.to_string(),
            }
            .into());
        }
        let parent = class
            .parent
            .clone()
            .or_else(|| (class.name != OBJECT).then(|| OBJECT.to_owned()));
        chain.push(class);
        match parent {
            Some(parent) => current = parent,
            None => break,
        }
    }
    chain.remove(0);
    Ok(chain)
}

/// Check the classes of one module against the whole class table: parent
/// existence, abstract and `override` rules.
pub fn check_inheritance(classes: &Classes, declared: &[ClassInfo], path: &str) -> Result<()> {
    for class in declared {
        let fail = |line: usize, error: Error| {
            diagnostic(path, line, &format!("class `{}`", class.name), error)
        };
        if class.name == OBJECT {
            return Err(fail(class.line, Errors::ObjectIsBuiltin.into()));
        }
        let chain = ancestors(classes, &class.name).map_err(|error| fail(class.line, error))?;
        check_members(class, &chain, MemberKind::Method)
            .map_err(|(line, error)| fail(line, error))?;
        check_members(class, &chain, MemberKind::Event)
            .map_err(|(line, error)| fail(line, error))?;
        if !class.is_abstract {
            check_implemented(class, &chain).map_err(|error| fail(class.line, error))?;
        }
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MemberKind {
    Method,
    Event,
}

impl MemberKind {
    fn members(self, class: &ClassInfo) -> &[MemberInfo] {
        match self {
            MemberKind::Method => &class.methods,
            MemberKind::Event => &class.events,
        }
    }

    fn noun(self) -> &'static str {
        match self {
            MemberKind::Method => "member",
            MemberKind::Event => "event",
        }
    }
}

fn check_members(
    class: &ClassInfo,
    chain: &[&ClassInfo],
    kind: MemberKind,
) -> std::result::Result<(), (usize, Error)> {
    let members = kind.members(class);
    for (index, member) in members.iter().enumerate() {
        let name = &member.name;
        let fail = |error: Errors| (member.line, Error::from(error));
        if members[..index].iter().any(|other| {
            other.name == *name
                && other.type_params == member.type_params
                && other.params == member.params
        }) {
            return Err(fail(Errors::DuplicateMember {
                noun: kind.noun().to_string(),
                name: name.clone(),
            }));
        }
        if member.is_abstract && !class.is_abstract {
            return Err(fail(Errors::AbstractMemberOutsideAbstractClass {
                name: name.clone(),
            }));
        }
        if member.is_abstract && (member.has_body || member.is_native) {
            return Err(fail(Errors::AbstractMemberHasBody { name: name.clone() }));
        }
        let inherited = chain.iter().find_map(|ancestor| {
            kind.members(ancestor)
                .iter()
                .find(|candidate| candidate.name == *name)
                .map(|candidate| (*ancestor, candidate))
        });
        match (inherited, member.is_override) {
            (None, true) => {
                return Err(fail(Errors::OverrideOverridesNothing {
                    name: name.clone(),
                }));
            }
            (Some((ancestor, parent)), false) => {
                let error = if parent.is_open() {
                    Errors::MemberHidesInherited {
                        name: name.clone(),
                        ancestor: ancestor.name.clone(),
                    }
                } else {
                    Errors::MemberFinalRedeclared {
                        name: name.clone(),
                        ancestor: ancestor.name.clone(),
                    }
                };
                return Err(fail(error));
            }
            (Some((ancestor, parent)), true) => {
                if !parent.is_open() {
                    return Err(fail(Errors::MemberFinalOverridden {
                        name: name.clone(),
                        ancestor: ancestor.name.clone(),
                    }));
                }
                if !signature_matches(kind, class, ancestor, parent, member) {
                    return Err(fail(Errors::SignatureMismatch {
                        name: name.clone(),
                        ancestor: ancestor.name.clone(),
                    }));
                }
            }
            (None, false) => {}
        }
    }
    Ok(())
}

/// Whether `member` of `class` may override `parent` of `ancestor`. A member
/// that returns the class declaring it may be overridden by one that returns
/// the overriding class.
fn signature_matches(
    kind: MemberKind,
    class: &ClassInfo,
    ancestor: &ClassInfo,
    parent: &MemberInfo,
    member: &MemberInfo,
) -> bool {
    if parent.params.len() != member.params.len() {
        return false;
    }
    if kind == MemberKind::Event {
        return true;
    }
    let returns = match (&parent.return_type, &member.return_type) {
        (Some(parent), Some(child)) if parent == SELF_TYPE || *parent == ancestor.name => {
            child == parent || child == SELF_TYPE || *child == class.name
        }
        (parent, child) => parent == child,
    };
    parent.params == member.params && returns
}

/// A non-abstract class must give every inherited abstract member a body.
fn check_implemented(class: &ClassInfo, chain: &[&ClassInfo]) -> Result<()> {
    for kind in [MemberKind::Method, MemberKind::Event] {
        let mut effective: HashMap<&str, bool> = HashMap::new();
        for owner in chain.iter().rev().copied().chain(std::iter::once(class)) {
            for member in kind.members(owner) {
                effective.insert(member.name.as_str(), member.is_abstract);
            }
        }
        let mut missing: Vec<&str> = effective
            .into_iter()
            .filter_map(|(name, is_abstract)| is_abstract.then_some(name))
            .collect();
        missing.sort_unstable();
        if let Some(name) = missing.first() {
            return Err(Errors::AbstractMemberNotImplemented {
                class: class.name.clone(),
                name: (*name).to_string(),
            }
            .into());
        }
    }
    Ok(())
}
