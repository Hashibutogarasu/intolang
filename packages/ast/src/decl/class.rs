use errors::Result;
use types::{Keyword, Literal, Punct};

use super::attribute::Attribute;
use super::event_decl::EventDecl;
use super::function::FunctionDecl;
use super::type_params::parse_type_params;
use crate::{Cursor, Expr, Located, SyntaxError, TypeRef};

/// `pub into: Type name = default;` inside a class body.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldDecl {
    pub line: usize,
    pub name: String,
    pub name_span: Located,
    pub ty: TypeRef,
    pub default: Option<Expr>,
    pub is_static: bool,
}

/// `[pub] [abstract] Name[<T, U>]() [: Parent[<T>]] { fields, methods and events }`.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassDecl {
    pub line: usize,
    pub name: String,
    pub name_span: Located,
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
    type Error = errors::Error;

    fn try_from(cursor: &mut Cursor<'_>) -> Result<Self> {
        let attributes = Attribute::parse_all(cursor)?;
        let line = cursor.line();
        let is_public = cursor.eat_keyword(Keyword::Pub);
        let is_abstract = cursor.eat_keyword(Keyword::Abstract);
        let (name, _) = cursor.expect_ident("a class name")?;
        let name_span = cursor.previous_span();
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
            name_span,
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

/// `pub into name = value;`: the type of the field is the type of its value.
fn inferred_field(
    cursor: &mut Cursor<'_>,
    class: &str,
    line: usize,
    is_static: bool,
) -> Result<FieldDecl> {
    let malformed = || {
        SyntaxError::new(
            line,
            format!("Expected class field 'pub into <name> = <value>;' in `{class}`"),
        )
    };
    let (name, _) = cursor
        .expect_ident("a field name")
        .map_err(|_| malformed())?;
    let name_span = cursor.previous_span();
    if !cursor.eat_punct(Punct::Assign) {
        return Err(SyntaxError::new(
            line,
            format!(
                "The field `{name}` of `{class}` needs a type or a value to take its type from"
            ),
        )
        .into());
    }
    let default = Expr::try_from(&mut *cursor).map_err(|_| malformed())?;
    cursor.end_statement().map_err(|_| malformed())?;
    let Some(written) = literal_type(&default) else {
        return Err(SyntaxError::new(
            line,
            format!("The type of the field `{name}` of `{class}` cannot be inferred from `{default}`; write `pub into: <Type> {name}`"),
        )
        .into());
    };
    Ok(FieldDecl {
        line,
        name,
        name_span,
        ty: TypeRef::from(written.as_str()),
        default: Some(default),
        is_static,
    })
}

/// The type a field takes from the value it starts with: the type of a
/// literal, or the class a constructor call builds.
fn literal_type(value: &Expr) -> Option<String> {
    match value {
        Expr::Literal(Literal::Int(_)) => Some("Int32".to_owned()),
        Expr::Literal(Literal::Float(_)) => Some("Float64".to_owned()),
        Expr::Literal(Literal::String(_)) => Some("String".to_owned()),
        Expr::Literal(Literal::Bool(_)) => Some("Boolean".to_owned()),
        Expr::Literal(Literal::Char(_)) => Some("Char".to_owned()),
        Expr::Call { callee, .. } => callee.as_ident().map(str::to_owned),
        _ => None,
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
    if !cursor.eat_keyword(Keyword::Into) {
        return Err(legacy().into());
    }
    let malformed = || {
        SyntaxError::new(
            line,
            format!("Expected class field 'pub into: <Type> <name> = <default>;' in `{class}`"),
        )
    };
    if !cursor.eat_punct(Punct::Colon) {
        return inferred_field(cursor, class, line, is_static);
    }
    let ty = TypeRef::try_from(&mut *cursor).map_err(|_| malformed())?;
    let (name, _) = cursor
        .expect_ident("a field name")
        .map_err(|_| malformed())?;
    let name_span = cursor.previous_span();
    let default = if cursor.eat_punct(Punct::Assign) {
        Some(Expr::try_from(&mut *cursor).map_err(|_| malformed())?)
    } else {
        None
    };
    cursor.end_statement().map_err(|_| malformed())?;
    Ok(FieldDecl {
        line,
        name,
        name_span,
        ty,
        default,
        is_static,
    })
}
