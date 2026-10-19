use anyhow::Result;
use parser::{ArithmeticLexeme, ComparisonLexeme, LogicalLexeme};
use std::fmt;
use std::str::FromStr;
use types::{
    ArithmeticOperator, ComparisonOperator, Keyword, Literal, LogicalOperator, Punct, TokenKind,
};

use super::path::Path;
use crate::stmt::{Scope, parse_block};
use crate::{Cursor, Param, Stmt, TypeRef};

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Lambda {
        params: Vec<Param>,
        body: Vec<Stmt>,
    },
    Thread {
        body: Vec<Stmt>,
    },
    Await(Box<Expr>),
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
    Logical {
        left: Box<Expr>,
        op: LogicalOperator,
        right: Box<Expr>,
    },
    Compare {
        left: Box<Expr>,
        op: ComparisonOperator,
        right: Box<Expr>,
    },
    Not(Box<Expr>),
    Cast {
        value: Box<Expr>,
        ty: TypeRef,
    },
    /// An array literal, `[1, 2, 3]`.
    Array(Vec<Expr>),
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
            Expr::Thread { body } => Expr::Thread {
                body: body.iter().map(|stmt| stmt.substitute(bindings)).collect(),
            },
            Expr::Await(inner) => Expr::Await(Box::new(inner.substitute(bindings))),
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
            Expr::Logical { left, op, right } => Expr::Logical {
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
            Expr::Cast { value, ty } => Expr::Cast {
                value: Box::new(value.substitute(bindings)),
                ty: ty.clone(),
            },
            Expr::Array(items) => {
                Expr::Array(items.iter().map(|item| item.substitute(bindings)).collect())
            }
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
        Literal::Char(value) => match value {
            '\'' => "'\\''".to_owned(),
            '\\' => "'\\\\'".to_owned(),
            '\n' => "'\\n'".to_owned(),
            '\t' => "'\\t'".to_owned(),
            other => format!("'{other}'"),
        },
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
            Expr::Thread { .. } => f.write_str("thread { .. }"),
            Expr::Await(inner) => write!(f, "await {inner}"),
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
            Expr::Logical { left, op, right } => {
                write!(f, "{left} {} {right}", LogicalLexeme::from(*op))
            }
            Expr::Compare { left, op, right } => {
                write!(f, "{left} {} {right}", ComparisonLexeme::from(*op))
            }
            Expr::Not(inner) => write!(f, "!{inner}"),
            Expr::Cast { value, ty } => write!(f, "{value} as {ty}"),
            Expr::Array(items) => {
                f.write_str("[")?;
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{item}")?;
                }
                f.write_str("]")
            }
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
        logical(cursor, LogicalOperator::Or)
    }
}

fn logical(cursor: &mut Cursor<'_>, level: LogicalOperator) -> Result<Expr> {
    let operand = |cursor: &mut Cursor<'_>| match level {
        LogicalOperator::Or => logical(cursor, LogicalOperator::And),
        LogicalOperator::And => comparison(cursor),
    };
    let mut left = operand(cursor)?;
    while logical_operator(cursor) == Some(level) {
        cursor.bump();
        cursor.bump();
        let right = operand(cursor)?;
        left = Expr::Logical {
            left: Box::new(left),
            op: level,
            right: Box::new(right),
        };
    }
    Ok(left)
}

fn logical_operator(cursor: &Cursor<'_>) -> Option<LogicalOperator> {
    let first = cursor.peek_symbol(0)?;
    let second = cursor.peek_symbol(1).filter(|_| cursor.adjacent(1))?;
    format!("{first}{second}")
        .parse::<LogicalLexeme>()
        .ok()
        .map(Into::into)
}

fn comparison(cursor: &mut Cursor<'_>) -> Result<Expr> {
    let left = binary(cursor, 0)?;
    match comparison_operator(cursor) {
        Some((op, width)) => {
            for _ in 0..width {
                cursor.bump();
            }
            let right = binary(cursor, 0)?;
            Ok(Expr::Compare {
                left: Box::new(left),
                op,
                right: Box::new(right),
            })
        }
        None => Ok(left),
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

/// The binary operators from the loosest binding to the tightest.
const LEVELS: [&[ArithmeticOperator]; 6] = [
    &[ArithmeticOperator::BitOr],
    &[ArithmeticOperator::BitXor],
    &[ArithmeticOperator::BitAnd],
    &[
        ArithmeticOperator::ShiftLeft,
        ArithmeticOperator::ShiftRight,
    ],
    &[ArithmeticOperator::Add, ArithmeticOperator::Sub],
    &[
        ArithmeticOperator::Mul,
        ArithmeticOperator::Div,
        ArithmeticOperator::Rem,
    ],
];

/// The operator at the cursor when it belongs to `level`, and how many symbols
/// it is written with. `&&`, `||` and the comparisons are not arithmetic.
fn arithmetic_operator(cursor: &Cursor<'_>, level: usize) -> Option<(ArithmeticOperator, usize)> {
    let first = cursor.peek_symbol(0)?;
    let second = cursor.peek_symbol(1).filter(|_| cursor.adjacent(1));
    let (operator, width) = match (first, second) {
        ('<', Some('<')) => (ArithmeticOperator::ShiftLeft, 2),
        ('>', Some('>')) => (ArithmeticOperator::ShiftRight, 2),
        ('&', Some('&')) | ('|', Some('|')) | ('<' | '>', _) => return None,
        (symbol, _) => (ArithmeticLexeme::try_from(symbol).ok()?.into(), 1),
    };
    LEVELS[level]
        .contains(&operator)
        .then_some((operator, width))
}

fn binary(cursor: &mut Cursor<'_>, level: usize) -> Result<Expr> {
    if level == LEVELS.len() {
        return cast(cursor);
    }
    let mut left = binary(cursor, level + 1)?;
    while let Some((op, width)) = arithmetic_operator(cursor, level) {
        for _ in 0..width {
            cursor.bump();
        }
        let right = binary(cursor, level + 1)?;
        left = Expr::Binary {
            left: Box::new(left),
            op,
            right: Box::new(right),
        };
    }
    Ok(left)
}

fn cast(cursor: &mut Cursor<'_>) -> Result<Expr> {
    let mut value = unary(cursor)?;
    while cursor.eat_keyword(Keyword::As) {
        value = Expr::Cast {
            value: Box::new(value),
            ty: TypeRef::parse_plain(cursor)?,
        };
    }
    Ok(value)
}

fn unary(cursor: &mut Cursor<'_>) -> Result<Expr> {
    if cursor.eat_keyword(Keyword::Await) {
        return Ok(Expr::Await(Box::new(unary(cursor)?)));
    }
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
        TokenKind::Char(value) => {
            let value = *value;
            cursor.bump();
            Ok(Expr::Literal(Literal::Char(value)))
        }
        TokenKind::Number(_) => Ok(Expr::Literal(number(cursor)?)),
        TokenKind::Identifier(_)
            if cursor.peek_keyword(0) == Some(Keyword::Thread)
                && cursor.peek_punct(1) == Some(Punct::LeftBrace) =>
        {
            cursor.bump();
            let body = parse_block(cursor, Scope::Function)?;
            Ok(Expr::Thread { body })
        }
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
        TokenKind::Symbol('[') => array(cursor),
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

fn array(cursor: &mut Cursor<'_>) -> Result<Expr> {
    cursor.bump();
    let mut items = Vec::new();
    if cursor.peek_symbol(0) == Some(']') {
        cursor.bump();
        return Ok(Expr::Array(items));
    }
    loop {
        items.push(Expr::try_from(&mut *cursor)?);
        if cursor.eat_punct(Punct::Comma) {
            if cursor.peek_symbol(0) == Some(']') {
                cursor.bump();
                return Ok(Expr::Array(items));
            }
            continue;
        }
        if cursor.peek_symbol(0) != Some(']') {
            return Err(cursor.error("Expected `]` to close the array"));
        }
        cursor.bump();
        return Ok(Expr::Array(items));
    }
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
