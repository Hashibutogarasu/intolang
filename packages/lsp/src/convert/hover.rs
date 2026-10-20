use lsp_types::{MarkupContent, MarkupKind};

use crate::documents::Documents;

pub fn hover(documents: &Documents, uri: &str, found: ipc::Hover) -> lsp_types::Hover {
    let mut markdown = format!("```into\n{}\n```", found.signature);
    if let Some(documentation) = found.documentation {
        markdown.push_str("\n\n");
        markdown.push_str(&documentation.markdown);
    }
    lsp_types::Hover {
        contents: lsp_types::HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: markdown,
        }),
        range: Some(documents.range(uri, found.range)),
    }
}
