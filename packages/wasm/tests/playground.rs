use serde_json::{Value, json};
use wasm::{Session, read_builtin, run};

const URI: &str = "file:///playground/main.into";

const HELLO: &str = r#"import "intolang://builtin/std";

into message = "Hello World!";

init {
    println(message);
}
"#;

const FIBONACCI: &str = r#"import "intolang://builtin/std";

init {
    into previous: Int32 = 0;
    into current: Int32 = 1;
    into index: Int32 = 0;

    while (index < 5) {
        println(previous);

        into next: Int32 = previous + current;
        previous = current;
        current = next;
        index = index + 1;
    }
}
"#;

fn open(session: &mut Session, text: &str) -> Value {
    let caused = session.notify(
        &json!({
            "method": "document/open",
            "params": { "uri": URI, "version": 1, "text": text },
        })
        .to_string(),
    );
    serde_json::from_str(&caused).unwrap()
}

fn ask(session: &mut Session, method: &str, line: u32, column: u32) -> Value {
    let answer = session.handle(
        &json!({
            "method": method,
            "params": { "uri": URI, "position": { "line": line, "column": column } },
        })
        .to_string(),
    );
    serde_json::from_str(&answer).unwrap()
}

#[test]
fn a_program_prints_what_it_says() {
    let outcome = run(HELLO, "");
    assert_eq!(outcome.error, None);
    assert_eq!(outcome.stdout, "Hello World!\n");
}

#[test]
fn a_loop_runs_to_its_end() {
    let outcome = run(FIBONACCI, "");
    assert_eq!(outcome.error, None);
    assert_eq!(outcome.stdout, "0\n1\n1\n2\n3\n");
}

#[test]
fn a_program_the_editor_underlines_does_not_run() {
    let outcome = run("init {\n    into = ;\n}\n", "");
    assert_eq!(outcome.stdout, "");
    assert!(outcome.error.unwrap().starts_with("line 2: "));
}

#[test]
fn a_package_other_than_the_standard_one_is_not_there() {
    let outcome = run("import \"cpp://standard/time\";\n\ninit {\n}\n", "");
    assert!(outcome.error.is_some());
}

#[test]
fn the_standard_package_can_be_read_by_its_path() {
    let text = read_builtin("/virtual/packages/intolang/src/main/builtin/println.into").unwrap();
    assert!(text.contains("pub fun println"));
    assert_eq!(
        read_builtin("/virtual/packages/intolang/missing.into"),
        None
    );
}

#[test]
fn a_document_that_parses_has_no_diagnostics() {
    let mut session = Session::new();
    let caused = open(&mut session, HELLO);
    assert_eq!(caused[0]["method"], "analysis/diagnostics");
    assert_eq!(caused[0]["params"]["diagnostics"], json!([]));
}

#[test]
fn a_syntax_error_is_a_diagnostic() {
    let mut session = Session::new();
    let caused = open(&mut session, "init {\n    into = ;\n}\n");
    let diagnostics = caused[0]["params"]["diagnostics"].as_array().unwrap();
    assert!(!diagnostics.is_empty());
}

#[test]
fn println_jumps_into_the_standard_package() {
    let mut session = Session::new();
    open(&mut session, HELLO);
    let answer = ask(&mut session, "analysis/definition", 5, 6);
    let locations = answer["ok"]["value"].as_array().unwrap();
    assert_eq!(locations.len(), 1, "{answer}");
    assert_eq!(
        locations[0]["uri"],
        "file:///virtual/packages/intolang/src/main/builtin/println.into"
    );
}

#[test]
fn hovering_a_call_shows_its_signature() {
    let mut session = Session::new();
    open(&mut session, HELLO);
    let answer = ask(&mut session, "analysis/hover", 5, 6);
    assert!(
        answer["ok"]["value"]["signature"]
            .as_str()
            .unwrap()
            .contains("println"),
        "{answer}"
    );
}

#[test]
fn a_request_that_is_not_one_is_an_error() {
    let mut session = Session::new();
    let answer: Value = serde_json::from_str(&session.handle("{}")).unwrap();
    assert!(answer["error"]["message"].is_string(), "{answer}");
}
