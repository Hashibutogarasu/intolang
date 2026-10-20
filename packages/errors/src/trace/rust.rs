use std::backtrace::{Backtrace, BacktraceStatus};

use super::{Frame, Layer, StackTrace};

/// The Rust backtrace of the place the error was made, captured as `RUST_BACKTRACE` allows.
#[derive(Debug)]
pub struct RustStackTrace {
    backtrace: Backtrace,
}

impl RustStackTrace {
    pub fn capture() -> Self {
        Self {
            backtrace: Backtrace::capture(),
        }
    }

    pub fn is_captured(&self) -> bool {
        self.backtrace.status() == BacktraceStatus::Captured
    }
}

impl StackTrace for RustStackTrace {
    fn layer(&self) -> Layer {
        Layer::Rust
    }

    fn frames(&self) -> Vec<Frame> {
        Vec::new()
    }

    fn detail(&self) -> Option<String> {
        self.is_captured().then(|| self.backtrace.to_string())
    }
}
