use ipc::{CompletionItem, CompletionKind};
use parser::Registry;

use super::context::{CompletionContext, Site};
use super::provider::CompletionProvider;

pub struct KeywordProvider;

impl CompletionProvider for KeywordProvider {
    fn candidates(&self, context: &CompletionContext<'_>) -> Vec<CompletionItem> {
        if !matches!(context.site, Site::Word { .. }) {
            return Vec::new();
        }
        let registry = Registry::global();
        let keywords = registry.keywords().iter().map(|definition| {
            (
                definition.spelling,
                definition.detail,
                CompletionKind::Keyword,
            )
        });
        let values = registry.values().iter().map(|definition| {
            (
                definition.spelling,
                definition.detail,
                CompletionKind::Variable,
            )
        });
        keywords
            .chain(values)
            .map(|(spelling, detail, kind)| CompletionItem {
                detail: Some(detail.to_owned()),
                ..context.item(spelling, kind)
            })
            .collect()
    }
}
