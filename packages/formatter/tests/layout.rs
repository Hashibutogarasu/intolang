use formatter::format_source;

#[test]
fn indents_blocks_with_four_spaces() {
    let source = "init {\ninto x = 1;\nif (x == 1) {\nprintln(x);\n} else {\nprintln(0);\n}\n}\n";
    assert_eq!(
        format_source(source),
        "init {\n    into x = 1;\n    if (x == 1) {\n        println(x);\n    } else {\n        println(0);\n    }\n}\n"
    );
}

#[test]
fn layout_does_not_depend_on_original_line_breaks_or_spaces() {
    let spread = "init {\n    into x\n        =\n    1 ;\n    println( x )\n    ;\n}\n";
    let packed = "init{into x=1;println(x);}";
    let expected = "init {\n    into x = 1;\n    println(x);\n}\n";
    assert_eq!(format_source(spread), expected);
    assert_eq!(format_source(packed), expected);
}

#[test]
fn collapses_blank_lines_but_keeps_one() {
    let source = "init {\n\n\n    a();\n\n\n\n    b();\n}\n";
    assert_eq!(format_source(source), "init {\n\n    a();\n\n    b();\n}\n");
}

#[test]
fn expands_enums_one_variant_per_line() {
    assert_eq!(
        format_source("enum Format { Text, JSON = 2 }"),
        "enum Format {\n    Text,\n    JSON = 2\n}\n"
    );
}

#[test]
fn keeps_attributes_on_their_own_line() {
    let source = "#[link(\"c\", \"src/native/hello.c\", \"greet\")] pub c fun greet(name: String): Int32;";
    assert_eq!(
        format_source(source),
        "#[link(\"c\", \"src/native/hello.c\", \"greet\")]\npub c fun greet(name: String): Int32;\n"
    );
}

#[test]
fn preserves_ambiguous_operator_spacing() {
    assert_eq!(
        format_source("into *p = 1; into a: List<Int32> = b*2; into c: String* = d;"),
        "into *p = 1;\ninto a: List<Int32> = b*2;\ninto c: String* = d;\n"
    );
}

#[test]
fn keeps_comments_trailing_and_standalone() {
    let source = "// header\ninit { // open\n    a(); // call\n    // note\n    b();\n}\n";
    assert_eq!(
        format_source(source),
        "// header\ninit { // open\n    a(); // call\n    // note\n    b();\n}\n"
    );
}

#[test]
fn keeps_multiline_call_arguments_broken() {
    let source = "init {\nf(\na,\nb\n);\n}\n";
    assert_eq!(
        format_source(source),
        "init {\n    f(\n        a,\n        b\n    );\n}\n"
    );
}

#[test]
fn returns_untokenizable_source_unchanged() {
    assert_eq!(format_source("println(\"open"), "println(\"open");
}

#[test]
fn formatting_is_idempotent() {
    let source = "pub JsonObject() { kind: Int32; text: String; }\nenum A { X, Y }\ninit { while (i < 3) { i = i + 1; if (i == 2) { break; } } }\n";
    let once = format_source(source);
    assert_eq!(format_source(&once), once);
}
