mod common;

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use analysis::path_to_uri;
use common::{position_of, session_with};

fn packages() -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let base = std::env::temp_dir().join(format!(
        "analysis-modules-{}-{}",
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
        package.join("src/main/into-info.into"),
        "export \"util://tools/half\";\nexport \"util://std/print\";\n",
    )
    .unwrap();
    fs::write(
        package.join("src/main/tools.into"),
        "/// Halves a number.\npub fun half(number: Int32): Int32 {\n    return number;\n}\n",
    )
    .unwrap();
    fs::canonicalize(base).unwrap()
}

const PROGRAM: &str = "import \"util://tools\";\n\ninit {\n    half(4);\n}\n";
const URI: &str = "file:///project/src/main/program.into";

#[test]
fn a_call_jumps_into_the_imported_module() {
    let base = packages();
    let mut editor = session_with(Some(base.to_str().unwrap()));
    assert!(
        editor
            .open(URI, PROGRAM)
            .iter()
            .all(|d| d.code != Some(0x10AA))
    );
    let module = base.join("util/src/main/tools.into");
    let locations = editor.definition(URI, position_of(PROGRAM, "half(4)", 0));
    assert_eq!(locations.len(), 1);
    assert_eq!(locations[0].uri, path_to_uri(&module));
    assert_eq!(locations[0].range.start.line, 1);
    assert_eq!(locations[0].range.start.column, 8);
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn hovering_an_imported_function_shows_its_documentation() {
    let base = packages();
    let mut editor = session_with(Some(base.to_str().unwrap()));
    editor.open(URI, PROGRAM);
    let hover = editor
        .hover(URI, position_of(PROGRAM, "half(4)", 0))
        .unwrap();
    assert_eq!(hover.signature, "pub fun half(number: Int32): Int32");
    assert_eq!(hover.documentation.unwrap().markdown, "Halves a number.");
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn the_import_string_jumps_to_the_module_file() {
    let base = packages();
    let mut editor = session_with(Some(base.to_str().unwrap()));
    editor.open(URI, PROGRAM);
    let locations = editor.definition(URI, position_of(PROGRAM, "util://", 0));
    assert_eq!(locations.len(), 1);
    assert_eq!(
        locations[0].uri,
        path_to_uri(&base.join("util/src/main/tools.into"))
    );
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn hovering_an_import_shows_the_package() {
    let base = packages();
    let mut editor = session_with(Some(base.to_str().unwrap()));
    editor.open(URI, PROGRAM);
    for needle in ["util://", "import"] {
        let hover = editor.hover(URI, position_of(PROGRAM, needle, 0)).unwrap();
        assert_eq!(hover.signature, "package util 0.1.0 (util)");
        let markdown = hover.documentation.unwrap().markdown;
        assert!(markdown.contains("- licence: MIT"));
        assert!(markdown.contains("- alias: `util`"));
    }
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn an_export_is_resolved_like_an_import() {
    let base = packages();
    let mut editor = session_with(Some(base.to_str().unwrap()));
    let info = path_to_uri(&base.join("util/src/main/into-info.into"));
    let text = "export \"util://tools/half\";\nexport \"util://std/print\";\n";
    editor.open(&info, text);
    let module = path_to_uri(&base.join("util/src/main/tools.into"));
    let jump = editor.definition(&info, position_of(text, "util://tools", 0));
    assert_eq!(jump[0].uri, module);
    let hover = editor.hover(&info, position_of(text, "export", 0)).unwrap();
    assert_eq!(hover.signature, "package util 0.1.0 (util)");
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn a_module_without_a_file_jumps_to_the_list_of_exports() {
    let base = packages();
    let mut editor = session_with(Some(base.to_str().unwrap()));
    let program = "import \"util://std\";\n";
    editor.open(URI, program);
    let locations = editor.definition(URI, position_of(program, "util://", 0));
    assert_eq!(locations.len(), 1);
    assert_eq!(
        locations[0].uri,
        path_to_uri(&base.join("util/src/main/into-info.into"))
    );
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn an_open_buffer_of_the_module_wins_over_the_file() {
    let base = packages();
    let mut editor = session_with(Some(base.to_str().unwrap()));
    let module = path_to_uri(&base.join("util/src/main/tools.into"));
    editor.open(
        &module,
        "\n\n\n/// Edited.\npub fun half(number: Int32): Int32 {\n    return number;\n}\n",
    );
    editor.open(URI, PROGRAM);
    let locations = editor.definition(URI, position_of(PROGRAM, "half(4)", 0));
    assert_eq!(locations[0].range.start.line, 4);
    fs::remove_dir_all(base).unwrap();
}
