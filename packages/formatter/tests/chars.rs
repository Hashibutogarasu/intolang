use formatter::format_source;

#[test]
fn a_character_literal_keeps_its_spelling() {
    assert_eq!(
        format_source("into c:Char='A';"),
        r#"into c: Char = 'A';
"#
    );
    assert_eq!(
        format_source(r"into n:Int32 = '\n' as Int32;"),
        r"into n: Int32 = '\n' as Int32;
"
    );
}

#[test]
fn a_brace_inside_a_character_literal_does_not_open_a_block() {
    assert_eq!(
        format_source("init{into c:Char='{';}"),
        r#"init {
    into c: Char = '{';
}
"#
    );
}

#[test]
fn formatting_a_cast_twice_changes_nothing() {
    let once = format_source("into c:Char=(('0' as Int32)+d) as Char;");
    assert_eq!(format_source(&once), once);
}
