use clap::ValueEnum;
use errors::ReportFormat;

/// How a failure is written: for people, or as JSON for tools.
#[derive(Clone, Copy, Default, ValueEnum)]
pub(super) enum ErrorFormat {
    #[default]
    Human,
    Json,
}

impl From<ErrorFormat> for ReportFormat {
    fn from(format: ErrorFormat) -> Self {
        match format {
            ErrorFormat::Human => Self::Human,
            ErrorFormat::Json => Self::Json,
        }
    }
}
