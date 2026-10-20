use lsp_types::{SemanticToken, SemanticTokenModifier, SemanticTokenType, SemanticTokens};

use crate::documents::Documents;

pub const TOKEN_TYPE: SemanticTokenType = SemanticTokenType::VARIABLE;
const REFERENCE: &str = "reference";

pub fn token_modifier() -> SemanticTokenModifier {
    SemanticTokenModifier::new(REFERENCE)
}

/// The names that own nothing as variables with the `reference` modifier, in
/// the relative encoding of the protocol.
pub fn non_owning(documents: &Documents, uri: &str, found: Vec<ipc::Range>) -> SemanticTokens {
    let mut ranges: Vec<lsp_types::Range> = found
        .into_iter()
        .map(|range| documents.range(uri, range))
        .filter(|range| range.start.line == range.end.line)
        .collect();
    ranges.sort_by_key(|range| (range.start.line, range.start.character));
    let mut line = 0;
    let mut character = 0;
    let data = ranges
        .into_iter()
        .map(|range| {
            let delta_line = range.start.line - line;
            let delta_start = match delta_line {
                0 => range.start.character - character,
                _ => range.start.character,
            };
            line = range.start.line;
            character = range.start.character;
            SemanticToken {
                delta_line,
                delta_start,
                length: range.end.character - range.start.character,
                token_type: 0,
                token_modifiers_bitset: 1,
            }
        })
        .collect();
    SemanticTokens {
        result_id: None,
        data,
    }
}
