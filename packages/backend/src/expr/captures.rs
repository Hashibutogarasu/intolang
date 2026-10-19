use ast::{Expr, Path, Stmt, StmtKind};
use std::collections::BTreeSet;

/// The first segment of every path a body mentions: the names it may capture
/// from the scopes around it.
pub fn mentioned_names(body: &[Stmt]) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for stmt in body {
        collect_statement(stmt, &mut names);
    }
    names
}

/// The paths a body calls, nested branches, loops and lambdas included.
pub fn called_paths(body: &[Stmt]) -> Vec<Path> {
    let mut calls = Vec::new();
    for stmt in body {
        collect_calls_in_statement(stmt, &mut calls);
    }
    calls
}

fn collect_calls_in_statement(stmt: &Stmt, calls: &mut Vec<Path>) {
    match &stmt.kind {
        StmtKind::Declaration(declaration) => {
            if let Some(value) = &declaration.value {
                collect_calls_in_expression(value, calls);
            }
        }
        StmtKind::Assign { value, .. } => collect_calls_in_expression(value, calls),
        StmtKind::If {
            condition,
            then_body,
            else_body,
        } => {
            collect_calls_in_expression(condition, calls);
            then_body
                .iter()
                .chain(else_body.iter().flatten())
                .for_each(|stmt| collect_calls_in_statement(stmt, calls));
        }
        StmtKind::While { condition, body } => {
            collect_calls_in_expression(condition, calls);
            body.iter()
                .for_each(|stmt| collect_calls_in_statement(stmt, calls));
        }
        StmtKind::Init(body) => body
            .iter()
            .for_each(|stmt| collect_calls_in_statement(stmt, calls)),
        StmtKind::Return(Some(value)) | StmtKind::Throw(value) | StmtKind::Expr(value) => {
            collect_calls_in_expression(value, calls)
        }
        StmtKind::Return(None) | StmtKind::Unload(_) | StmtKind::Break => {}
    }
}

fn collect_calls_in_expression(expression: &Expr, calls: &mut Vec<Path>) {
    match expression {
        Expr::Lambda { body, .. } | Expr::Thread { body } => body
            .iter()
            .for_each(|stmt| collect_calls_in_statement(stmt, calls)),
        Expr::Await(inner) | Expr::Not(inner) | Expr::Cast { value: inner, .. } => {
            collect_calls_in_expression(inner, calls)
        }
        Expr::Array(items) => items
            .iter()
            .for_each(|item| collect_calls_in_expression(item, calls)),
        Expr::Object(entries) => entries
            .iter()
            .for_each(|(_, value)| collect_calls_in_expression(value, calls)),
        Expr::Literal(_) | Expr::Path(_) => {}
        Expr::Call { callee, args } => {
            calls.push(callee.clone());
            args.iter()
                .for_each(|arg| collect_calls_in_expression(arg, calls));
        }
        Expr::Binary { left, right, .. }
        | Expr::Logical { left, right, .. }
        | Expr::Compare { left, right, .. } => {
            collect_calls_in_expression(left, calls);
            collect_calls_in_expression(right, calls);
        }
    }
}

/// Whether a body returns before its last statement ends it: a `return`
/// inside a branch or a loop, or one that is followed by other statements.
pub fn returns_early(body: &[Stmt]) -> bool {
    fn nested(stmt: &Stmt) -> bool {
        match &stmt.kind {
            StmtKind::Return(_) => true,
            StmtKind::If {
                then_body,
                else_body,
                ..
            } => then_body
                .iter()
                .chain(else_body.iter().flatten())
                .any(nested),
            StmtKind::While { body, .. } => body.iter().any(nested),
            _ => false,
        }
    }
    match body.split_last() {
        None => false,
        Some((last, rest)) => {
            rest.iter().any(nested) || (!matches!(last.kind, StmtKind::Return(_)) && nested(last))
        }
    }
}

/// The names a body assigns to as a whole, nested branches and loops included.
pub fn assigned_names(body: &[Stmt]) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for stmt in body {
        collect_assigned(stmt, &mut names);
    }
    names
}

fn collect_assigned(stmt: &Stmt, names: &mut BTreeSet<String>) {
    match &stmt.kind {
        StmtKind::Assign { target, .. } => {
            if let Some(name) = target.as_ident() {
                names.insert(name.to_owned());
            }
        }
        StmtKind::If {
            then_body,
            else_body,
            ..
        } => then_body
            .iter()
            .chain(else_body.iter().flatten())
            .for_each(|stmt| collect_assigned(stmt, names)),
        StmtKind::While { body, .. } | StmtKind::Init(body) => {
            body.iter().for_each(|stmt| collect_assigned(stmt, names))
        }
        _ => {}
    }
}

fn collect_statement(stmt: &Stmt, names: &mut BTreeSet<String>) {
    match &stmt.kind {
        StmtKind::Declaration(declaration) => {
            if let Some(value) = &declaration.value {
                collect_expression(value, names);
            }
        }
        StmtKind::Assign { target, value } => {
            names.insert(target.first().to_owned());
            collect_expression(value, names);
        }
        StmtKind::Unload(name) => {
            names.insert(name.clone());
        }
        StmtKind::If {
            condition,
            then_body,
            else_body,
        } => {
            collect_expression(condition, names);
            then_body
                .iter()
                .chain(else_body.iter().flatten())
                .for_each(|stmt| collect_statement(stmt, names));
        }
        StmtKind::While { condition, body } => {
            collect_expression(condition, names);
            body.iter().for_each(|stmt| collect_statement(stmt, names));
        }
        StmtKind::Init(body) => body.iter().for_each(|stmt| collect_statement(stmt, names)),
        StmtKind::Return(value) => {
            if let Some(value) = value {
                collect_expression(value, names);
            }
        }
        StmtKind::Throw(value) | StmtKind::Expr(value) => collect_expression(value, names),
        StmtKind::Break => {}
    }
}

fn collect_expression(expression: &Expr, names: &mut BTreeSet<String>) {
    match expression {
        Expr::Lambda { body, .. } | Expr::Thread { body } => {
            body.iter().for_each(|stmt| collect_statement(stmt, names))
        }
        Expr::Await(inner) => collect_expression(inner, names),
        Expr::Literal(_) => {}
        Expr::Path(path) => {
            names.insert(path.first().to_owned());
        }
        Expr::Call { callee, args } => {
            names.insert(callee.first().to_owned());
            args.iter().for_each(|arg| collect_expression(arg, names));
        }
        Expr::Binary { left, right, .. }
        | Expr::Logical { left, right, .. }
        | Expr::Compare { left, right, .. } => {
            collect_expression(left, names);
            collect_expression(right, names);
        }
        Expr::Not(inner) | Expr::Cast { value: inner, .. } => collect_expression(inner, names),
        Expr::Array(items) => items
            .iter()
            .for_each(|item| collect_expression(item, names)),
        Expr::Object(entries) => entries
            .iter()
            .for_each(|(_, value)| collect_expression(value, names)),
    }
}
