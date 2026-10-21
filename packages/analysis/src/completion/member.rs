use ipc::CompletionItem;

use super::context::{CompletionContext, Site};
use super::provider::CompletionProvider;
use super::symbols::symbol_item;
use crate::index::Scope;

pub struct MemberProvider;

impl CompletionProvider for MemberProvider {
    fn candidates(&self, context: &CompletionContext<'_>) -> Vec<CompletionItem> {
        let (Site::Member { receiver }, Some(analysis)) = (&context.site, context.analysis) else {
            return Vec::new();
        };
        let scope = Scope::new(analysis);
        let Some(class) = scope.type_of_path(receiver, context.line) else {
            return Vec::new();
        };
        scope
            .members_of(&class)
            .into_iter()
            .map(|(_, symbol)| symbol_item(context, symbol))
            .collect()
    }
}
