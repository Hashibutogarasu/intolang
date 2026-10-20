use super::ErrorReport;
use crate::error::Error;

/// The notes and the message, the layers of the stack trace and the code of the error.
pub struct HumanReport;

impl ErrorReport for HumanReport {
    fn report(&self, error: &Error) -> String {
        let mut text = String::new();
        for note in error.notes().iter().rev() {
            text.push_str(note);
            text.push_str(": ");
        }
        text.push_str(&error.kind().to_string());
        let trace = error.trace().render();
        if !trace.is_empty() {
            text.push_str("\n\n");
            text.push_str(&trace);
        }
        if let Some(code) = error.code() {
            text.push_str(&format!("\n\nerror code: {code:#06x}"));
        }
        text
    }
}
