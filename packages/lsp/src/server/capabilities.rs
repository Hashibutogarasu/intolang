use lsp_types::{
    CompletionOptions, HoverProviderCapability, OneOf, ServerCapabilities,
    TextDocumentSyncCapability, TextDocumentSyncKind,
};

const TRIGGERS: [&str; 4] = [".", "/", ":", "\""];

pub fn capabilities() -> ServerCapabilities {
    ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(TRIGGERS.map(str::to_owned).to_vec()),
            ..Default::default()
        }),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        definition_provider: Some(OneOf::Left(true)),
        document_symbol_provider: Some(OneOf::Left(true)),
        document_formatting_provider: Some(OneOf::Left(true)),
        ..Default::default()
    }
}
