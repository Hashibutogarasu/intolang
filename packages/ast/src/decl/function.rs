use errors::Result;
use parser::{KeywordLexeme, LanguageLexeme};
use types::{Keyword, Language, Punct};

use super::attribute::Attribute;
use super::param::Param;
use super::type_params::parse_type_params;
use crate::stmt::{Scope, parse_block};
use crate::{Cursor, Located, Stmt, TypeRef};

/// `[pub] [static] [abstract|default|override] [c|cpp] fun name<T>(params):
/// Type { body }`, or the same header ending in `;` for a declaration
/// implemented elsewhere.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDecl {
    pub line: usize,
    pub name: String,
    pub name_span: Located,
    pub is_public: bool,
    pub is_static: bool,
    pub is_abstract: bool,
    pub is_default: bool,
    pub is_override: bool,
    pub language: Option<Language>,
    pub attributes: Vec<Attribute>,
    pub type_params: Vec<String>,
    pub params: Vec<Param>,
    pub return_type: Option<TypeRef>,
    pub body: Option<Vec<Stmt>>,
}

impl FunctionDecl {
    /// Whether a function declaration (with optional attributes) starts here.
    pub(crate) fn ahead(cursor: &Cursor<'_>) -> bool {
        let mut probe = cursor.clone();
        if Attribute::parse_all(&mut probe).is_err() {
            return false;
        }
        Self::header_ahead(&probe)
    }

    /// Whether modifiers followed by `fun` start at the cursor.
    pub(crate) fn header_ahead(cursor: &Cursor<'_>) -> bool {
        let mut offset = 0;
        while let Some(word) = cursor.peek_word(offset) {
            match word.parse::<KeywordLexeme>().ok().map(Keyword::from) {
                Some(Keyword::Fun) => return true,
                Some(
                    Keyword::Pub
                    | Keyword::Static
                    | Keyword::Abstract
                    | Keyword::Default
                    | Keyword::Override,
                ) => {}
                _ if word.parse::<LanguageLexeme>().is_ok() => {}
                _ => return false,
            }
            offset += 1;
        }
        false
    }
}

impl TryFrom<&mut Cursor<'_>> for FunctionDecl {
    type Error = errors::Error;

    fn try_from(cursor: &mut Cursor<'_>) -> Result<Self> {
        let attributes = Attribute::parse_all(cursor)?;
        let line = cursor.line();
        let (mut is_public, mut is_static, mut language) = (false, false, None);
        let (mut is_abstract, mut is_default, mut is_override) = (false, false, false);
        loop {
            if cursor.eat_keyword(Keyword::Pub) {
                is_public = true;
            } else if cursor.eat_keyword(Keyword::Static) {
                is_static = true;
            } else if cursor.eat_keyword(Keyword::Abstract) {
                is_abstract = true;
            } else if cursor.eat_keyword(Keyword::Default) {
                is_default = true;
            } else if cursor.eat_keyword(Keyword::Override) {
                is_override = true;
            } else if let Some(parsed) = cursor
                .peek_word(0)
                .and_then(|word| word.parse::<LanguageLexeme>().ok())
            {
                language = Some(Language::from(parsed));
                cursor.bump();
            } else {
                break;
            }
        }
        if !cursor.eat_keyword(Keyword::Fun) {
            return Err(cursor.error("Expected `fun`"));
        }
        let (name, _) = cursor.expect_ident("a function name")?;
        let name_span = cursor.previous_span();
        let type_params = parse_type_params(cursor)?;
        cursor.expect_punct(Punct::LeftParen, "`(` after the function name")?;
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
        let return_type = if cursor.eat_punct(Punct::Colon) {
            Some(TypeRef::try_from(&mut *cursor)?)
        } else {
            None
        };
        let body = if cursor.is_punct(Punct::LeftBrace) {
            Some(parse_block(cursor, Scope::Function)?)
        } else {
            cursor.end_statement()?;
            None
        };
        Ok(Self {
            line,
            name,
            name_span,
            is_public,
            is_static,
            is_abstract,
            is_default,
            is_override,
            language,
            attributes,
            type_params,
            params,
            return_type,
            body,
        })
    }
}
