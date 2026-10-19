use parser::{CompoundLexeme, KeywordLexeme, PunctLexeme};
use types::{CompoundOperator, Keyword, Punct, Token, TokenKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Word(Option<Keyword>),
    Literal,
    Punct(Punct),
    Compound(CompoundOperator),
    Symbol(char),
    Comment,
}

/// A printable unit: a token, or two adjacent symbol tokens that form one
/// compound operator.
#[derive(Debug, Clone)]
pub(crate) struct Piece {
    pub text: String,
    pub kind: Kind,
    pub line: usize,
    pub end_line: usize,
    pub start: usize,
    pub end: usize,
}

impl Piece {
    pub fn is(&self, punct: Punct) -> bool {
        self.kind == Kind::Punct(punct)
    }

    pub fn is_keyword(&self, keyword: Keyword) -> bool {
        self.kind == Kind::Word(Some(keyword))
    }

    pub fn is_word_like(&self) -> bool {
        matches!(self.kind, Kind::Word(_) | Kind::Literal)
    }

    pub fn is_operator(&self) -> bool {
        matches!(self.kind, Kind::Compound(_) | Kind::Punct(Punct::Assign))
    }
}

pub(crate) fn pieces(tokens: &[Token], chars: &[char]) -> Vec<Piece> {
    let raw = |token: &Token| -> String {
        chars[token.span.start..token.span.end.min(chars.len())]
            .iter()
            .collect()
    };
    let mut pieces = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        let token = &tokens[index];
        let span = token.span;
        let (text, kind, end) = match &token.kind {
            TokenKind::Identifier(name) => {
                let keyword = name.parse::<KeywordLexeme>().ok().map(Keyword::from);
                (name.clone(), Kind::Word(keyword), span.end)
            }
            TokenKind::Number(_) | TokenKind::String(_) | TokenKind::Char(_) => {
                (raw(token), Kind::Literal, span.end)
            }
            TokenKind::Comment(text) => (text.clone(), Kind::Comment, span.end),
            TokenKind::Symbol(symbol) => {
                let compound = tokens.get(index + 1).and_then(|next| match next.kind {
                    TokenKind::Symbol(second) if next.span.start == span.end => {
                        CompoundLexeme::try_from((*symbol, second))
                            .ok()
                            .map(|lexeme| (CompoundOperator::from(lexeme), second, next.span.end))
                    }
                    _ => None,
                });
                match compound {
                    Some((operator, second, end)) => {
                        index += 1;
                        (format!("{symbol}{second}"), Kind::Compound(operator), end)
                    }
                    None => match PunctLexeme::try_from(*symbol) {
                        Ok(punct) => (symbol.to_string(), Kind::Punct(punct.into()), span.end),
                        Err(_) => (symbol.to_string(), Kind::Symbol(*symbol), span.end),
                    },
                }
            }
        };
        pieces.push(Piece {
            text,
            kind,
            line: span.line,
            end_line: span.end_line,
            start: span.start,
            end,
        });
        index += 1;
    }
    pieces
}
