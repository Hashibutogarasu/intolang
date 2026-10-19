use anyhow::Result;
use types::{Keyword, Punct};

use super::attribute::Attribute;
use super::event_decl::EventDecl;
use super::function::FunctionDecl;
use super::type_params::parse_type_params;
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

/// `[pub] [abstract] Name[<T, U>]() [: Parent[<T>]] { fields, methods and events }`.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassDecl {
    pub line: usize,
    pub name: String,
    pub is_public: bool,
    pub is_abstract: bool,
    /// The type parameters written after the name, `T` in `Box<T>()`.
    pub type_params: Vec<String>,
    /// The name of the parent class, without its type arguments.
    pub parent: Option<String>,
    /// The parent as written, with its type arguments.
    pub parent_type: Option<TypeRef>,
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
        if probe.peek_word(offset).is_none() || probe.peek_keyword(offset).is_some() {
            return false;
        }
        offset = skip_brackets(&probe, offset + 1);
        let header = probe.peek_punct(offset) == Some(Punct::LeftParen)
            && probe.peek_punct(offset + 1) == Some(Punct::RightParen);
        if !header {
            return false;
        }
        match probe.peek_punct(offset + 2) {
            Some(Punct::LeftBrace) => true,
            Some(Punct::Colon) => {
                if probe.peek_word(offset + 3).is_none() {
                    return false;
                }
                let end = skip_brackets(&probe, offset + 4);
                probe.peek_punct(end) == Some(Punct::LeftBrace)
            }
            _ => false,
        }
    }
}

/// The offset after the `<...>` that starts at `offset`, or `offset` itself
/// when no angle brackets start there.
fn skip_brackets(cursor: &Cursor<'_>, offset: usize) -> usize {
    if cursor.peek_symbol(offset) != Some('<') {
        return offset;
    }
    let mut depth = 0usize;
    let mut index = offset;
    while let Some(_) = cursor.peek_nth(index) {
        match cursor.peek_symbol(index) {
            Some('<') => depth += 1,
            Some('>') => {
                depth -= 1;
                if depth == 0 {
                    return index + 1;
                }
            }
            _ => {}
        }
        index += 1;
    }
    offset
}

impl TryFrom<&mut Cursor<'_>> for ClassDecl {
    type Error = anyhow::Error;

    fn try_from(cursor: &mut Cursor<'_>) -> Result<Self> {
        let attributes = Attribute::parse_all(cursor)?;
        let line = cursor.line();
        let is_public = cursor.eat_keyword(Keyword::Pub);
        let is_abstract = cursor.eat_keyword(Keyword::Abstract);
        let (name, _) = cursor.expect_ident("a class name")?;
        let type_params = parse_type_params(cursor)?;
        cursor.expect_punct(Punct::LeftParen, "`(`")?;
        cursor.expect_punct(Punct::RightParen, "`)`")?;
        let parent_type = if cursor.eat_punct(Punct::Colon) {
            Some(TypeRef::parse_plain(cursor)?)
        } else {
            None
        };
        let parent = parent_type.as_ref().map(|parent| {
            parent
                .as_str()
                .split('<')
                .next()
                .unwrap_or_default()
                .to_owned()
        });
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
            type_params,
            parent,
            parent_type,
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
