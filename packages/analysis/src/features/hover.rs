use std::path::Path;

use ipc::{Documentation, Hover};

use crate::analyzer::Analysis;
use crate::index::{Resolved, resolve};
use crate::text::LineIndex;

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
        Resolved::Module { path, token } => Some(Hover {
            signature: format!("module {}", path.display()),
            documentation: None,
            range: line_index.range(token),
        }),
    }
}
