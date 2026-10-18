use anyhow::Result;
use ast::Program;
use moduler::NativeFunction;
use parser::PathLexeme;
use std::path::Path;

use crate::diagnostic::syntax_diagnostic;

/// Lower the public modules selected by a library package's `lib.into` file.
pub fn lower_library(
    source: &str,
    source_path: &Path,
    packages_dir: &Path,
) -> Result<Vec<NativeFunction>> {
    let program = source
        .parse::<Program>()
        .map_err(|error| syntax_diagnostic(&PathLexeme::from(source_path).to_string(), error))?;
    moduler::lower_library(&program, source_path, packages_dir)
}
