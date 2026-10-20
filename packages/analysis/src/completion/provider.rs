use ipc::CompletionItem;

use super::context::CompletionContext;

pub trait CompletionProvider {
    /// What this provider offers at the cursor; empty when it does not apply.
    fn candidates(&self, context: &CompletionContext<'_>) -> Vec<CompletionItem>;
}
