use serde::Serialize;

use super::ErrorReport;
use crate::context::ErrorContext;
use crate::error::Error;
use crate::trace::{Frame, Layer};

#[derive(Serialize)]
struct JsonLayer {
    layer: Layer,
    frames: Vec<Frame>,
    detail: Option<String>,
}

#[derive(Serialize)]
struct JsonError {
    context: ErrorContext,
    notes: Vec<String>,
    trace: Vec<JsonLayer>,
}

/// The context of the error and its stack trace as one JSON object.
pub struct JsonReport;

impl ErrorReport for JsonReport {
    fn report(&self, error: &Error) -> String {
        let trace = error
            .trace()
            .layers()
            .into_iter()
            .map(|layer| JsonLayer {
                layer: layer.layer(),
                frames: layer.frames(),
                detail: layer.detail(),
            })
            .collect();
        let report = JsonError {
            context: error.resolved_context(),
            notes: error.notes().iter().rev().cloned().collect(),
            trace,
        };
        serde_json::to_string(&report).expect("an error report is plain data")
    }
}
