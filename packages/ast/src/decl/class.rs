use anyhow::Result;
use types::{Keyword, Punct};

use super::attribute::Attribute;
use super::event_decl::EventDecl;
use super::function::FunctionDecl;
use crate::{Cursor, Expr, SyntaxError, TypeRef};

/// `pub into: Type name = default;` inside a class body.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldDecl {
    pub line: usize,
    pub name: String,
    pub ty: TypeRef,
    pub default: Option<Expr>,
    pub is_static: bool,
}

/// `[pub] [abstract] Name() [: Parent] { fields, methods and events }`.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassDecl {
    pub line: usize,
    pub name: String,
    pub is_public: bool,
    pub is_abstract: bool,
    pub parent: Option<String>,
    pub attributes: Vec<Attribute>,
    pub fields: Vec<FieldDecl>,
    pub methods: Vec<FunctionDecl>,
    pub events: Vec<EventDecl>,
}

impl ClassDecl {
    /// Whether a class declaration (with optional attributes) starts here.
    pub(crate) fn ahead(cursor: &Cursor<'_>) -> bool {
        let mut probe = cursor.clone();
        if Attribute::parse_all(&mut probe).is_err() {
            return false;
        }
        let mut offset = usize::from(probe.peek_keyword(0) == Some(Keyword::Pub));
        if probe.peek_keyword(offset) == Some(Keyword::Abstract) {
            offset += 1;
        }
        let header = probe.peek_word(offset).is_some()
            && probe.peek_keyword(offset).is_none()
            && probe.peek_punct(offset + 1) == Some(Punct::LeftParen)
            && probe.peek_punct(offset + 2) == Some(Punct::RightParen);
        if !header {
            return false;
        }
        match probe.peek_punct(offset + 3) {
            Some(Punct::LeftBrace) => true,
            Some(Punct::Colon) => {
                probe.peek_word(offset + 4).is_some()
                    && probe.peek_punct(offset + 5) == Some(Punct::LeftBrace)
            }
            _ => false,
        }
    }
}

impl TryFrom<&mut Cursor<'_>> for ClassDecl {
    type Error = anyhow::Error;

    fn try_from(cursor: &mut Cursor<'_>) -> Result<Self> {
        let attributes = Attribute::parse_all(cursor)?;
        let line = cursor.line();
        let is_public = cursor.eat_keyword(Keyword::Pub);
        let is_abstract = cursor.eat_keyword(Keyword::Abstract);
        let (name, _) = cursor.expect_ident("a class name")?;
        cursor.expect_punct(Punct::LeftParen, "`(`")?;
        cursor.expect_punct(Punct::RightParen, "`)`")?;
        let parent = if cursor.eat_punct(Punct::Colon) {
            Some(cursor.expect_ident("a parent class name")?.0)
        } else {
            None
        };
        cursor.expect_punct(Punct::LeftBrace, "`{` after the class header")?;
        let mut fields = Vec::new();
        let mut methods = Vec::new();
        let mut events = Vec::new();
        loop {
            if cursor.eat_punct(Punct::RightBrace) {
                break;
            }
            if cursor.at_end() {
                return Err(cursor.error(format!("Expected `}}` to close class `{name}`")));
            }
            if EventDecl::ahead(cursor) {
                events.push(EventDecl::try_from(&mut *cursor)?);
            } else if FunctionDecl::ahead(cursor) {
                methods.push(FunctionDecl::try_from(&mut *cursor)?);
            } else {
                fields.push(field(cursor, &name)?);
            }
        }
        Ok(Self {
            line,
            name,
            is_public,
            is_abstract,
            parent,
            attributes,
            fields,
            methods,
            events,
        })
    }
}

fn field(cursor: &mut Cursor<'_>, class: &str) -> Result<FieldDecl> {
    let line = cursor.line();
    let legacy = || {
        SyntaxError::new(
            line,
            format!(
                "Class `{class}` fields must be declared as 'pub into: <Type> <name> = <default>;'"
            ),
        )
    };
    if !cursor.eat_keyword(Keyword::Pub) {
        return Err(legacy().into());
    }
    let is_static = cursor.eat_keyword(Keyword::Static);
    if !(cursor.eat_keyword(Keyword::Into) && cursor.eat_punct(Punct::Colon)) {
        return Err(legacy().into());
    }
    let malformed = || {
        SyntaxError::new(
            line,
            format!("Expected class field 'pub into: <Type> <name> = <default>;' in `{class}`"),
        )
    };
    let ty = TypeRef::try_from(&mut *cursor).map_err(|_| malformed())?;
    let (name, _) = cursor
        .expect_ident("a field name")
        .map_err(|_| malformed())?;
    let default = if cursor.eat_punct(Punct::Assign) {
        Some(Expr::try_from(&mut *cursor).map_err(|_| malformed())?)
    } else {
        None
    };
    cursor.end_statement().map_err(|_| malformed())?;
    Ok(FieldDecl {
        line,
        name,
        ty,
        default,
        is_static,
    })
}
