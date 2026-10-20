use super::ErrorReport;
use crate::error::Error;

/// The notes and the message, the layers of the stack trace and the code of the error.
pub struct HumanReport;

impl HumanReport {
    /// The notes and the message of the error on one line.
    pub fn headline(error: &Error) -> String {
        let mut text = String::new();
        for note in error.notes().iter().rev() {
            text.push_str(note);
            text.push_str(": ");
        }
        text.push_str(&error.kind().to_string());
        text
    }

    /// The layers of the stack trace and the code, each after a blank line.
    pub fn detail(error: &Error) -> String {
        let mut text = String::new();
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

impl ErrorReport for HumanReport {
    fn report(&self, error: &Error) -> String {
        format!("{}{}", Self::headline(error), Self::detail(error))
    }

    fn emit_with(&self, error: &Error, headline: &dyn Fn(&str)) {
        headline(&Self::headline(error));
        let detail = Self::detail(error);
        if !detail.is_empty() {
            eprintln!("{}", detail.trim_start());
        }
    }
}
