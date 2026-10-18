use anyhow::{Error, Result, bail};
use std::fmt;
use std::str::FromStr;
use types::{ArithmeticOperator, ComparisonOperator, CompoundOperator};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArithmeticLexeme(ArithmeticOperator);

impl FromStr for ArithmeticLexeme {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self> {
        Ok(Self(match text {
            "+" | "add" => ArithmeticOperator::Add,
            "-" | "sub" => ArithmeticOperator::Sub,
            "*" | "mul" => ArithmeticOperator::Mul,
            "/" | "div" => ArithmeticOperator::Div,
            other => bail!("Unsupported arithmetic operator `{other}`"),
        }))
    }
}

impl ArithmeticLexeme {
    /// The spelling used for the operation in a lowered message.
    pub fn message_name(&self) -> &'static str {
        match self.0 {
            ArithmeticOperator::Add => "add",
            ArithmeticOperator::Sub => "sub",
            ArithmeticOperator::Mul => "mul",
            ArithmeticOperator::Div => "div",
        }
    }
}

impl TryFrom<char> for ArithmeticLexeme {
    type Error = Error;

    fn try_from(symbol: char) -> Result<Self> {
        symbol.to_string().parse()
    }
}

impl From<ArithmeticLexeme> for ArithmeticOperator {
    fn from(lexeme: ArithmeticLexeme) -> Self {
        lexeme.0
    }
}

impl From<ArithmeticOperator> for ArithmeticLexeme {
    fn from(operator: ArithmeticOperator) -> Self {
        Self(operator)
    }
}

impl fmt::Display for ArithmeticLexeme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            ArithmeticOperator::Add => "+",
            ArithmeticOperator::Sub => "-",
            ArithmeticOperator::Mul => "*",
            ArithmeticOperator::Div => "/",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompoundLexeme(CompoundOperator);

impl TryFrom<(char, char)> for CompoundLexeme {
    type Error = Error;

    fn try_from((first, second): (char, char)) -> Result<Self> {
        format!("{first}{second}").parse()
    }
}

impl FromStr for CompoundLexeme {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self> {
        Ok(Self(match text {
            "==" => CompoundOperator::Equal,
            "!=" => CompoundOperator::NotEqual,
            "<=" => CompoundOperator::LessEqual,
            ">=" => CompoundOperator::GreaterEqual,
            "&&" => CompoundOperator::And,
            "||" => CompoundOperator::Or,
            other => bail!("`{other}` is not a compound operator"),
        }))
    }
}

impl From<CompoundOperator> for CompoundLexeme {
    fn from(operator: CompoundOperator) -> Self {
        Self(operator)
    }
}

impl From<CompoundLexeme> for CompoundOperator {
    fn from(lexeme: CompoundLexeme) -> Self {
        lexeme.0
    }
}

impl fmt::Display for CompoundLexeme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            CompoundOperator::Equal => "==",
            CompoundOperator::NotEqual => "!=",
            CompoundOperator::LessEqual => "<=",
            CompoundOperator::GreaterEqual => ">=",
            CompoundOperator::And => "&&",
            CompoundOperator::Or => "||",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComparisonLexeme(ComparisonOperator);

impl ComparisonLexeme {
    /// Operators in the order they must be tried when scanning an
    /// expression, so that `>=` is found before `>`.
    pub const SCAN_ORDER: [&'static str; 6] = [">=", "<=", "==", "!=", ">", "<"];
}

impl FromStr for ComparisonLexeme {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self> {
        Ok(Self(match text {
            "==" => ComparisonOperator::Equal,
            "!=" => ComparisonOperator::NotEqual,
            "<" => ComparisonOperator::Less,
            "<=" => ComparisonOperator::LessEqual,
            ">" => ComparisonOperator::Greater,
            ">=" => ComparisonOperator::GreaterEqual,
            other => bail!("Unsupported comparison operator `{other}`"),
        }))
    }
}

impl From<ComparisonLexeme> for ComparisonOperator {
    fn from(lexeme: ComparisonLexeme) -> Self {
        lexeme.0
    }
}

impl From<ComparisonOperator> for ComparisonLexeme {
    fn from(operator: ComparisonOperator) -> Self {
        Self(operator)
    }
}

impl fmt::Display for ComparisonLexeme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            ComparisonOperator::Equal => "==",
            ComparisonOperator::NotEqual => "!=",
            ComparisonOperator::Less => "<",
            ComparisonOperator::LessEqual => "<=",
            ComparisonOperator::Greater => ">",
            ComparisonOperator::GreaterEqual => ">=",
        })
    }
}
