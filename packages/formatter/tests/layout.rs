use formatter::format_source;

#[test]
fn indents_blocks_with_four_spaces() {
    assert_eq!(
        format_source(
            r#"init {
into x = 1;
if (x == 1) {
println(x);
} else {
println(0);
}
}
"#
        ),
        r#"init {
    into x = 1;
    if (x == 1) {
        println(x);
    } else {
        println(0);
    }
}
"#
    );
}

#[test]
fn layout_does_not_depend_on_original_line_breaks_or_spaces() {
    let spread = r#"init {
    into x
        =
    1 ;
    println( x )
    ;
}
"#;
    let packed = "init{into x=1;println(x);}";
    let expected = r#"init {
    into x = 1;
    println(x);
}
"#;
    assert_eq!(format_source(spread), expected);
    assert_eq!(format_source(packed), expected);
}

#[test]
fn collapses_blank_lines_but_keeps_one() {
    assert_eq!(
        format_source(
            r#"init {


    a();



    b();
}
"#
        ),
        r#"init {

    a();

    b();
}
"#
    );
}

#[test]
fn expands_enums_one_variant_per_line() {
    assert_eq!(
        format_source("enum Format { Text, JSON = 2 }"),
        r#"enum Format {
    Text,
    JSON = 2
}
"#
    );
}

#[test]
fn keeps_attributes_on_their_own_line() {
    assert_eq!(
        format_source(
            r#"#[link("c", "src/native/hello.c", "greet")] pub c fun greet(name: String): Int32;"#
        ),
        r#"#[link("c", "src/native/hello.c", "greet")]
pub c fun greet(name: String): Int32;
"#
    );
}

#[test]
fn preserves_ambiguous_operator_spacing() {
    assert_eq!(
        format_source("into *p = 1; into a: List<Int32> = b*2; into c: String* = d;"),
        r#"into *p = 1;
into a: List<Int32> = b*2;
into c: String* = d;
"#
    );
}

#[test]
fn keeps_comments_trailing_and_standalone() {
    let source = r#"// header
init { // open
    a(); // call
    // note
    b();
}
"#;
    assert_eq!(format_source(source), source);
}

#[test]
fn keeps_multiline_call_arguments_broken() {
    assert_eq!(
        format_source(
            r#"init {
f(
a,
b
);
}
"#
        ),
        r#"init {
    f(
        a,
        b
    );
}
"#
    );
}

#[test]
fn returns_untokenizable_source_unchanged() {
    assert_eq!(format_source(r#"println("open"#), r#"println("open"#);
}

#[test]
fn formatting_is_idempotent() {
    let once = format_source(
        r#"pub JsonObject() { kind: Int32; text: String; }
enum A { X, Y }
init { while (i < 3) { i = i + 1; if (i == 2) { break; } } }
"#,
    );
    assert_eq!(format_source(&once), once);
}
