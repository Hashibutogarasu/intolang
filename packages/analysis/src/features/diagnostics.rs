use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

use ast::Program;
use errors::{Error, Severity as ErrorSeverity};
use ipc::{Diagnostic, Location, RelatedDiagnostic, Severity};
use types::Span;

use crate::text::LineIndex;

pub struct Source<'a> {
    pub uri: &'a str,
    pub path: Option<&'a Path>,
    pub text: &'a str,
    pub line_index: &'a LineIndex,
    pub packages_dir: Option<&'a Path>,
}

const TESTS_DIRECTORY: &str = "/src/tests/";

/// Everything wrong with the document: its syntax errors, imports that name
/// nothing, then the errors of lowering the program.
pub fn diagnose(source: &Source<'_>) -> Vec<Diagnostic> {
    let label = source.path.map(|path| path.display().to_string());
    let (program, syntax_errors) = Program::parse_all(source.text);
    if !syntax_errors.is_empty() {
        return syntax_errors
            .into_iter()
            .map(|error| {
                let error = match &label {
                    Some(label) => error.bound_to(label),
                    None => error,
                };
                located(&error, source, label.as_deref())
            })
            .collect();
    }
    let mut diagnostics = Vec::new();
    if let Some(packages_dir) = source.packages_dir {
        for (decl, error) in moduler::unresolved_imports(&program, packages_dir) {
            diagnostics.push(Diagnostic {
                range: source.line_index.range(decl.uri_span.0),
                ..plain(&error)
            });
        }
    }
    let imports_resolvable = program.imports().next().is_none()
        || (source.packages_dir.is_some() && diagnostics.is_empty());
    let Some(path) = source.path else {
        return diagnostics;
    };
    if !imports_resolvable || path.to_string_lossy().contains(TESTS_DIRECTORY) {
        return diagnostics;
    }
    let lowered = catch_unwind(AssertUnwindSafe(|| {
        backend::lower_file_all(source.text, path, source.packages_dir)
    }));
    if let Ok(Err(errors)) = lowered {
        diagnostics.extend(
            errors
                .iter()
                .map(|error| lowering(error, source, label.as_deref())),
        );
    }
    diagnostics
}

pub(super) fn plain(error: &Error) -> Diagnostic {
    let context = error.resolved_context();
    Diagnostic {
        range: Default::default(),
        severity: match context.severity {
            ErrorSeverity::Error => Severity::Error,
            ErrorSeverity::Warning => Severity::Warning,
            ErrorSeverity::Hint => Severity::Hint,
        },
        code: error.code(),
        message: context.message,
        related: Vec::new(),
        expected: Vec::new(),
        found: None,
        hint: None,
        unnecessary: false,
    }
}

fn located(error: &Error, source: &Source<'_>, label: Option<&str>) -> Diagnostic {
    let context = error.resolved_context();
    let range = match context.span {
        Some(span) if span.start != span.end => source.line_index.range(Span {
            line: span.line,
            end_line: span.end_line,
            start: span.start,
            end: span.end,
        }),
        Some(span) => source.line_index.line_range(span.line),
        None => source.line_index.line_range(1),
    };
    let related = context
        .related
        .iter()
        .filter(|related| {
            related
                .path
                .as_deref()
                .is_none_or(|path| Some(path) == label)
        })
        .filter_map(|related| {
            let span = related.span?;
            Some(RelatedDiagnostic {
                location: Location {
                    uri: source.uri.to_owned(),
                    range: source.line_index.line_range(span.line),
                },
                message: related.message.clone(),
            })
        })
        .collect();
    let syntax = context.syntax.clone();
    Diagnostic {
        range,
        related,
        expected: syntax
            .as_ref()
            .map(|s| s.expected.clone())
            .unwrap_or_default(),
        found: syntax.as_ref().and_then(|s| s.found.clone()),
        hint: syntax.and_then(|s| s.hint),
        ..plain(error)
    }
}

fn lowering(error: &Error, source: &Source<'_>, label: Option<&str>) -> Diagnostic {
    if error.trace().into.is_none() {
        return located(error, source, label);
    }
    let frame = error
        .trace()
        .into
        .as_ref()
        .and_then(|trace| trace.frames.first());
    let line = frame.map_or(1, |frame| frame.line.max(1));
    let mut diagnostic = Diagnostic {
        range: source.line_index.line_range(line),
        ..plain(error)
    };
    if let Some(frame) = frame.filter(|frame| Some(frame.path.as_str()) != label) {
        diagnostic.message = format!("{} ({}:{})", diagnostic.message, frame.path, frame.line);
    }
    diagnostic
}
