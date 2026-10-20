use errors::{Error, Result};
use parser::{KeywordLexeme, PunctLexeme};
use types::{Keyword, Punct, Token, TokenKind};

use super::error::SyntaxError;

/// A read position over a token stream, with the lookahead helpers the node
/// builders share.
#[derive(Clone)]
pub struct Cursor<'a> {
    tokens: &'a [Token],
    index: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, index: 0 }
    }

    pub fn at_end(&self) -> bool {
        self.index >= self.tokens.len()
    }

    pub fn position(&self) -> usize {
        self.index
    }

    pub fn rewind(&mut self, position: usize) {
        self.index = position;
    }

    pub fn peek(&self) -> Option<&'a Token> {
        self.tokens.get(self.index)
    }

    pub fn peek_nth(&self, offset: usize) -> Option<&'a Token> {
        self.tokens.get(self.index + offset)
    }

    pub fn bump(&mut self) -> Option<&'a Token> {
        let token = self.tokens.get(self.index);
        if token.is_some() {
            self.index += 1;
        }
        token
    }

    /// The line of the next token, or of the last one at the end of input.
    pub fn line(&self) -> usize {
        self.peek()
            .or_else(|| self.tokens.last())
            .map_or(1, Token::line)
    }

    /// The last line of the token just consumed.
    pub fn previous_end_line(&self) -> usize {
        self.index
            .checked_sub(1)
            .and_then(|index| self.tokens.get(index))
            .map_or(1, |token| token.span.end_line)
    }

    pub fn error(&self, message: impl Into<String>) -> Error {
        self.syntax_error(message).into()
    }

    /// An error that names the construct the parser expected at the next token.
    pub fn error_expecting(&self, expected: &str, message: impl Into<String>) -> Error {
        self.syntax_error(message).expecting(expected).into()
    }

    /// A syntax error at the next token, with the token found there.
    pub fn syntax_error(&self, message: impl Into<String>) -> SyntaxError {
        let error = SyntaxError::new(self.line(), message).found(self.describe_next());
        match self.peek().or_else(|| self.tokens.last()) {
            Some(token) => error.at_span(token.span),
            None => error,
        }
    }

    /// The error of an expected construct: the message is `Expected {what}`.
    fn expected(&self, what: &str) -> Error {
        self.syntax_error(format!("Expected {what}"))
            .expecting(what)
            .into()
    }

    fn describe_next(&self) -> String {
        match self.peek().map(|token| &token.kind) {
            None => "end of input".to_string(),
            Some(TokenKind::Identifier(word)) => format!("`{word}`"),
            Some(TokenKind::Number(number)) => format!("`{number}`"),
            Some(TokenKind::Symbol(symbol)) => format!("`{symbol}`"),
            Some(TokenKind::String(_)) => "a string".to_string(),
            Some(TokenKind::Char(_)) => "a character".to_string(),
            Some(TokenKind::Comment(_)) => "a comment".to_string(),
        }
    }

    /// Whether the token at `offset` starts exactly where the one before it
    /// ends, with no whitespace between them.
    pub fn adjacent(&self, offset: usize) -> bool {
        let Some(index) = (self.index + offset).checked_sub(1) else {
            return false;
        };
        match (self.tokens.get(index), self.tokens.get(index + 1)) {
            (Some(before), Some(after)) => before.span.end == after.span.start,
            _ => false,
        }
    }

    pub fn peek_symbol(&self, offset: usize) -> Option<char> {
        match self.peek_nth(offset)?.kind {
            TokenKind::Symbol(symbol) => Some(symbol),
            _ => None,
        }
    }

    pub fn peek_punct(&self, offset: usize) -> Option<Punct> {
        let symbol = self.peek_symbol(offset)?;
        PunctLexeme::try_from(symbol).ok().map(Punct::from)
    }

    pub fn is_punct(&self, punct: Punct) -> bool {
        self.peek_punct(0) == Some(punct)
    }

    pub fn eat_punct(&mut self, punct: Punct) -> bool {
        let found = self.is_punct(punct);
        if found {
            self.index += 1;
        }
        found
    }

    pub fn expect_punct(&mut self, punct: Punct, what: &str) -> Result<&'a Token> {
        if self.is_punct(punct) {
            return Ok(self.bump().expect("token checked above"));
        }
        Err(self.expected(what))
    }

    pub fn peek_word(&self, offset: usize) -> Option<&'a str> {
        match &self.peek_nth(offset)?.kind {
            TokenKind::Identifier(word) => Some(word),
            _ => None,
        }
    }

    pub fn peek_keyword(&self, offset: usize) -> Option<Keyword> {
        let word = self.peek_word(offset)?;
        word.parse::<KeywordLexeme>().ok().map(Keyword::from)
    }

    pub fn is_keyword(&self, keyword: Keyword) -> bool {
        self.peek_keyword(0) == Some(keyword)
    }

    pub fn eat_keyword(&mut self, keyword: Keyword) -> bool {
        let found = self.is_keyword(keyword);
        if found {
            self.index += 1;
        }
        found
    }

    pub fn peek_string(&self, offset: usize) -> Option<&'a str> {
        match &self.peek_nth(offset)?.kind {
            TokenKind::String(value) => Some(value),
            _ => None,
        }
    }

    pub fn expect_ident(&mut self, what: &str) -> Result<(String, usize)> {
        match self.peek() {
            Some(Token {
                kind: TokenKind::Identifier(word),
                span,
            }) => {
                let found = (word.clone(), span.line);
                self.index += 1;
                Ok(found)
            }
            _ => Err(self.expected(what)),
        }
    }

    /// Consume a statement terminator. A missing `;` is accepted when the
    /// next token closes a block or begins on a later line.
    pub fn end_statement(&mut self) -> Result<()> {
        if self.eat_punct(Punct::Semicolon) {
            return Ok(());
        }
        let ends = match self.peek() {
            None => true,
            Some(next) => {
                self.is_punct(Punct::RightBrace) || next.line() > self.previous_end_line()
            }
        };
        if ends {
            Ok(())
        } else {
            Err(self.expected("`;`"))
        }
    }
}
