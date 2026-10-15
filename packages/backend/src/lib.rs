mod imports;
mod library;
mod formatter;
mod lexer;
mod parser;
mod tests;
mod validation;

pub use lexer::{Token, TokenKind, tokenize};
pub use tests::TestSummary;
pub use library::NativeFunction;

/// Format Into source text using the backend's canonical layout.
pub fn format_source(source: &str) -> String {
    formatter::format_source(source)
}

/// Parse and lower an Into source file to its observable output.
pub fn lower(source: &str) -> anyhow::Result<String> {
    parser::lower(source, None)
}

/// Resolve imports against bundled packages, then parse and lower the source.
pub fn lower_with_packages(
    source: &str,
    packages_dir: Option<&std::path::Path>,
) -> anyhow::Result<String> {
    parser::lower(source, packages_dir)
}

/// Resolve and lower a source file while retaining its path for diagnostics.
pub fn lower_file(
    source: &str,
    source_path: &std::path::Path,
    packages_dir: Option<&std::path::Path>,
) -> anyhow::Result<String> {
    parser::lower_named(source, packages_dir, &source_path.display().to_string())
}

/// Run all `#[test]` functions in one Into source file.
pub fn run_tests(
    source: &str,
    source_path: &std::path::Path,
    packages_dir: &std::path::Path,
) -> anyhow::Result<TestSummary> {
    tests::run(source, source_path, packages_dir)
}

/// Lower the public modules selected by a library package's `lib.into` file.
pub fn lower_library(
    source: &str,
    source_path: &std::path::Path,
    packages_dir: &std::path::Path,
) -> anyhow::Result<Vec<NativeFunction>> {
    library::lower(source, source_path, packages_dir)
}
