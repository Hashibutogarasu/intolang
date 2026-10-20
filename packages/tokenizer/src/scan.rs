use errors::{Errors, Result};
use parser::{CharLexeme, CommentLexeme, IdentLexeme, NumberLexeme, StringLexeme, SymbolLexeme};
use types::{Span, Token};

/// Split Into source into tokens, skipping whitespace. `//` comments are
/// skipped too unless `keep_comments` is set.
pub(crate) fn scan(source: &str, keep_comments: bool) -> Result<Vec<Token>> {
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
            let start = index;
            while index < chars.len() && chars[index] != '\n' {
                index += 1;
            }
            if keep_comments {
                let text: String = chars[start..index].iter().collect();
                let text = text.trim_end_matches('\r');
                let span = Span {
                    line,
                    end_line: line,
                    start,
                    end: start + text.chars().count(),
                };
                tokens.push(text.parse::<CommentLexeme>()?.with_span(span).into());
            }
            continue;
        }
        if ch == '"' {
            let start = index;
            let start_line = line;
            let body_start = index + 1;
            index = body_start;
            let mut closed = false;
            while index < chars.len() {
                match chars[index] {
                    '"' => {
                        closed = true;
                        break;
                    }
                    '\\' => {
                        if index + 1 >= chars.len() {
                            index += 1;
                            break;
                        }
                        if chars[index + 1] == '\n' {
                            line += 1;
                        }
                        index += 2;
                    }
                    '\n' => {
                        line += 1;
                        index += 1;
                    }
                    _ => index += 1,
                }
            }
            if !closed {
                return Err(Errors::UnterminatedString { line: start_line }.into());
            }
            let raw: String = chars[body_start..index].iter().collect();
            index += 1;
            let span = Span {
                line: start_line,
                end_line: line,
                start,
                end: index,
            };
            tokens.push(raw.parse::<StringLexeme>()?.with_span(span).into());
            continue;
        }
        if ch == '\'' {
            let start = index;
            let mut end = index + 1;
            while end < chars.len() && chars[end] != '\'' && chars[end] != '\n' {
                end += if chars[end] == '\\' { 2 } else { 1 };
            }
            if chars.get(end) != Some(&'\'') {
                return Err(Errors::UnterminatedChar { line }.into());
            }
            let raw: String = chars[start + 1..end].iter().collect();
            let span = Span {
                line,
                end_line: line,
                start,
                end: end + 1,
            };
            tokens.push(raw.parse::<CharLexeme>()?.with_span(span).into());
            index = end + 1;
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
            let text: String = chars[start..index].iter().collect();
            let span = Span {
                line,
                end_line: line,
                start,
                end: index,
            };
            tokens.push(text.parse::<IdentLexeme>()?.with_span(span).into());
            continue;
        }
        if ch.is_ascii_digit() {
            let start = index;
            index += 1;
            while index < chars.len() && chars[index].is_ascii_digit() {
                index += 1;
            }
            let text: String = chars[start..index].iter().collect();
            let span = Span {
                line,
                end_line: line,
                start,
                end: index,
            };
            tokens.push(text.parse::<NumberLexeme>()?.with_span(span).into());
            continue;
        }
        let span = Span {
            line,
            end_line: line,
            start: index,
            end: index + 1,
        };
        tokens.push(SymbolLexeme::from(ch).with_span(span).into());
        index += 1;
    }
    Ok(tokens)
}
