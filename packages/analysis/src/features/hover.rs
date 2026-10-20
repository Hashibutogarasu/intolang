use std::path::Path;

use ipc::{Documentation, Hover};
use moduler::ImportTarget;

use crate::analyzer::Analysis;
use crate::index::{Resolved, resolve};
use crate::text::LineIndex;
use crate::workspace::real;

pub fn hover(
    analysis: &Analysis,
    line_index: &LineIndex,
    offset: usize,
    packages_dir: Option<&Path>,
) -> Option<Hover> {
    match resolve(analysis, offset, packages_dir)? {
        Resolved::Symbol { symbol, token, .. } => Some(Hover {
            signature: symbol.signature.clone(),
            documentation: symbol
                .docs
                .clone()
                .map(|markdown| Documentation { markdown }),
            range: line_index.range(token),
        }),
        Resolved::Module { target, token } => Some(Hover {
            signature: format!(
                "package {} {} ({})",
                target.package.name, target.package.version, target.package.alias
            ),
            documentation: Some(Documentation {
                markdown: package_details(&target),
            }),
            range: line_index.range(token),
        }),
    }
}

fn package_details(target: &ImportTarget) -> String {
    let package = &target.package;
    let mut lines = vec![
        format!("- alias: `{}`", package.alias),
        format!("- version: {}", package.version),
        format!("- licence: {}", package.licence),
        format!("- location: `{}`", real(&package.root).display()),
    ];
    if let Some(file) = &target.file {
        lines.push(format!("- file: `{}`", real(file).display()));
    }
    lines.join("\n")
}
