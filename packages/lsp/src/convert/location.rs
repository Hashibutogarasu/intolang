use super::uri::uri;
use crate::documents::Documents;

pub fn location(documents: &Documents, found: &ipc::Location) -> Option<lsp_types::Location> {
    Some(lsp_types::Location {
        uri: uri(&found.uri)?,
        range: documents.range(&found.uri, found.range),
    })
}
