mod imports;
mod lexer;
mod parser;
mod tests;

pub use lexer::{Token, TokenKind, tokenize};
pub use tests::TestSummary;

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
