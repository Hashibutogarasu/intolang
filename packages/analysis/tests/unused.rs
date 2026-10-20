mod common;

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use common::{position_of, session, session_with};
use ipc::{Diagnostic, Position, Range, Severity};

const URI: &str = "file:///project/src/main/unused.into";

fn hints(diagnostics: Vec<Diagnostic>) -> Vec<Diagnostic> {
    diagnostics
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Hint)
        .collect()
}

fn named(source: &str, needle: &str, occurrence: usize) -> Range {
    let start = position_of(source, needle, occurrence);
    Range::new(
        start,
        Position::new(start.line, start.column + needle.chars().count() as u32),
    )
}

#[test]
fn a_variable_nothing_reads_is_faded() {
    let source = "init {\n    into used = 1;\n    into spare = 2;\n    println(used);\n}\n";
    let found = hints(session().open(URI, source));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].message, "`spare` is never used");
    assert_eq!(found[0].code, Some(0x10AC));
    assert_eq!(found[0].range, named(source, "spare", 0));
    assert!(found[0].unnecessary);
}

#[test]
fn a_shadowing_name_does_not_count_as_a_use_of_the_outer_one() {
    let source = "init {\n    into outer = 1;\n    into other = 2;\n    println(other);\n}\n";
    let found = hints(session().open(URI, source));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].range, named(source, "outer", 0));
}

#[test]
fn the_parameters_of_a_function_with_a_body_and_of_a_lambda_are_checked() {
    let source = "pub fun pick(first: Int32, second: Int32): Int32 {\n    return first;\n}\n\npub fun declared(ignored: Int32): Int32;\n\ninit {\n    into f = (left, right) => {\n        println(left);\n    };\n}\n";
    let found = hints(session().open(URI, source));
    let ranges: Vec<Range> = found.iter().map(|diagnostic| diagnostic.range).collect();
    assert!(ranges.contains(&named(source, "second", 0)), "{found:?}");
    assert!(ranges.contains(&named(source, "right", 0)), "{found:?}");
    assert!(!ranges.contains(&named(source, "ignored", 0)), "{found:?}");
    assert!(!ranges.contains(&named(source, "first", 0)), "{found:?}");
    assert!(!ranges.contains(&named(source, "left", 0)), "{found:?}");
}

#[test]
fn a_member_with_the_same_name_is_not_a_use() {
    let source = "init {\n    into value = 1;\n    into other = 2;\n    println(other.value);\n}\n";
    let found = hints(session().open(URI, source));
    assert!(
        found
            .iter()
            .any(|diagnostic| diagnostic.range == named(source, "value", 0)),
        "{found:?}"
    );
}

fn packages() -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let base = std::env::temp_dir().join(format!(
        "analysis-unused-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&base);
    let package = base.join("util");
    fs::create_dir_all(package.join("src/main")).unwrap();
    fs::write(
        package.join("into-package.json"),
        r#"{"name":"util","alias":"util","version":"0.1.0","LICENCE":"MIT","library":"src/main/lib.into"}"#,
    )
    .unwrap();
    fs::write(
        package.join("src/main/tools.into"),
        "pub fun half(number: Int32): Int32 {\n    return number;\n}\n",
    )
    .unwrap();
    fs::canonicalize(base).unwrap()
}

#[test]
fn an_import_nothing_uses_is_faded_whole() {
    let base = packages();
    let source = "import \"util://tools\";\n\ninit {\n    println(1);\n}\n";
    let mut editor = session_with(Some(base.to_str().unwrap()));
    let found = hints(editor.open(URI, source));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].range, named(source, "import \"util://tools\";", 0));
    assert!(found[0].unnecessary);
    assert_eq!(found[0].code, Some(0x10AD));
    assert_eq!(found[0].message, "The import `util://tools` is never used");
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn an_import_whose_names_are_used_is_not_faded() {
    let base = packages();
    let source = "import \"util://tools\";\n\ninit {\n    half(4);\n}\n";
    let mut editor = session_with(Some(base.to_str().unwrap()));
    assert!(hints(editor.open(URI, source)).is_empty());
    fs::remove_dir_all(base).unwrap();
}

const OWNERSHIP: &str = "pub Box() {
    pub into: Int32 value = 0;
}

pub fun peek(box: Box&): Int32 {
    return box.value;
}

init {
    now into owner = Box();
    now into view = owner.borrow();
    peek(view);
}
";

#[test]
fn the_names_that_own_nothing_are_listed() {
    let mut editor = session();
    editor.open(URI, OWNERSHIP);
    let ranges = editor.non_owning(URI);
    let expected = vec![
        named(OWNERSHIP, "box", 0),
        named(OWNERSHIP, "box", 1),
        named(OWNERSHIP, "view", 0),
        named(OWNERSHIP, "view", 1),
    ];
    assert_eq!(ranges, expected);
}
