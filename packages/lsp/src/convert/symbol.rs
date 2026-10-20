use ipc::SymbolKind;
use lsp_types::SymbolInformation;

use super::uri::uri;
use crate::documents::Documents;

#[allow(deprecated)]
pub fn symbol(
    documents: &Documents,
    uri_text: &str,
    found: ipc::Symbol,
) -> Option<SymbolInformation> {
    let kind = match found.kind {
        SymbolKind::Function if found.container.is_some() => lsp_types::SymbolKind::METHOD,
        SymbolKind::Function | SymbolKind::Macro => lsp_types::SymbolKind::FUNCTION,
        SymbolKind::Class => lsp_types::SymbolKind::CLASS,
        SymbolKind::Field => lsp_types::SymbolKind::FIELD,
        SymbolKind::Enum => lsp_types::SymbolKind::ENUM,
        SymbolKind::Variant => lsp_types::SymbolKind::ENUM_MEMBER,
        SymbolKind::Event => lsp_types::SymbolKind::EVENT,
        SymbolKind::Variable => lsp_types::SymbolKind::VARIABLE,
    };
    Some(SymbolInformation {
        name: found.name,
        kind,
        tags: None,
        deprecated: None,
        location: lsp_types::Location {
            uri: uri(uri_text)?,
            range: documents.range(uri_text, found.range),
        },
        container_name: found.container,
    })
}
