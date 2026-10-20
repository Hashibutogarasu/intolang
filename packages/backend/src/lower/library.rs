use anyhow::Result;
use ast::{Program, SyntaxError};
use moduler::NativeFunction;
use parser::PathLexeme;
use std::path::Path;

/// Lower the public modules selected by a library package's `lib.into` file.
pub fn lower_library(
    source: &str,
    source_path: &Path,
    packages_dir: &Path,
) -> Result<Vec<NativeFunction>> {
    let program = source
        .parse::<Program>()
        .map_err(|error| SyntaxError::bind(error, &PathLexeme::from(source_path).to_string()))?;
    moduler::lower_library(&program, source_path, packages_dir)
}
