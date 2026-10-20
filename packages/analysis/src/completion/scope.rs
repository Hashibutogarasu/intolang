use ipc::CompletionItem;

use super::context::{CompletionContext, Site};
use super::provider::CompletionProvider;
use super::symbols::symbol_item;
use crate::index::Scope;

pub struct ScopeProvider;

impl CompletionProvider for ScopeProvider {
    fn candidates(&self, context: &CompletionContext<'_>) -> Vec<CompletionItem> {
        let (Site::Word { .. }, Some(analysis)) = (&context.site, context.analysis) else {
            return Vec::new();
        };
        Scope::new(analysis)
            .visible(context.line)
            .into_iter()
            .map(|(_, symbol)| symbol_item(context, symbol))
            .collect()
    }
}
