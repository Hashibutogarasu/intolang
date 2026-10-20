use errors::{Error, Result};
use std::str::FromStr;
use tokenizer::Tokens;

use super::item::Item;
use crate::decl::{ClassDecl, EnumDecl, EventDecl, FunctionDecl, MacroDecl, UseDecl};
use crate::{Cursor, Stmt};

/// A parsed source file: its top-level items in source order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Program {
    pub items: Vec<Item>,
}

impl Program {
    pub fn imports(&self) -> impl Iterator<Item = &UseDecl> {
        self.items.iter().filter_map(|item| match item {
            Item::Import(decl) => Some(decl),
            _ => None,
        })
    }

    pub fn exports(&self) -> impl Iterator<Item = &UseDecl> {
        self.items.iter().filter_map(|item| match item {
            Item::Export(decl) => Some(decl),
            _ => None,
        })
    }

    pub fn enums(&self) -> impl Iterator<Item = &EnumDecl> {
        self.items.iter().filter_map(|item| match item {
            Item::Enum(decl) => Some(decl),
            _ => None,
        })
    }

    pub fn classes(&self) -> impl Iterator<Item = &ClassDecl> {
        self.items.iter().filter_map(|item| match item {
            Item::Class(decl) => Some(decl),
            _ => None,
        })
    }

    pub fn functions(&self) -> impl Iterator<Item = &FunctionDecl> {
        self.items.iter().filter_map(|item| match item {
            Item::Function(decl) => Some(decl),
            _ => None,
        })
    }

    pub fn events(&self) -> impl Iterator<Item = &EventDecl> {
        self.items.iter().filter_map(|item| match item {
            Item::Event(decl) => Some(decl),
            _ => None,
        })
    }

    pub fn macros(&self) -> impl Iterator<Item = &MacroDecl> {
        self.items.iter().filter_map(|item| match item {
            Item::Macro(decl) => Some(decl),
            _ => None,
        })
    }

    pub fn statements(&self) -> impl Iterator<Item = &Stmt> {
        self.items.iter().filter_map(|item| match item {
            Item::Stmt(stmt) => Some(stmt),
            _ => None,
        })
    }
}

impl Program {
    /// Parse every item that can be parsed, and collect the syntax error of
    /// each one that cannot instead of stopping at the first.
    pub fn parse_recovering(tokens: &Tokens) -> (Self, Vec<Error>) {
        let mut cursor = Cursor::recovering(tokens);
        let mut items = Vec::new();
        while !cursor.at_end() {
            let start = cursor.position();
            match Item::try_from(&mut cursor) {
                Ok(item) => items.push(item),
                Err(error) => cursor
                    .recover(error, start)
                    .expect("a recovering cursor collects the error"),
            }
        }
        (Self { items }, cursor.take_recovered())
    }

    /// [`Program::parse_recovering`] over source text; a lexical error ends
    /// the parse with that one error.
    pub fn parse_all(source: &str) -> (Self, Vec<Error>) {
        match source.parse::<Tokens>() {
            Ok(tokens) => Self::parse_recovering(&tokens),
            Err(error) => (Self::default(), vec![error]),
        }
    }
}

impl TryFrom<&Tokens> for Program {
    type Error = Error;

    fn try_from(tokens: &Tokens) -> Result<Self> {
        let mut cursor = Cursor::new(tokens);
        let mut items = Vec::new();
        while !cursor.at_end() {
            items.push(Item::try_from(&mut cursor)?);
        }
        Ok(Self { items })
    }
}

impl FromStr for Program {
    type Err = Error;

    fn from_str(source: &str) -> Result<Self> {
        Program::try_from(&source.parse::<Tokens>()?)
    }
}
