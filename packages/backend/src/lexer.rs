use anyhow::{Result, bail};

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Identifier(String),
    Number(String),
    String(String),
    Symbol(char),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
}

/// Lex Into source, including escaped strings and `//` comments.
pub fn tokenize(source: &str) -> Result<Vec<Token>> {
    let chars: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let (mut index, mut line) = (0, 1);

    while index < chars.len() {
        let ch = chars[index];
        if ch.is_whitespace() {
            if ch == '\n' {
                line += 1;
            }
            index += 1;
            continue;
        }
        if ch == '/' && chars.get(index + 1) == Some(&'/') {
            while index < chars.len() && chars[index] != '\n' {
                index += 1;
            }
            continue;
        }
        if ch == '"' {
            let start_line = line;
            let mut value = String::new();
            index += 1;
            let mut closed = false;
            while index < chars.len() {
                match chars[index] {
                    '"' => {
                        closed = true;
                        index += 1;
                        break;
                    }
                    '\\' => {
                        index += 1;
                        match chars.get(index).copied() {
                            Some('n') => value.push('\n'),
                            Some('t') => value.push('\t'),
                            Some('"') => value.push('"'),
                            Some('\\') => value.push('\\'),
                            Some(other) => {
                                value.push('\\');
                                value.push(other);
                            }
                            None => break,
                        }
                        index += 1;
                    }
                    '\n' => {
                        value.push('\n');
                        line += 1;
                        index += 1;
                    }
                    c => {
                        value.push(c);
                        index += 1;
                    }
                }
            }
            if !closed {
                bail!("Unterminated string at line {start_line}");
            }
            tokens.push(Token {
                kind: TokenKind::String(value),
                line: start_line,
            });
            continue;
        }
        if ch.is_ascii_alphabetic() || ch == '_' {
            let start = index;
            index += 1;
            while index < chars.len()
                && (chars[index].is_ascii_alphanumeric() || chars[index] == '_')
            {
                index += 1;
            }
            tokens.push(Token {
                kind: TokenKind::Identifier(chars[start..index].iter().collect()),
                line,
            });
            continue;
        }
        if ch.is_ascii_digit() {
            let start = index;
            index += 1;
            while index < chars.len() && chars[index].is_ascii_digit() {
                index += 1;
            }
            tokens.push(Token {
                kind: TokenKind::Number(chars[start..index].iter().collect()),
                line,
            });
            continue;
        }
        tokens.push(Token {
            kind: TokenKind::Symbol(ch),
            line,
        });
        index += 1;
    }
    Ok(tokens)
}
