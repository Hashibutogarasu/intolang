use ast::{Expr, Stmt, StmtKind};
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
        Expr::Binary { left, right, .. } | Expr::Compare { left, right, .. } => {
            collect_expression(left, names);
            collect_expression(right, names);
        }
        Expr::Not(inner) => collect_expression(inner, names),
    }
}
