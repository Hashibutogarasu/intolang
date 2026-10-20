use std::fmt;

use crate::context::ErrorContext;
use crate::errors::{Domain, Errors};
use crate::report::{ErrorReport, HumanReport, ReportFormat};
use crate::trace::{
    IntoLangStackTrace, NativeStackTrace, PanicStackTrace, RustStackTrace, StackTraces,
};

pub type Result<T> = std::result::Result<T, Error>;

/// An error of the toolchain: what happened, the layers of its stack trace and the
/// context it was found in.
#[derive(Debug)]
pub struct Error {
    kind: Errors,
    trace: StackTraces,
    context: Option<ErrorContext>,
}

impl Error {
    pub fn new(kind: Errors) -> Self {
        Self {
            kind,
            trace: StackTraces {
                rust: Some(RustStackTrace::capture()),
                ..StackTraces::default()
            },
            context: None,
        }
    }

    /// An error of the Into program with the frames it has there.
    pub fn into_lang(kind: Errors, trace: IntoLangStackTrace) -> Self {
        Self::new(kind).with_into(trace)
    }

    /// An error of the toolchain itself. It never has an Into language layer.
    pub fn compiler(kind: Errors) -> Self {
        Self::new(kind)
    }

    /// The error of a panic: the raw panic information is its last layer.
    pub fn from_panic(panic: PanicStackTrace) -> Self {
        let kind = Errors::Panic {
            message: panic.message.clone(),
        };
        Self::new(kind).with_panic(panic)
    }

    /// Add the Into language layer. Errors of the toolchain itself do not go through the
    /// Into language, so they drop it.
    pub fn with_into(mut self, trace: IntoLangStackTrace) -> Self {
        if self.kind.domain() == Domain::IntoLang {
            self.trace.into = Some(trace);
        }
        self
    }

    pub fn with_native(mut self, trace: NativeStackTrace) -> Self {
        self.trace.native = Some(trace);
        self
    }

    pub fn with_panic(mut self, trace: PanicStackTrace) -> Self {
        self.trace.panic = Some(trace);
        self
    }

    pub fn with_context(mut self, context: ErrorContext) -> Self {
        self.context = Some(context);
        self
    }

    pub fn kind(&self) -> &Errors {
        &self.kind
    }

    pub fn domain(&self) -> Domain {
        self.kind.domain()
    }

    pub fn code(&self) -> Option<u16> {
        self.kind.code()
    }

    pub fn trace(&self) -> &StackTraces {
        &self.trace
    }

    pub fn context(&self) -> Option<&ErrorContext> {
        self.context.as_ref()
    }

    /// The context given to the error, or the one its kind implies.
    pub fn resolved_context(&self) -> ErrorContext {
        self.context
            .clone()
            .unwrap_or_else(|| ErrorContext::of(&self.kind))
    }

    pub fn report(&self, format: ReportFormat) -> String {
        format.reporter().report(self)
    }
}

impl From<Errors> for Error {
    fn from(kind: Errors) -> Self {
        Self::new(kind)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&HumanReport.report(self))
    }
}

impl std::error::Error for Error {}
