use std::path::{Path, PathBuf};

use ipc::{Location, Position, Range};

use crate::analyzer::Analysis;
use crate::foreign::locate_symbol;
use crate::index::{NativeRef, Resolved, resolve};
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
            token,
        }) if symbol.name_span == token && symbol.native.is_some() => {
            let native = symbol.native.as_ref().expect("checked above");
            analysis
                .root
                .iter()
                .flat_map(|root| native_definition(root, native))
                .collect()
        }
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
        Some(Resolved::Native { native, root, .. }) => {
            native_definition(&root, native).into_iter().collect()
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

fn native_definition(root: &PathBuf, native: &NativeRef) -> Option<Location> {
    let file = root.join(&native.source_path);
    let text = std::fs::read_to_string(&file).ok()?;
    Some(Location {
        uri: path_to_uri(&real(&file)),
        range: locate_symbol(&text, &native.target).unwrap_or_default(),
    })
}
