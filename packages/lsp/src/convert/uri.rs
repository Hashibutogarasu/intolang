use std::str::FromStr;

pub fn uri(text: &str) -> Option<lsp_types::Uri> {
    lsp_types::Uri::from_str(text).ok()
}
