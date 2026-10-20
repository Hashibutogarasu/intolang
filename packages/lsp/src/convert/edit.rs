use crate::documents::Documents;

pub fn edit(documents: &Documents, uri: &str, found: ipc::TextEdit) -> lsp_types::TextEdit {
    lsp_types::TextEdit {
        range: documents.range(uri, found.range),
        new_text: found.text,
    }
}
