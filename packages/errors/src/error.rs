use std::fmt;

use crate::context::{ErrorContext, SourceSpan};
use crate::errors::{Domain, Errors};
use crate::report::{ErrorReport, HumanReport, ReportFormat};
use crate::trace::{
    IntoLangStackTrace, NativeStackTrace, PanicStackTrace, RustStackTrace, StackTraces,
};

pub type Result<T> = std::result::Result<T, Error>;

/// An error of the toolchain: what happened, the layers of its stack trace and the
/// context it was found in.
#[derive(Debug)]
pub struct Error(Box<Details>);

#[derive(Debug)]
struct Details {
    kind: Errors,
    trace: StackTraces,
    context: Option<ErrorContext>,
    notes: Vec<String>,
}

impl Error {
    pub fn new(kind: Errors) -> Self {
        Self(Box::new(Details {
            kind,
            trace: StackTraces {
                rust: Some(RustStackTrace::capture()),
                ..StackTraces::default()
            },
            context: None,
            notes: Vec::new(),
        }))
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
    /// An Into language layer that is there already is kept: it names the innermost place.
    pub fn with_into(mut self, trace: IntoLangStackTrace) -> Self {
        if self.0.kind.domain() == Domain::IntoLang {
            self.0.trace.into.get_or_insert(trace);
        }
        self
    }

    /// Bind an error of the Into program that knows its line to the source `path` it was found
    /// in: the path joins its context and the Into language layer gets its frame. Other errors,
    /// and errors that are bound already, are left as they are.
    pub fn bound_to(mut self, path: &str) -> Self {
        if self.0.kind.domain() != Domain::IntoLang || self.0.trace.into.is_some() {
            return self;
        }
        let scope = self
            .0
            .context
            .as_ref()
            .and_then(|context| context.syntax.as_ref())
            .map_or_else(|| "module".to_string(), |syntax| syntax.scope.clone());
        let line = match (&self.0.kind, &self.0.context) {
            (Errors::UnterminatedString { line } | Errors::UnterminatedChar { line }, _) => *line,
            (_, Some(context)) => match context.span {
                Some(span) => span.line,
                None => return self,
            },
            _ => return self,
        };
        let mut context = self
            .0
            .context
            .take()
            .unwrap_or_else(|| ErrorContext::of(&self.0.kind));
        context.path = Some(path.to_string());
        context.span.get_or_insert(SourceSpan::on_line(line));
        self.0.trace.into = Some(IntoLangStackTrace::at(scope, path, line));
        self.0.context = Some(context);
        self
    }

    pub fn with_native(mut self, trace: NativeStackTrace) -> Self {
        self.0.trace.native = Some(trace);
        self
    }

    pub fn with_panic(mut self, trace: PanicStackTrace) -> Self {
        self.0.trace.panic = Some(trace);
        self
    }

    pub fn with_context(mut self, context: ErrorContext) -> Self {
        self.0.context = Some(context);
        self
    }

    /// Say what was being done when the error happened. The notes are printed from the
    /// last one added to the first, before the message of the error.
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.0.notes.push(note.into());
        self
    }

    /// The notes in the order they were added, the innermost first.
    pub fn notes(&self) -> &[String] {
        &self.0.notes
    }

    pub fn kind(&self) -> &Errors {
        &self.0.kind
    }

    pub fn domain(&self) -> Domain {
        self.0.kind.domain()
    }

    pub fn code(&self) -> Option<u16> {
        self.0.kind.code()
    }

    pub fn trace(&self) -> &StackTraces {
        &self.0.trace
    }

    pub fn context(&self) -> Option<&ErrorContext> {
        self.0.context.as_ref()
    }

    /// The context given to the error, or the one its kind implies.
    pub fn resolved_context(&self) -> ErrorContext {
        self.0
            .context
            .clone()
            .unwrap_or_else(|| ErrorContext::of(&self.0.kind))
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
