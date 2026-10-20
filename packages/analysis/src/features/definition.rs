use std::path::Path;

use ipc::{Location, Position, Range};

use crate::analyzer::Analysis;
use crate::index::{Resolved, resolve};
use crate::text::{LineIndex, path_to_uri};
use crate::workspace::real;

pub fn definition(
    analysis: &Analysis,
    uri: &str,
    line_index: &LineIndex,
    offset: usize,
    packages_dir: Option<&Path>,
) -> Vec<Location> {
    match resolve(analysis, offset, packages_dir) {
        Some(Resolved::Symbol {
            module: None,
            symbol,
            ..
        }) => vec![Location {
            uri: uri.to_owned(),
            range: line_index.range(symbol.name_span),
        }],
        Some(Resolved::Symbol {
            module: Some(number),
            symbol,
            ..
        }) => {
            let module = &analysis.modules[number];
            vec![Location {
                uri: path_to_uri(&module.path),
                range: module.line_index.range(symbol.name_span),
            }]
        }
        Some(Resolved::Module { target, .. }) => target
            .file
            .iter()
            .map(|file| Location {
                uri: path_to_uri(&real(file)),
                range: Range::new(Position::new(0, 0), Position::new(0, 0)),
            })
            .collect(),
        None => Vec::new(),
    }
}
