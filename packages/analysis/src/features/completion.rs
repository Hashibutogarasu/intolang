use ipc::{CompletionItem, Position};
use moduler::SourceProvider;

use super::diagnostics::Source;
use crate::analyzer::Analysis;
use crate::completion::{CompletionContext, complete};

/// The candidates at `position`. The names come from the document with the
/// cursor line blanked, as the line being typed rarely parses; when the rest
/// does not parse either, they come from the last analysis that did.
pub fn completion(
    source: &Source<'_>,
    stored: Option<&Analysis>,
    sources: &dyn SourceProvider,
    position: Position,
) -> Vec<CompletionItem> {
    let line = position.line as usize;
    let Some(text) = source.text.split('\n').nth(line) else {
        return Vec::new();
    };
    let before: String = text.chars().take(position.column as usize).collect();
    let blanked: Vec<String> = source
        .text
        .split('\n')
        .enumerate()
        .map(|(number, text)| match number == line {
            true => text
                .chars()
                .map(|character| if character == '\r' { character } else { ' ' })
                .collect(),
            false => text.to_owned(),
        })
        .collect();
    let fresh = Analysis::build(
        &blanked.join("\n"),
        source.path,
        source.packages_dir,
        sources,
    )
    .ok();
    let context = CompletionContext::locate(
        &before,
        position,
        fresh.as_ref().or(stored),
        source.packages_dir,
    );
    complete(&context)
}
