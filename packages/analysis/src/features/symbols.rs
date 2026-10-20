use ipc::Symbol;

use crate::analyzer::Analysis;
use crate::text::LineIndex;

pub fn document_symbols(analysis: &Analysis, line_index: &LineIndex) -> Vec<Symbol> {
    analysis
        .index
        .symbols
        .iter()
        .filter(|symbol| !symbol.local)
        .map(|symbol| {
            let range = line_index.range(symbol.name_span);
            Symbol {
                name: symbol.name.clone(),
                kind: symbol.kind,
                container: symbol.container.clone(),
                range,
                name_range: range,
            }
        })
        .collect()
}
