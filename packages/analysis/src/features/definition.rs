use std::path::{Path, PathBuf};

use ipc::{Location, Position, Range};
use moduler::ImportTarget;
use package::MANIFEST_FILE;

use crate::analyzer::Analysis;
use crate::foreign::locate_symbol;
use crate::index::{ModulePart, NativeRef, Resolved, resolve};
use crate::text::{LineIndex, path_to_uri};
use crate::workspace::{SOURCE_EXTENSION, SOURCE_ROOT, real};

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
        Some(Resolved::Keyword { target, .. }) => vec![Location {
            uri: uri.to_owned(),
            range: line_index.range(target),
        }],
        Some(Resolved::Module { target, part, .. }) => module_locations(&target, &part),
        None => Vec::new(),
    }
}

fn start_of(path: &Path) -> Location {
    Location {
        uri: path_to_uri(&real(path)),
        range: Range::new(Position::new(0, 0), Position::new(0, 0)),
    }
}

fn module_locations(target: &ImportTarget, part: &ModulePart) -> Vec<Location> {
    match part {
        ModulePart::Whole => target.file.iter().map(|file| start_of(file)).collect(),
        ModulePart::Scheme => vec![start_of(&target.package.root.join(MANIFEST_FILE))],
        ModulePart::Path { prefix } => {
            let sources = target.package.root.join(SOURCE_ROOT);
            let mut path = prefix.as_str();
            loop {
                let file = sources.join(format!("{path}.{SOURCE_EXTENSION}"));
                if package::is_file(&file) {
                    return vec![start_of(&file)];
                }
                let directory = sources.join(path);
                if package::is_dir(&directory) {
                    return vec![start_of(&directory)];
                }
                match path.rsplit_once('/') {
                    Some((parent, _)) => path = parent,
                    None => return target.file.iter().map(|file| start_of(file)).collect(),
                }
            }
        }
    }
}

fn native_definition(root: &PathBuf, native: &NativeRef) -> Option<Location> {
    let file = root.join(&native.source_path);
    let text = package::read_to_string(&file).ok()?;
    Some(Location {
        uri: path_to_uri(&real(&file)),
        range: locate_symbol(&text, &native.target).unwrap_or_default(),
    })
}
