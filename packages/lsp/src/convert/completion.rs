use ipc::CompletionKind;
use lsp_types::{
    Command, CompletionItemKind, CompletionTextEdit, Documentation, InsertTextFormat,
    MarkupContent, MarkupKind, TextEdit,
};

use crate::documents::Documents;

const RETRIGGER: &str = "editor.action.triggerSuggest";

pub fn completion(
    documents: &Documents,
    uri: &str,
    found: ipc::CompletionItem,
) -> lsp_types::CompletionItem {
    let continues = found.text.ends_with('/');
    lsp_types::CompletionItem {
        label: found.label,
        kind: Some(kind(found.kind)),
        detail: found.detail,
        documentation: found.documentation.map(|documentation| {
            Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: documentation.markdown,
            })
        }),
        insert_text_format: Some(match found.snippet {
            true => InsertTextFormat::SNIPPET,
            false => InsertTextFormat::PLAIN_TEXT,
        }),
        text_edit: Some(CompletionTextEdit::Edit(TextEdit {
            range: documents.range(uri, found.range),
            new_text: found.text,
        })),
        command: continues.then(|| Command {
            title: "Suggest".to_owned(),
            command: RETRIGGER.to_owned(),
            arguments: None,
        }),
        ..Default::default()
    }
}

fn kind(kind: CompletionKind) -> CompletionItemKind {
    match kind {
        CompletionKind::Keyword => CompletionItemKind::KEYWORD,
        CompletionKind::Function | CompletionKind::Macro => CompletionItemKind::FUNCTION,
        CompletionKind::Class => CompletionItemKind::CLASS,
        CompletionKind::Field => CompletionItemKind::FIELD,
        CompletionKind::Enum => CompletionItemKind::ENUM,
        CompletionKind::Variant => CompletionItemKind::ENUM_MEMBER,
        CompletionKind::Event => CompletionItemKind::EVENT,
        CompletionKind::Variable => CompletionItemKind::VARIABLE,
        CompletionKind::Module => CompletionItemKind::MODULE,
        CompletionKind::Folder => CompletionItemKind::FOLDER,
        CompletionKind::Snippet => CompletionItemKind::SNIPPET,
    }
}
