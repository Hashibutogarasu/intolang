use serde::{Deserialize, Serialize};

use super::{IntoLangStackTrace, NativeStackTrace, PanicStackTrace, RustStackTrace};

/// The layers of a stack trace, in the order they are printed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Layer {
    Into,
    Rust,
    Native,
    Panic,
}

impl Layer {
    pub const fn title(self) -> &'static str {
        match self {
            Self::Into => "Into language backtrace:",
            Self::Rust => "Rust backtrace:",
            Self::Native => "Native backtrace:",
            Self::Panic => "Panic:",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Frame {
    pub label: String,
    pub location: Option<String>,
}

impl Frame {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            location: None,
        }
    }

    pub fn at(label: impl Into<String>, location: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            location: Some(location.into()),
        }
    }
}

/// One layer of the stack trace of an error.
pub trait StackTrace {
    fn layer(&self) -> Layer;

    fn frames(&self) -> Vec<Frame>;

    /// Text that does not fit a frame: captured output, a panic message, a Rust backtrace.
    fn detail(&self) -> Option<String> {
        None
    }

    fn is_empty(&self) -> bool {
        self.frames().is_empty() && self.detail().is_none()
    }

    fn render(&self) -> String {
        let mut text = self.layer().title().to_string();
        for (index, frame) in self.frames().iter().enumerate() {
            text.push_str(&format!("\n  {index}: {}", frame.label));
            if let Some(location) = &frame.location {
                text.push_str(&format!(" at {location}"));
            }
        }
        if let Some(detail) = self.detail() {
            for line in detail.lines() {
                text.push_str(&format!("\n  {line}"));
            }
        }
        text
    }
}

/// The four layers of an error: Into language, Rust, native and the raw panic.
#[derive(Debug, Default)]
pub struct StackTraces {
    pub into: Option<IntoLangStackTrace>,
    pub rust: Option<RustStackTrace>,
    pub native: Option<NativeStackTrace>,
    pub panic: Option<PanicStackTrace>,
}

impl StackTraces {
    /// The layers that hold something, Into language first and the raw panic last.
    pub fn layers(&self) -> Vec<&dyn StackTrace> {
        let layers: [Option<&dyn StackTrace>; 4] = [
            self.into.as_ref().map(|layer| layer as &dyn StackTrace),
            self.rust.as_ref().map(|layer| layer as &dyn StackTrace),
            self.native.as_ref().map(|layer| layer as &dyn StackTrace),
            self.panic.as_ref().map(|layer| layer as &dyn StackTrace),
        ];
        layers
            .into_iter()
            .flatten()
            .filter(|layer| !layer.is_empty())
            .collect()
    }

    pub fn render(&self) -> String {
        self.layers()
            .iter()
            .map(|layer| layer.render())
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}
