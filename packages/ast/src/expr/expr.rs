use anyhow::Result;
use parser::{ArithmeticLexeme, ComparisonLexeme};
use std::fmt;
use std::str::FromStr;
use types::{ArithmeticOperator, ComparisonOperator, Literal, Punct, TokenKind};

use super::path::Path;
use crate::stmt::{Scope, parse_block};
use crate::{Cursor, Param, Stmt};

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// `(params) => { body }`: an anonymous function.
    Lambda {
        params: Vec<Param>,
        body: Vec<Stmt>,
    },
    Literal(Literal),
    Path(Path),
    Call {
        callee: Path,
        args: Vec<Expr>,
    },
    Binary {
        left: Box<Expr>,
        op: ArithmeticOperator,
        right: Box<Expr>,
    },
    Compare {
        left: Box<Expr>,
        op: ComparisonOperator,
        right: Box<Expr>,
    },
    Not(Box<Expr>),
}

impl Expr {
    pub fn ident(name: impl Into<String>) -> Self {
        Expr::Path(Path::single(name))
    }

    pub fn int(value: i64) -> Self {
        Expr::Literal(Literal::Int(value))
    }

    pub fn as_path(&self) -> Option<&Path> {
        match self {
            Expr::Path(path) => Some(path),
            _ => None,
        }
    }

    pub fn as_ident(&self) -> Option<&str> {
        self.as_path().and_then(Path::as_ident)
    }

    /// Replace names by expressions, as when a call is inlined: a bound name
    /// stands for its argument, and a bound receiver prefix is rewritten to
    /// the argument's path.
    pub fn substitute(&self, bindings: &[(String, Expr)]) -> Expr {
        match self {
            Expr::Literal(_) => self.clone(),
            Expr::Lambda { params, body } => {
                let visible: Vec<(String, Expr)> = bindings
                    .iter()
                    .filter(|(name, _)| params.iter().all(|param| &param.name != name))
                    .cloned()
                    .collect();
                Expr::Lambda {
                    params: params.clone(),
                    body: body.iter().map(|stmt| stmt.substitute(&visible)).collect(),
                }
            }
            Expr::Path(path) => substitute_path(path, bindings).map_or_else(
                || self.clone(),
                |replacement| match replacement {
                    Replacement::Expr(expr) => expr,
                    Replacement::Path(path) => Expr::Path(path),
                },
            ),
            Expr::Call { callee, args } => Expr::Call {
                callee: match substitute_path(callee, bindings) {
                    Some(Replacement::Path(path)) => path,
                    Some(Replacement::Expr(Expr::Path(path))) => path,
                    _ => callee.clone(),
                },
                args: args.iter().map(|arg| arg.substitute(bindings)).collect(),
            },
            Expr::Binary { left, op, right } => Expr::Binary {
                left: Box::new(left.substitute(bindings)),
                op: *op,
                right: Box::new(right.substitute(bindings)),
            },
            Expr::Compare { left, op, right } => Expr::Compare {
                left: Box::new(left.substitute(bindings)),
                op: *op,
                right: Box::new(right.substitute(bindings)),
            },
            Expr::Not(inner) => Expr::Not(Box::new(inner.substitute(bindings))),
        }
    }
}

enum Replacement {
    Expr(Expr),
    Path(Path),
}

fn substitute_path(path: &Path, bindings: &[(String, Expr)]) -> Option<Replacement> {
    let (_, bound) = bindings.iter().find(|(name, _)| name == path.first())?;
    match (path.as_ident(), bound) {
        (Some(_), bound) => Some(Replacement::Expr(bound.clone())),
        (None, Expr::Path(receiver)) => Some(Replacement::Path(path.rebased(receiver))),
        (None, _) => None,
    }
}

fn literal_text(literal: &Literal) -> String {
    match literal {
        Literal::Int(value) => value.to_string(),
        Literal::Float(value) => value.to_string(),
        Literal::Bool(value) => value.to_string(),
        Literal::String(value) => {
            let mut text = String::from("\"");
            for ch in value.chars() {
                match ch {
                    '"' => text.push_str("\\\""),
                    '\\' => text.push_str("\\\\"),
                    '\n' => text.push_str("\\n"),
                    '\t' => text.push_str("\\t"),
                    other => text.push(other),
                }
            }
            text.push('"');
            text
        }
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Literal(literal) => f.write_str(&literal_text(literal)),
            Expr::Lambda { params, .. } => {
                f.write_str("(")?;
                for (index, param) in params.iter().enumerate() {
                    if index > 0 {
                        f.write_str(", ")?;
                    }
                    f.write_str(&param.name)?;
                }
                f.write_str(") => { .. }")
            }
            Expr::Path(path) => write!(f, "{path}"),
            Expr::Call { callee, args } => {
                write!(f, "{callee}(")?;
                for (index, arg) in args.iter().enumerate() {
                    if index > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{arg}")?;
                }
                f.write_str(")")
            }
            Expr::Binary { left, op, right } => {
                write!(f, "{left} {} {right}", ArithmeticLexeme::from(*op))
            }
            Expr::Compare { left, op, right } => {
                write!(f, "{left} {} {right}", ComparisonLexeme::from(*op))
            }
            Expr::Not(inner) => write!(f, "!{inner}"),
        }
    }
}

impl FromStr for Expr {
    type Err = anyhow::Error;

    fn from_str(source: &str) -> Result<Self> {
        let tokens = tokenizer::Tokens::try_from(source)?;
        let mut cursor = Cursor::new(&tokens);
        let expr = Expr::try_from(&mut cursor)?;
        if !cursor.at_end() {
            return Err(cursor.error("Unexpected tokens after the expression"));
        }
        Ok(expr)
    }
}

impl TryFrom<&mut Cursor<'_>> for Expr {
    type Error = anyhow::Error;

    fn try_from(cursor: &mut Cursor<'_>) -> Result<Self> {
        let left = additive(cursor)?;
        match comparison_operator(cursor) {
            Some((op, width)) => {
                for _ in 0..width {
                    cursor.bump();
                }
                let right = additive(cursor)?;
                Ok(Expr::Compare {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                })
            }
            None => Ok(left),
        }
    }
}

fn comparison_operator(cursor: &Cursor<'_>) -> Option<(ComparisonOperator, usize)> {
    let first = cursor.peek_symbol(0)?;
    if cursor.adjacent(1)
        && let Some(second) = cursor.peek_symbol(1)
        && let Ok(lexeme) = format!("{first}{second}").parse::<ComparisonLexeme>()
    {
        return Some((lexeme.into(), 2));
    }
    first
        .to_string()
        .parse::<ComparisonLexeme>()
        .ok()
        .map(|lexeme| (lexeme.into(), 1))
}

fn arithmetic_operator(cursor: &Cursor<'_>) -> Option<ArithmeticOperator> {
    let symbol = cursor.peek_symbol(0)?;
    ArithmeticLexeme::try_from(symbol).ok().map(Into::into)
}

fn additive(cursor: &mut Cursor<'_>) -> Result<Expr> {
    let mut left = unary(cursor)?;
    while let Some(op) = arithmetic_operator(cursor) {
        cursor.bump();
        let right = unary(cursor)?;
        left = Expr::Binary {
            left: Box::new(left),
            op,
            right: Box::new(right),
        };
    }
    Ok(left)
}

fn unary(cursor: &mut Cursor<'_>) -> Result<Expr> {
    if cursor.peek_symbol(0) == Some('!') {
        cursor.bump();
        return Ok(Expr::Not(Box::new(unary(cursor)?)));
    }
    if cursor.peek_symbol(0) == Some('-')
        && let Some(TokenKind::Number(_)) = cursor.peek_nth(1).map(|token| &token.kind)
    {
        cursor.bump();
        return match number(cursor)? {
            Literal::Int(value) => Ok(Expr::Literal(Literal::Int(-value))),
            Literal::Float(value) => Ok(Expr::Literal(Literal::Float(-value))),
            _ => unreachable!("numbers are ints or floats"),
        };
    }
    primary(cursor)
}

fn number(cursor: &mut Cursor<'_>) -> Result<Literal> {
    let Some(TokenKind::Number(whole)) = cursor.peek().map(|token| token.kind.clone()) else {
        return Err(cursor.error("Expected a number"));
    };
    cursor.bump();
    let fraction = match (cursor.peek_punct(0), cursor.peek_nth(1).map(|t| &t.kind)) {
        (Some(Punct::Dot), Some(TokenKind::Number(fraction)))
            if cursor.adjacent(0) && cursor.adjacent(1) =>
        {
            Some(fraction.clone())
        }
        _ => None,
    };
    match fraction {
        Some(fraction) => {
            cursor.bump();
            cursor.bump();
            let text = format!("{whole}.{fraction}");
            Ok(Literal::Float(text.parse::<f64>().map_err(|_| {
                cursor.error(format!("Invalid number `{text}`"))
            })?))
        }
        None => Ok(Literal::Int(whole.parse::<i64>().map_err(|_| {
            cursor.error(format!("Integer literal `{whole}` is out of range"))
        })?)),
    }
}

fn primary(cursor: &mut Cursor<'_>) -> Result<Expr> {
    let Some(token) = cursor.peek() else {
        return Err(cursor.error("Expected an expression"));
    };
    match &token.kind {
        TokenKind::String(value) => {
            cursor.bump();
            Ok(Expr::Literal(Literal::String(value.clone())))
        }
        TokenKind::Number(_) => Ok(Expr::Literal(number(cursor)?)),
        TokenKind::Identifier(word) if word == "true" || word == "false" => {
            let value = word == "true";
            cursor.bump();
            Ok(Expr::Literal(Literal::Bool(value)))
        }
        TokenKind::Identifier(_) => {
            let path = Path::try_from(&mut *cursor)?;
            if cursor.is_punct(Punct::LeftParen) {
                cursor.bump();
                let args = arguments(cursor)?;
                Ok(Expr::Call { callee: path, args })
            } else {
                Ok(Expr::Path(path))
            }
        }
        TokenKind::Symbol('(') if lambda_ahead(cursor) => lambda(cursor),
        TokenKind::Symbol('(') => {
            cursor.bump();
            let inner = Expr::try_from(&mut *cursor)?;
            cursor.expect_punct(Punct::RightParen, "`)`")?;
            Ok(inner)
        }
        _ => Err(cursor.error("Expected an expression")),
    }
}

/// Whether the parenthesis at the cursor closes right before `=>`.
fn lambda_ahead(cursor: &Cursor<'_>) -> bool {
    let mut probe = cursor.clone();
    let mut depth = 0usize;
    while let Some(token) = probe.bump() {
        match token.kind {
            TokenKind::Symbol('(') => depth += 1,
            TokenKind::Symbol(')') => {
                depth -= 1;
                if depth == 0 {
                    return probe.peek_symbol(0) == Some('=')
                        && probe.peek_symbol(1) == Some('>')
                        && probe.adjacent(1);
                }
            }
            _ => {}
        }
    }
    false
}

fn lambda(cursor: &mut Cursor<'_>) -> Result<Expr> {
    cursor.expect_punct(Punct::LeftParen, "`(`")?;
    let mut params = Vec::new();
    if !cursor.eat_punct(Punct::RightParen) {
        loop {
            params.push(Param::try_from(&mut *cursor)?);
            if cursor.eat_punct(Punct::Comma) {
                continue;
            }
            cursor.expect_punct(Punct::RightParen, "`)` to close the parameters")?;
            break;
        }
    }
    cursor.bump();
    cursor.bump();
    let body = parse_block(cursor, Scope::Function)?;
    Ok(Expr::Lambda { params, body })
}

fn arguments(cursor: &mut Cursor<'_>) -> Result<Vec<Expr>> {
    let mut args = Vec::new();
    if cursor.eat_punct(Punct::RightParen) {
        return Ok(args);
    }
    loop {
        args.push(Expr::try_from(&mut *cursor)?);
        if cursor.eat_punct(Punct::Comma) {
            continue;
        }
        cursor.expect_punct(Punct::RightParen, "`)` to close the arguments")?;
        return Ok(args);
    }
}
