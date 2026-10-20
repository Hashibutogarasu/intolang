mod common;

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use analysis::path_to_uri;
use common::{position_of, session_with};

fn packages() -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let base = std::env::temp_dir().join(format!(
        "analysis-primitives-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&base);
    let package = base.join("intolang");
    fs::create_dir_all(package.join("src/main")).unwrap();
    fs::write(
        package.join("into-package.json"),
        r#"{"name":"Into Language","alias":"builtin","version":"0.0.1","licence":"MIT","library":"src/main/lib.into"}"#,
    )
    .unwrap();
    fs::write(
        package.join("src/main/object.into"),
        "pub abstract Object() {}\n",
    )
    .unwrap();
    fs::write(
        package.join("src/main/primitives.into"),
        "/// A signed 32-bit integer.\npub Int32() {}\n\npub String() {}\n",
    )
    .unwrap();
    fs::canonicalize(base).unwrap()
}

const PROGRAM: &str = "init {\n    into count: Int32 = 3;\n    into label: String = \"a\";\n}\n";
const URI: &str = "file:///project/src/main/program.into";

#[test]
fn a_primitive_type_jumps_to_its_declaration_in_the_builtin_package() {
    let base = packages();
    let mut editor = session_with(Some(base.to_str().unwrap()));
    editor.open(URI, PROGRAM);
    let module = base.join("intolang/src/main/primitives.into");
    let locations = editor.definition(URI, position_of(PROGRAM, "Int32", 0));
    assert_eq!(locations.len(), 1);
    assert_eq!(locations[0].uri, path_to_uri(&module));
    assert_eq!(locations[0].range.start.line, 1);
    let locations = editor.definition(URI, position_of(PROGRAM, "String", 0));
    assert_eq!(locations[0].range.start.line, 3);
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn hovering_a_primitive_type_shows_its_documentation() {
    let base = packages();
    let mut editor = session_with(Some(base.to_str().unwrap()));
    editor.open(URI, PROGRAM);
    let hover = editor.hover(URI, position_of(PROGRAM, "Int32", 0)).unwrap();
    assert_eq!(
        hover.documentation.unwrap().markdown,
        "A signed 32-bit integer."
    );
    fs::remove_dir_all(base).unwrap();
}
