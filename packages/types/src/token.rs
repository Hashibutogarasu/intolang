/// Where a token sits in its source text. Offsets count characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub line: usize,
    pub end_line: usize,
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn on_line(line: usize) -> Self {
        Self {
            line,
            end_line: line,
            start: 0,
            end: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Identifier(String),
    Number(String),
    String(String),
    Char(char),
    Symbol(char),
    Comment(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, line: usize) -> Self {
        Self {
            kind,
            span: Span::on_line(line),
        }
    }

    pub fn line(&self) -> usize {
        self.span.line
    }
}
