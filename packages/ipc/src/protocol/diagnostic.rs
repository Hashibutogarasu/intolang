use serde::{Deserialize, Serialize};

use super::position::Location;
use super::position::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelatedDiagnostic {
    pub location: Location,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub range: Range,
    pub severity: Severity,
    pub code: Option<u16>,
    pub message: String,
    #[serde(default)]
    pub related: Vec<RelatedDiagnostic>,
    #[serde(default)]
    pub expected: Vec<String>,
    #[serde(default)]
    pub found: Option<String>,
    #[serde(default)]
    pub hint: Option<String>,
}
