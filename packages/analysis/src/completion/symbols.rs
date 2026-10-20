use ipc::{CompletionItem, CompletionKind, Documentation, SymbolKind};

use super::context::CompletionContext;
use crate::index::Symbol;

pub fn kind_of(kind: SymbolKind) -> CompletionKind {
    match kind {
        SymbolKind::Function => CompletionKind::Function,
        SymbolKind::Class => CompletionKind::Class,
        SymbolKind::Field => CompletionKind::Field,
        SymbolKind::Enum => CompletionKind::Enum,
        SymbolKind::Variant => CompletionKind::Variant,
        SymbolKind::Event => CompletionKind::Event,
        SymbolKind::Macro => CompletionKind::Macro,
        SymbolKind::Variable => CompletionKind::Variable,
    }
}

pub fn symbol_item(context: &CompletionContext<'_>, symbol: &Symbol) -> CompletionItem {
    CompletionItem {
        detail: Some(symbol.signature.clone()),
        documentation: symbol
            .docs
            .clone()
            .map(|markdown| Documentation { markdown }),
        ..context.item(symbol.name.clone(), kind_of(symbol.kind))
    }
}
