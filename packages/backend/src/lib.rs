mod formatter;
mod imports;
mod lexer;
mod library;
mod parser;
mod tests;
mod types;
mod validation;

pub use lexer::{Token, TokenKind, tokenize};
pub use library::NativeFunction;
pub use tests::TestSummary;
pub use types::Type;

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

#[cfg(test)]
mod lowering_tests {
    use super::lower;

    #[test]
    fn lowers_explicit_integer_type() {
        let source = "init {\n    into value: Int32 = 42;\n}\n";
        let output = lower(source).unwrap();
        assert!(
            output.contains("NATIVE_ASSIGN\tvalue\tInt32\tn:42"),
            "{output}"
        );
    }

    #[test]
    fn lowers_int64_and_float_types() {
        let source = "init {\n    into big: Int64 = 5000000000;\n    into ratio: Float = 2.5;\n}\n";
        let output = lower(source).unwrap();
        assert!(
            output.contains("NATIVE_ASSIGN\tbig\tInt64\tn:5000000000"),
            "{output}"
        );
        assert!(
            output.contains("NATIVE_ASSIGN\tratio\tFloat64\tn:2.5"),
            "{output}"
        );
    }

    #[test]
    fn lowers_numeric_while_loop() {
        let source = "init {\n    into index: Int32 = 0;\n    while (index < 10) {\n        index = index + 1;\n    }\n}\n";
        let output = lower(source).unwrap();
        assert!(
            output.contains("NATIVE_WHILE_BEGIN\tindex\t<\tn:10\tInt32"),
            "{output}"
        );
        assert!(
            output.contains("NATIVE_ARITH\tindex\tindex\tadd\tn:1\tInt32"),
            "{output}"
        );
        assert!(output.contains("NATIVE_LOOP_END"), "{output}");
    }

    #[test]
    fn rejects_type_mismatched_arithmetic() {
        let source = "init {\n    into index: Int32 = 0;\n    into total: Int64 = index + 1;\n}\n";
        assert!(lower(source).is_err());
    }
}
