mod common;

use std::path::PathBuf;

use common::{position_of, session_with};

fn packages() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned()
}

const URI: &str = "file:///project/src/main/program.into";
const PROGRAM: &str = r#"import "intolang://builtin/std";
import "cpp://standard/time";

Box() {}

init {
    into box = Box();
    println(box.toString());
}
"#;

#[test]
fn println_jumps_to_its_declaration() {
    let mut editor = session_with(Some(&packages()));
    let diagnostics = editor.open(URI, PROGRAM);
    let locations = editor.definition(URI, position_of(PROGRAM, "println", 0));
    assert_eq!(locations.len(), 1, "{locations:?} {diagnostics:?}");
}

#[test]
fn a_method_of_the_root_class_jumps_to_its_declaration() {
    let mut editor = session_with(Some(&packages()));
    let diagnostics = editor.open(URI, PROGRAM);
    let locations = editor.definition(URI, position_of(PROGRAM, "toString", 0));
    assert_eq!(locations.len(), 1, "{locations:?} {diagnostics:?}");
}

#[test]
fn the_ownership_members_of_the_root_class_jump_to_their_declarations() {
    let program = r#"Box() {}

init {
    into box = Box();
    into other = box.copy();
    other.destroy();
}
"#;
    let mut editor = session_with(Some(&packages()));
    let diagnostics = editor.open(URI, program);
    for method in ["copy", "destroy"] {
        let locations = editor.definition(URI, position_of(program, method, 0));
        assert_eq!(
            locations.len(),
            1,
            "{method}: {locations:?} {diagnostics:?}"
        );
    }
}

#[test]
fn the_members_of_an_event_jump_to_their_declarations() {
    let program = r#"pub event started(code: Int32) {}

init {
    into listener = started.on((code) => {});
    started.emit(1);
}
"#;
    let mut editor = session_with(Some(&packages()));
    let diagnostics = editor.open(URI, program);
    for method in ["on", "emit"] {
        let dot = position_of(program, &format!(".{method}("), 0);
        let locations = editor.definition(URI, ipc::Position::new(dot.line, dot.column + 1));
        assert_eq!(
            locations.len(),
            1,
            "{method}: {locations:?} {diagnostics:?}"
        );
    }
}

#[test]
fn a_nested_package_import_jumps_to_its_module() {
    let mut editor = session_with(Some(&packages()));
    let diagnostics = editor.open(URI, PROGRAM);
    let locations = editor.definition(URI, position_of(PROGRAM, "time\"", 0));
    assert_eq!(locations.len(), 1, "{locations:?} {diagnostics:?}");
}
