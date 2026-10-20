use errors::Result;
use types::{Keyword, Punct};

use super::block::{Scope, parse_block};
use super::declaration::Declaration;
use crate::decl::FunctionDecl;
use crate::{Cursor, Expr, Path, SyntaxError};

#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    pub line: usize,
    pub kind: StmtKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    Declaration(Declaration),
    Assign {
        target: Path,
        value: Expr,
    },
    Unload(String),
    If {
        condition: Expr,
        then_body: Vec<Stmt>,
        else_body: Option<Vec<Stmt>>,
    },
    While {
        condition: Expr,
        body: Vec<Stmt>,
    },
    Break,
    Return(Option<Expr>),
    Throw(Expr),
    Init(Vec<Stmt>),
    Expr(Expr),
}

impl Stmt {
    pub(crate) fn parse(cursor: &mut Cursor<'_>, scope: Scope) -> Result<Stmt> {
        let line = cursor.line();
        if cursor.is_punct(Punct::RightBrace) {
            return Err(cursor.error("Unexpected closing brace"));
        }
        if FunctionDecl::ahead(cursor) {
            return Err(match scope {
                Scope::Function => SyntaxError::new(
                    line,
                    "Function declarations cannot be nested inside another function",
                )
                .in_scope("function body"),
                _ => SyntaxError::new(line, "Function declarations must be at module level"),
            }
            .into());
        }
        let kind = match cursor.peek_keyword(0) {
            Some(Keyword::If) => parse_if(cursor, scope)?,
            Some(Keyword::Else) => {
                return Err(cursor.error("`else` must follow an `if` block"));
            }
            Some(Keyword::While) => {
                cursor.bump();
                let condition = Expr::try_from(&mut *cursor)?;
                let body = parse_block(cursor, inner_scope(scope))?;
                StmtKind::While { condition, body }
            }
            Some(Keyword::Init) if cursor.peek_punct(1) == Some(Punct::LeftBrace) => {
                cursor.bump();
                StmtKind::Init(parse_block(cursor, Scope::Block)?)
            }
            Some(Keyword::Break) => {
                cursor.bump();
                cursor.end_statement()?;
                StmtKind::Break
            }
            Some(Keyword::Return) => {
                cursor.bump();
                let value = if ends_statement(cursor) {
                    None
                } else {
                    Some(Expr::try_from(&mut *cursor)?)
                };
                cursor.end_statement()?;
                StmtKind::Return(value)
            }
            Some(Keyword::Throw) => {
                cursor.bump();
                let value = Expr::try_from(&mut *cursor)?;
                cursor.end_statement()?;
                StmtKind::Throw(value)
            }
            Some(Keyword::Unload) => {
                cursor.bump();
                let (name, _) = cursor.expect_ident("a variable name after `unload`")?;
                cursor.end_statement()?;
                StmtKind::Unload(name)
            }
            _ if Declaration::ahead(cursor) => {
                StmtKind::Declaration(Declaration::try_from(&mut *cursor)?)
            }
            _ => parse_expression_statement(cursor)?,
        };
        Ok(Stmt { line, kind })
    }
}

impl Stmt {
    /// The names `body` declares, in order, with those of the statements
    /// nested in branches and loops.
    pub fn declared_names(body: &[Stmt]) -> Vec<String> {
        let mut names = Vec::new();
        for stmt in body {
            match &stmt.kind {
                StmtKind::Declaration(declaration) => names.push(declaration.name.clone()),
                StmtKind::If {
                    then_body,
                    else_body,
                    ..
                } => {
                    names.extend(Stmt::declared_names(then_body));
                    names.extend(Stmt::declared_names(
                        else_body.as_deref().unwrap_or_default(),
                    ));
                }
                StmtKind::While { body, .. } | StmtKind::Init(body) => {
                    names.extend(Stmt::declared_names(body));
                }
                _ => {}
            }
        }
        names
    }

    /// Give the declarations in the statement, and in the statements nested in
    /// it, the names `rename` chooses; names it does not know stay.
    pub fn rename_declared(&self, rename: &dyn Fn(&str) -> Option<String>) -> Stmt {
        let nested = |stmts: &[Stmt]| -> Vec<Stmt> {
            stmts
                .iter()
                .map(|stmt| stmt.rename_declared(rename))
                .collect()
        };
        let kind = match &self.kind {
            StmtKind::Declaration(declaration) => StmtKind::Declaration(Declaration {
                name: rename(&declaration.name).unwrap_or_else(|| declaration.name.clone()),
                ..declaration.clone()
            }),
            StmtKind::If {
                condition,
                then_body,
                else_body,
            } => StmtKind::If {
                condition: condition.clone(),
                then_body: nested(then_body),
                else_body: else_body.as_deref().map(nested),
            },
            StmtKind::While { condition, body } => StmtKind::While {
                condition: condition.clone(),
                body: nested(body),
            },
            StmtKind::Init(body) => StmtKind::Init(nested(body)),
            other => other.clone(),
        };
        Stmt {
            line: self.line,
            kind,
        }
    }

    /// Replace bound names by expressions throughout the statement, as when
    /// a function body is inlined at a call site. Declared names are kept.
    pub fn substitute(&self, bindings: &[(String, Expr)]) -> Stmt {
        let body = |stmts: &[Stmt]| -> Vec<Stmt> {
            stmts.iter().map(|stmt| stmt.substitute(bindings)).collect()
        };
        let kind = match &self.kind {
            StmtKind::Declaration(declaration) => StmtKind::Declaration(Declaration {
                value: declaration
                    .value
                    .as_ref()
                    .map(|value| value.substitute(bindings)),
                ..declaration.clone()
            }),
            StmtKind::Assign { target, value } => StmtKind::Assign {
                target: match Expr::Path(target.clone()).substitute(bindings) {
                    Expr::Path(path) => path,
                    _ => target.clone(),
                },
                value: value.substitute(bindings),
            },
            StmtKind::Unload(name) => StmtKind::Unload(
                match Expr::ident(name.clone()).substitute(bindings).as_ident() {
                    Some(replaced) => replaced.to_owned(),
                    None => name.clone(),
                },
            ),
            StmtKind::If {
                condition,
                then_body,
                else_body,
            } => StmtKind::If {
                condition: condition.substitute(bindings),
                then_body: body(then_body),
                else_body: else_body.as_deref().map(body),
            },
            StmtKind::While {
                condition,
                body: while_body,
            } => StmtKind::While {
                condition: condition.substitute(bindings),
                body: body(while_body),
            },
            StmtKind::Break => StmtKind::Break,
            StmtKind::Return(value) => {
                StmtKind::Return(value.as_ref().map(|value| value.substitute(bindings)))
            }
            StmtKind::Throw(value) => StmtKind::Throw(value.substitute(bindings)),
            StmtKind::Init(init_body) => StmtKind::Init(body(init_body)),
            StmtKind::Expr(expression) => StmtKind::Expr(expression.substitute(bindings)),
        };
        Stmt {
            line: self.line,
            kind,
        }
    }
}

impl TryFrom<&mut Cursor<'_>> for Stmt {
    type Error = errors::Error;

    fn try_from(cursor: &mut Cursor<'_>) -> Result<Self> {
        Stmt::parse(cursor, Scope::Module)
    }
}

fn inner_scope(scope: Scope) -> Scope {
    match scope {
        Scope::Function => Scope::Function,
        _ => Scope::Block,
    }
}

fn ends_statement(cursor: &Cursor<'_>) -> bool {
    cursor.at_end()
        || cursor.is_punct(Punct::Semicolon)
        || cursor.is_punct(Punct::RightBrace)
}

fn parse_if(cursor: &mut Cursor<'_>, scope: Scope) -> Result<StmtKind> {
    cursor.bump();
    let condition = Expr::try_from(&mut *cursor)?;
    let then_body = parse_block(cursor, inner_scope(scope))?;
    let else_body = if cursor.eat_keyword(Keyword::Else) {
        if cursor.is_keyword(Keyword::If) {
            let line = cursor.line();
            let kind = parse_if(cursor, scope)?;
            Some(vec![Stmt { line, kind }])
        } else {
            Some(parse_block(cursor, inner_scope(scope))?)
        }
    } else {
        None
    };
    Ok(StmtKind::If {
        condition,
        then_body,
        else_body,
    })
}

fn parse_expression_statement(cursor: &mut Cursor<'_>) -> Result<StmtKind> {
    if cursor.peek_word(0).is_some() {
        let start = cursor.position();
        if let Ok(target) = Path::try_from(&mut *cursor) {
            let assigns = cursor.is_punct(Punct::Assign)
                && !(cursor.adjacent(1) && cursor.peek_punct(1) == Some(Punct::Assign));
            if assigns {
                cursor.bump();
                let value = Expr::try_from(&mut *cursor)?;
                cursor.end_statement()?;
                return Ok(StmtKind::Assign { target, value });
            }
        }
        cursor.rewind(start);
    }
    let expression = Expr::try_from(&mut *cursor)?;
    cursor.end_statement()?;
    Ok(StmtKind::Expr(expression))
}
