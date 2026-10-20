mod common;

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use analysis::path_to_uri;
use common::{position_of, session_with};

const SOURCE: &str = r#"#include <string.h>

/* length(text) is described here */
int into_text_length(const char *text);

static int helper(int value) {
    return value;
}

int
into_text_length(const char *text) {
    return (int)strlen(text) + helper(0);
}
"#;

fn project() -> (PathBuf, String) {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let base = std::env::temp_dir().join(format!(
        "analysis-native-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&base);
    let root = base.join("app");
    fs::create_dir_all(root.join("src/main")).unwrap();
    fs::create_dir_all(root.join("src/ffi")).unwrap();
    fs::write(
        root.join("into-package.json"),
        r#"{"name":"app","alias":"app","version":"0.1.0","LICENCE":"MIT","runs":"src/main/app.into"}"#,
    )
    .unwrap();
    fs::write(root.join("src/ffi/text.c"), SOURCE).unwrap();
    let program = "#[link(\"c\", \"src/ffi/text.c\", \"into_text_length\")]\npub c fun length(text: String): Int32;\n";
    let path = root.join("src/main/app.into");
    fs::write(&path, program).unwrap();
    (fs::canonicalize(base).unwrap(), program.to_owned())
}

#[test]
fn a_foreign_function_jumps_to_its_definition_in_the_native_source() {
    let (base, program) = project();
    let uri = path_to_uri(&base.join("app/src/main/app.into"));
    let mut editor = session_with(None);
    editor.open(&uri, &program);
    let source = path_to_uri(&base.join("app/src/ffi/text.c"));
    for needle in ["length(text", "into_text_length", "src/ffi"] {
        let locations = editor.definition(&uri, position_of(&program, needle, 0));
        assert_eq!(locations.len(), 1, "{needle}");
        assert_eq!(locations[0].uri, source);
        assert_eq!(locations[0].range.start.line, 10, "{needle}");
        assert_eq!(locations[0].range.start.column, 0);
    }
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn hovering_a_foreign_function_names_its_native_source() {
    let (base, program) = project();
    let uri = path_to_uri(&base.join("app/src/main/app.into"));
    let mut editor = session_with(None);
    editor.open(&uri, &program);
    let hover = editor
        .hover(&uri, position_of(&program, "length(text", 0))
        .unwrap();
    assert!(
        hover
            .documentation
            .unwrap()
            .markdown
            .contains("`src/ffi/text.c` `into_text_length`")
    );
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn a_call_still_jumps_to_the_declaration() {
    let (base, program) = project();
    let uri = path_to_uri(&base.join("app/src/main/app.into"));
    let mut editor = session_with(None);
    let text = format!("{program}init {{\n    length(\"a\");\n}}\n");
    editor.open(&uri, &text);
    let locations = editor.definition(&uri, position_of(&text, "length(\"a\")", 0));
    assert_eq!(locations[0].uri, uri);
    assert_eq!(locations[0].range.start.line, 1);
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn a_declaration_without_a_definition_is_the_fallback() {
    let (base, program) = project();
    fs::write(
        base.join("app/src/ffi/text.c"),
        "int into_text_length(const char *text);\n",
    )
    .unwrap();
    let uri = path_to_uri(&base.join("app/src/main/app.into"));
    let mut editor = session_with(None);
    editor.open(&uri, &program);
    let locations = editor.definition(&uri, position_of(&program, "into_text_length", 0));
    assert_eq!(locations[0].range.start.line, 0);
    assert_eq!(locations[0].range.start.column, 4);
    fs::remove_dir_all(base).unwrap();
}
