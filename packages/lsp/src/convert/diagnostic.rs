use lsp_types::{DiagnosticRelatedInformation, DiagnosticSeverity, Location, NumberOrString};

use super::uri::uri;
use crate::documents::Documents;

const SOURCE: &str = "intolang";

pub fn diagnostic(
    documents: &Documents,
    uri_text: &str,
    found: ipc::Diagnostic,
) -> lsp_types::Diagnostic {
    let related: Vec<DiagnosticRelatedInformation> = found
        .related
        .iter()
        .filter_map(|related| {
            Some(DiagnosticRelatedInformation {
                location: Location {
                    uri: uri(&related.location.uri)?,
                    range: documents.range(&related.location.uri, related.location.range),
                },
                message: related.message.clone(),
            })
        })
        .collect();
    let message = match &found.hint {
        Some(hint) => format!("{}\n{hint}", found.message),
        None => found.message.clone(),
    };
    lsp_types::Diagnostic {
        range: documents.range(uri_text, found.range),
        severity: Some(match found.severity {
            ipc::Severity::Error => DiagnosticSeverity::ERROR,
            ipc::Severity::Warning => DiagnosticSeverity::WARNING,
        }),
        code: found
            .code
            .map(|code| NumberOrString::String(format!("0x{code:04X}"))),
        source: Some(SOURCE.to_owned()),
        message,
        related_information: (!related.is_empty()).then_some(related),
        ..Default::default()
    }
}
