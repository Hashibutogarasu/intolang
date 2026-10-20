use std::backtrace::Backtrace;
use std::panic::{self, PanicHookInfo};
use std::sync::Mutex;

use super::{Frame, Layer, StackTrace};

static LAST_PANIC: Mutex<Option<PanicStackTrace>> = Mutex::new(None);

/// The raw panic information: the message, where it happened and the backtrace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanicStackTrace {
    pub message: String,
    pub location: Option<String>,
    pub backtrace: String,
}

impl PanicStackTrace {
    fn from_hook(info: &PanicHookInfo<'_>) -> Self {
        let payload = info.payload();
        let message = payload
            .downcast_ref::<&str>()
            .map(|text| text.to_string())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "Box<dyn Any>".to_string());
        Self {
            message,
            location: info
                .location()
                .map(|place| format!("{}:{}:{}", place.file(), place.line(), place.column())),
            backtrace: Backtrace::capture().to_string(),
        }
    }
}

impl StackTrace for PanicStackTrace {
    fn layer(&self) -> Layer {
        Layer::Panic
    }

    fn frames(&self) -> Vec<Frame> {
        match &self.location {
            Some(location) => vec![Frame::at(&self.message, location)],
            None => vec![Frame::new(&self.message)],
        }
    }

    fn detail(&self) -> Option<String> {
        let backtrace = self.backtrace.trim();
        (!backtrace.is_empty() && backtrace != "disabled backtrace").then(|| self.backtrace.clone())
    }
}

/// Keep the raw panic information of the last panic and print it as the last layer
/// of a stack trace.
pub fn install_panic_hook() {
    panic::set_hook(Box::new(|info| {
        let trace = PanicStackTrace::from_hook(info);
        eprintln!("{}", trace.render());
        if let Ok(mut last) = LAST_PANIC.lock() {
            *last = Some(trace);
        }
    }));
}

/// The panic the hook kept last, if any.
pub fn take_panic() -> Option<PanicStackTrace> {
    LAST_PANIC.lock().ok().and_then(|mut last| last.take())
}
