use types::{Span, TokenKind};

use super::symbol::Symbol;
use crate::analyzer::Analysis;

/// The spans of the names that refer to the local `symbol`, besides its
/// declaration.
pub fn uses(analysis: &Analysis, symbol: &Symbol) -> Vec<Span> {
    let tokens = &analysis.tokens;
    tokens
        .iter()
        .enumerate()
        .filter(|(position, token)| {
            let TokenKind::Identifier(word) = &token.kind else {
                return false;
            };
            *word == symbol.name
                && token.span != symbol.name_span
                && (*position == 0 || tokens[position - 1].kind != TokenKind::Symbol('.'))
                && analysis
                    .index
                    .local(word, token.span.line)
                    .is_some_and(|found| found.name_span == symbol.name_span)
        })
        .map(|(_, token)| token.span)
        .collect()
}
