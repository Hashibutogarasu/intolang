use super::{HumanReport, JsonReport};
use crate::error::Error;

/// A way to write an error. Both ways report the same error; only the text differs.
pub trait ErrorReport {
    fn report(&self, error: &Error) -> String;

    /// Write the report where this way of reporting belongs. A report for people gives
    /// its first line to `headline`, so the caller can write it in its own line format.
    fn emit_with(&self, error: &Error, headline: &dyn Fn(&str));
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ReportFormat {
    #[default]
    Human,
    Json,
}

impl ReportFormat {
    /// The only place that chooses between the reports.
    pub fn reporter(self) -> Box<dyn ErrorReport> {
        match self {
            Self::Human => Box::new(HumanReport),
            Self::Json => Box::new(JsonReport),
        }
    }
}
