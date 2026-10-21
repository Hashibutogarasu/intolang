use std::collections::HashSet;
use std::path::Path;

use errors::{Error, Errors};
use ipc::Diagnostic;
use types::{Span, TokenKind};

use super::diagnostics::plain;
use crate::analyzer::Analysis;
use crate::index::uses;
use crate::text::LineIndex;
use crate::workspace::real;

/// The variables and imports that nothing uses, as hints that the editor shows
/// faded.
pub fn unused(
    analysis: &Analysis,
    line_index: &LineIndex,
    packages_dir: Option<&Path>,
) -> Vec<Diagnostic> {
    let mut diagnostics = unused_variables(analysis, line_index);
    diagnostics.extend(unused_imports(analysis, line_index, packages_dir));
    diagnostics
}

fn hint(line_index: &LineIndex, span: Span, kind: Errors) -> Diagnostic {
    Diagnostic {
        range: line_index.range(span),
        unnecessary: true,
        ..plain(&Error::new(kind))
    }
}

fn unused_variables(analysis: &Analysis, line_index: &LineIndex) -> Vec<Diagnostic> {
    analysis
        .index
        .symbols
        .iter()
        .filter(|symbol| symbol.track_use && uses(analysis, symbol).is_empty())
        .map(|symbol| {
            hint(
                line_index,
                symbol.name_span,
                Errors::UnusedVariable {
                    name: symbol.name.clone(),
                },
            )
        })
        .collect()
}

fn unused_imports(
    analysis: &Analysis,
    line_index: &LineIndex,
    packages_dir: Option<&Path>,
) -> Vec<Diagnostic> {
    let Some(packages_dir) = packages_dir else {
        return Vec::new();
    };
    let tokens = &analysis.tokens;
    let mut diagnostics = Vec::new();
    for decl in analysis.program.imports() {
        let Ok(Some(target)) = moduler::import_target(decl, packages_dir) else {
            continue;
        };
        let Some(module) = target.file.as_ref().and_then(|file| {
            analysis
                .modules
                .iter()
                .find(|module| module.path == real(file))
        }) else {
            continue;
        };
        let names: HashSet<&str> = module
            .index
            .symbols
            .iter()
            .filter(|symbol| !symbol.local)
            .map(|symbol| symbol.name.as_str())
            .collect();
        let used = tokens.iter().any(|token| {
            matches!(&token.kind, TokenKind::Identifier(word) if names.contains(word.as_str()))
        });
        if names.is_empty() || used {
            continue;
        }
        let Some(string) = tokens
            .iter()
            .position(|token| token.span == decl.uri_span.0)
        else {
            continue;
        };
        let first = tokens[string.saturating_sub(1)].span;
        let last = match tokens.get(string + 1) {
            Some(next) if next.kind == TokenKind::Symbol(';') => next.span,
            _ => tokens[string].span,
        };
        let span = Span {
            line: first.line,
            end_line: last.end_line,
            start: first.start,
            end: last.end,
        };
        diagnostics.push(hint(
            line_index,
            span,
            Errors::UnusedImport {
                uri: decl.uri.clone(),
            },
        ));
    }
    diagnostics
}
