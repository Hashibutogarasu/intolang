use types::{Keyword, Punct};

use crate::piece::{Kind, Piece};

/// Keywords that are followed by a space even before a parenthesis.
fn spaces_before_paren(keyword: Keyword) -> bool {
    matches!(keyword, Keyword::If | Keyword::While | Keyword::Else)
}

/// Whether the source separated `prev` and `cur` by whitespace or a line
/// break. Used where the spacing is ambiguous (`*`, `<`, `>`, `-`, `/`).
fn had_gap(prev: &Piece, cur: &Piece) -> bool {
    cur.start > prev.end || cur.line > prev.end_line
}

/// Whether a space belongs between two neighbouring pieces of one line.
pub(crate) fn needs_space(prev: &Piece, cur: &Piece) -> bool {
    if matches!(
        cur.kind,
        Kind::Punct(
            Punct::Semicolon
                | Punct::Comma
                | Punct::RightParen
                | Punct::Dot
                | Punct::Colon
                | Punct::RightBracket
        )
    ) {
        return false;
    }
    if matches!(
        prev.kind,
        Kind::Punct(Punct::LeftParen | Punct::Dot | Punct::Hash | Punct::LeftBracket)
    ) {
        return false;
    }
    if matches!(prev.kind, Kind::Punct(Punct::Comma | Punct::Colon))
        || (prev.is(Punct::RightBrace) && cur.is_keyword(Keyword::Else))
    {
        return true;
    }
    if cur.is(Punct::LeftParen) {
        return match prev.kind {
            Kind::Word(keyword) => keyword.is_some_and(spaces_before_paren),
            _ if prev.is_operator() => true,
            _ => had_gap(prev, cur),
        };
    }
    if prev.is_operator() || cur.is_operator() || cur.is(Punct::LeftBrace) {
        return true;
    }
    if cur.is(Punct::LeftBracket) {
        return if prev.is_word_like() {
            false
        } else {
            had_gap(prev, cur)
        };
    }
    if prev.is_word_like() && cur.is_word_like() {
        return true;
    }
    had_gap(prev, cur)
}
