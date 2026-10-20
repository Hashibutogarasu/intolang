use ipc::Range;

use crate::analyzer::Analysis;
use crate::index::uses;
use crate::text::LineIndex;

/// Where the variables that own nothing are named: the declaration and every
/// use of a `T&` binding or of the result of `borrow`.
pub fn non_owning(analysis: &Analysis, line_index: &LineIndex) -> Vec<Range> {
    let mut spans: Vec<_> = analysis
        .index
        .symbols
        .iter()
        .filter(|symbol| symbol.reference)
        .flat_map(|symbol| {
            let mut spans = uses(analysis, symbol);
            spans.push(symbol.name_span);
            spans
        })
        .collect();
    spans.sort_by_key(|span| span.start);
    spans.dedup();
    spans
        .into_iter()
        .map(|span| line_index.range(span))
        .collect()
}
