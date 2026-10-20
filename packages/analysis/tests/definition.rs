mod common;

use common::{position_of, session};
use ipc::{Position, Range};

const URI: &str = "file:///project/src/main/definition.into";

const SOURCE: &str = "pub Counter() {
    pub into: Int32 value = 0;
    pub fun bump(by: Int32): Int32 {
        return by;
    }
}

enum Color { Red, Green }

pub fun twice(number: Int32): Int32 {
    into doubled = number;
    return doubled;
}

init {
    now into counter = Counter();
    counter.bump(1);
    into total = counter.value;
    into color = Color.Green;
    twice(2);
}
";

fn jump(source: &str, needle: &str, occurrence: usize) -> Option<Range> {
    let mut editor = session();
    editor.open(URI, source);
    let locations = editor.definition(URI, position_of(source, needle, occurrence));
    assert!(locations.len() <= 1);
    locations.first().map(|location| {
        assert_eq!(location.uri, URI);
        location.range
    })
}

fn names(source: &str, needle: &str, occurrence: usize, length: u32) -> Range {
    let start = position_of(source, needle, occurrence);
    Range::new(start, Position::new(start.line, start.column + length))
}

/// The range of the first `text` in `source`, which may span lines.
fn phrase(source: &str, text: &str) -> Range {
    let start = position_of(source, text, 0);
    let lines = text.matches('\n').count() as u32;
    let column = match text.rsplit_once('\n') {
        Some((_, last)) => last.chars().count() as u32,
        None => start.column + text.chars().count() as u32,
    };
    Range::new(start, Position::new(start.line + lines, column))
}

#[test]
fn a_call_jumps_to_the_function() {
    assert_eq!(
        jump(SOURCE, "twice(2)", 0),
        Some(names(SOURCE, "twice(number", 0, 5))
    );
}

#[test]
fn a_constructor_call_jumps_to_the_class() {
    assert_eq!(
        jump(SOURCE, "Counter();", 0),
        Some(names(SOURCE, "Counter()", 0, 7))
    );
}

#[test]
fn a_local_binding_and_a_parameter_jump_to_their_declarations() {
    assert_eq!(
        jump(SOURCE, "counter.bump", 0),
        Some(names(SOURCE, "counter =", 0, 7))
    );
    assert_eq!(
        jump(SOURCE, "number;", 0),
        Some(names(SOURCE, "number:", 0, 6))
    );
    assert_eq!(
        jump(SOURCE, "doubled;", 0),
        Some(names(SOURCE, "doubled =", 0, 7))
    );
}

#[test]
fn members_are_found_through_the_type_of_the_receiver() {
    assert_eq!(
        jump(SOURCE, "bump(1)", 0),
        Some(names(SOURCE, "bump(by", 0, 4))
    );
    assert_eq!(
        jump(SOURCE, "value;", 0),
        Some(names(SOURCE, "value =", 0, 5))
    );
    assert_eq!(
        jump(SOURCE, "Green;", 0),
        Some(names(SOURCE, "Green }", 0, 5))
    );
}

#[test]
fn a_break_jumps_to_the_loop_it_leaves() {
    let source = r#"init {
    into rows = 0;
    while (rows < 3) {
        into columns = 0;
        while (columns < 3) {
            if (columns == 1) {
                break;
            }
            columns = columns + 1;
        }
        if (rows == 2) {
            break;
        }
        rows = rows + 1;
    }
}
"#;
    assert_eq!(
        jump(source, "break", 0),
        Some(names(source, "while (columns", 0, 5))
    );
    assert_eq!(
        jump(source, "break", 1),
        Some(names(source, "while (rows", 0, 5))
    );
}

#[test]
fn an_else_jumps_to_the_branch_it_follows() {
    let source = r#"init {
    into number = 1;
    if (number == 1) {
        number = 2;
    } else if (number == 2) {
        number = 3;
    } else    if (number == 3) {
        number = 4;
    } else
    if (number == 4) {
        number = 5;
    } else {
        number = 6;
    }
}
"#;
    let mut editor = session();
    editor.open(URI, source);
    // Each line is a keyword (or both words of `else if`) and the branch it
    // continues; the jump starts on the first and on the last word.
    for (from, to) in [
        ("else if", "if"),
        ("else    if", "else if"),
        ("else\n    if", "else    if"),
        ("else {", "else\n    if"),
    ] {
        let target = phrase(source, to);
        let start = position_of(source, from, 0);
        let end = phrase(source, from).end;
        let last_word = if from.ends_with("if") {
            Position::new(end.line, end.column - 1)
        } else {
            start
        };
        for at in [start, last_word] {
            let locations = editor.definition(URI, at);
            assert_eq!(locations.len(), 1, "{from:?}");
            assert_eq!(locations[0].range, target, "{from:?}");
        }
    }
}

#[test]
fn an_if_that_begins_a_statement_jumps_nowhere() {
    let source = r#"init {
    into number = 1;
    if (number == 1) {
        number = 2;
    }
    if (number == 2) {
        number = 3;
    }
}
"#;
    assert_eq!(jump(source, "if (number == 2)", 0), None);
    assert_eq!(jump(source, "if (number == 1)", 0), None);
}

#[test]
fn a_break_outside_a_loop_jumps_nowhere() {
    let source = r#"init {
    if (1 == 1) {
        break;
    }
}
"#;
    assert_eq!(jump(source, "break", 0), None);
}

#[test]
fn a_name_that_is_declared_nowhere_jumps_nowhere() {
    let source = "init {\n    missing();\n}\n";
    assert_eq!(jump(source, "missing", 0), None);
}

#[test]
fn a_local_is_not_visible_from_another_function() {
    let source =
        "pub fun first() {\n    into hidden = 1;\n}\n\npub fun second() {\n    hidden;\n}\n";
    assert_eq!(jump(source, "hidden;", 0), None);
}

#[test]
fn positions_count_characters_not_bytes() {
    let source = "// 日本語のコメント\npub fun twice() {}\n\ninit {\n    into text = \"日本語\"; twice();\n}\n";
    let range = jump(source, "twice();", 0).expect("a definition");
    assert_eq!(range.start, position_of(source, "twice()", 0));
    assert_eq!(range.end.column - range.start.column, 5);
    let mut editor = session();
    editor.open(URI, source);
    let call = position_of(source, "twice();", 0);
    let hover = editor.hover(URI, call).expect("a hover");
    assert_eq!(hover.range.start, call);
    assert_eq!(hover.range.end.column, call.column + 5);
}

#[test]
fn the_last_good_analysis_answers_while_the_text_does_not_parse() {
    let mut editor = session();
    editor.open(URI, SOURCE);
    let broken = SOURCE.replace("twice(2);", "twice(2) )) ;");
    assert!(!editor.change(URI, &broken).is_empty());
    let locations = editor.definition(URI, position_of(&broken, "twice(2)", 0));
    assert_eq!(locations.len(), 1);
}
