use ipc::CompletionItem;

use super::callback::CallbackProvider;
use super::context::CompletionContext;
use super::import_path::ImportPathProvider;
use super::keyword::KeywordProvider;
use super::member::MemberProvider;
use super::provider::CompletionProvider;
use super::scope::ScopeProvider;

/// The candidates of every provider that begin with what is typed, the first
/// of those with the same label winning.
pub fn complete(context: &CompletionContext<'_>) -> Vec<CompletionItem> {
    let providers: [&dyn CompletionProvider; 5] = [
        &ScopeProvider,
        &MemberProvider,
        &KeywordProvider,
        &ImportPathProvider,
        &CallbackProvider,
    ];
    let prefix = context.prefix.to_lowercase();
    let mut items: Vec<CompletionItem> = Vec::new();
    for item in providers
        .iter()
        .flat_map(|provider| provider.candidates(context))
    {
        let matches = item.snippet || item.label.to_lowercase().starts_with(&prefix);
        if matches && !items.iter().any(|known| known.label == item.label) {
            items.push(item);
        }
    }
    items
}
