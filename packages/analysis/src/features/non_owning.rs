use errors::{Error, Errors};
use ipc::Diagnostic;

use super::diagnostics::plain;
use crate::analyzer::Analysis;
use crate::index::uses;
use crate::text::LineIndex;

/// Every place the name of a binding that owns nothing stands, a `T&` binding
/// or the result of `borrow`, so that editors underline it.
pub fn non_owning(analysis: &Analysis, line_index: &LineIndex) -> Vec<Diagnostic> {
    let mut places: Vec<_> = analysis
        .index
        .symbols
        .iter()
        .filter(|symbol| symbol.reference)
        .flat_map(|symbol| {
            let mut spans = uses(analysis, symbol);
            spans.push(symbol.name_span);
            spans.into_iter().map(|span| (span, symbol.name.clone()))
        })
        .collect();
    places.sort_by_key(|(span, _)| span.start);
    places.dedup_by_key(|(span, _)| *span);
    places
        .into_iter()
        .map(|(span, name)| Diagnostic {
            range: line_index.range(span),
            ..plain(&Error::new(Errors::NotOwning { name }))
        })
        .collect()
}
