mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use common::{position_of, session, session_with};
use ipc::{CompletionItem, CompletionKind, Position};

const URI: &str = "file:///project/src/main/completion.into";

const SCOPE: &str = "pub Counter() {
    pub into: Int32 value = 0;
    pub fun bump(by: Int32): Int32 {
        return by;
    }
}

pub fun twice(number: Int32): Int32 {
    return number;
}

init {
    now into counter = Counter();
    |
}
";

fn split(marked: &str) -> (String, Position) {
    let at = position_of(marked, "|", 0);
    (marked.replacen('|', "", 1), at)
}

fn items(marked: &str) -> Vec<CompletionItem> {
    let (text, at) = split(marked);
    session().complete_after_open(&text, at)
}

trait OpenAndComplete {
    fn complete_after_open(&mut self, text: &str, at: Position) -> Vec<CompletionItem>;
}

impl OpenAndComplete for common::Session {
    fn complete_after_open(&mut self, text: &str, at: Position) -> Vec<CompletionItem> {
        self.open(URI, text);
        self.complete(URI, at)
    }
}

fn labels(marked: &str) -> Vec<String> {
    items(marked).into_iter().map(|item| item.label).collect()
}

fn typed(line: &str) -> Vec<String> {
    labels(&SCOPE.replace('|', &format!("{line}|")))
}

#[test]
fn names_in_scope_are_offered_by_their_prefix() {
    let offered = typed("cou");
    assert!(offered.contains(&"counter".to_owned()), "{offered:?}");
    assert!(!offered.contains(&"twice".to_owned()), "{offered:?}");
    assert_eq!(typed("cons"), ["const"]);
    assert_eq!(typed("tw"), ["twice"]);
    assert!(typed("Cou").contains(&"Counter".to_owned()));
}

#[test]
fn every_registered_keyword_is_offered() {
    assert_eq!(typed("whi"), ["while"]);
    let all = typed("");
    for keyword in parser::KeywordLexeme::all() {
        assert!(all.contains(&keyword.to_string()), "{keyword}");
    }
    for value in ["true", "false", "this"] {
        assert!(all.contains(&value.to_owned()), "{value}");
    }
}

#[test]
fn a_keyword_candidate_says_what_it_does() {
    let items = items(&SCOPE.replace('|', "fu|"));
    let keyword = items.iter().find(|item| item.label == "fun").unwrap();
    assert_eq!(keyword.kind, CompletionKind::Keyword);
    assert_eq!(keyword.detail.as_deref(), Some("Declare a function"));
}

#[test]
fn a_name_declared_after_the_cursor_is_not_offered() {
    let offered = labels("init {\n    into early = 1;\n    ea|\n    into zeta = 2;\n}\n");
    assert!(offered.contains(&"early".to_owned()), "{offered:?}");
    assert!(!offered.contains(&"zeta".to_owned()), "{offered:?}");
    let offered = labels("init {\n    into early = 1;\n    |\n    into zeta = 2;\n}\n");
    assert!(!offered.contains(&"zeta".to_owned()), "{offered:?}");
}

#[test]
fn a_candidate_replaces_the_prefix() {
    let items = items(&SCOPE.replace('|', "cou|"));
    let item = items.iter().find(|item| item.label == "counter").unwrap();
    assert_eq!(item.range.end.column - item.range.start.column, 3);
    assert_eq!(item.kind, CompletionKind::Variable);
    assert_eq!(item.detail.as_deref(), Some("now into counter: Counter"));
}

#[test]
fn the_members_of_a_value_follow_the_dot() {
    let offered = typed("counter.");
    assert!(offered.contains(&"value".to_owned()), "{offered:?}");
    assert!(offered.contains(&"bump".to_owned()), "{offered:?}");
    assert!(!offered.contains(&"twice".to_owned()), "{offered:?}");
    assert!(!offered.contains(&"while".to_owned()), "{offered:?}");
    assert_eq!(typed("counter.bu"), ["bump"]);
    assert!(typed("unknown.").is_empty());
}

#[test]
fn the_variants_of_an_enum_follow_the_dot() {
    let offered = labels("enum Color { Red, Green }\n\ninit {\n    Color.|\n}\n");
    assert_eq!(offered, ["Red", "Green"]);
}

#[test]
fn nothing_is_offered_in_comments_and_strings() {
    assert!(labels("init {\n    // twi|\n}\n").is_empty());
    assert!(labels("init {\n    into text = \"twi|\";\n}\n").is_empty());
}

#[test]
fn the_names_of_the_last_good_parse_serve_a_broken_document() {
    let mut editor = session();
    editor.open(URI, &SCOPE.replace('|', ""));
    let broken = SCOPE
        .replace("return number;", "return number")
        .replace('|', "tw");
    editor.change(URI, &broken);
    let at = position_of(&broken, "tw\n", 0);
    let offered = editor.complete(URI, Position::new(at.line, at.column + 2));
    assert!(offered.iter().any(|item| item.label == "twice"));
}

#[test]
fn an_event_handler_takes_the_names_of_the_event_arguments() {
    let source = "pub event audioReady(sampleRate: Int32, channels: Int32) {}\n\ninit {\n    audioReady.on(|\n}\n";
    let items = items(source);
    let snippet = items.iter().find(|item| item.snippet).unwrap();
    assert_eq!(snippet.label, "(sampleRate, channels) => { }");
    assert_eq!(
        snippet.text,
        "(${1:sampleRate}, ${2:channels}) => {\n\t$0\n}"
    );
    assert_eq!(snippet.kind, CompletionKind::Snippet);
}

#[test]
fn a_callback_parameter_takes_the_names_of_its_type() {
    let pair = "pub fun each(action: BiCallback<String, Json>) {\n}\n\ninit {\n    each(|\n}\n";
    let offered = items(pair);
    let snippet = offered.iter().find(|item| item.snippet).unwrap();
    assert_eq!(snippet.text, "(${1:key}, ${2:value}) => {\n\t$0\n}");
    let single = "pub fun each(action: Callback<Json>) {\n}\n\ninit {\n    each(|\n}\n";
    let offered = items(single);
    let snippet = offered.iter().find(|item| item.snippet).unwrap();
    assert_eq!(snippet.text, "(${1:json}) => {\n\t$0\n}");
}

#[test]
fn only_the_argument_that_takes_a_callback_gets_the_snippet() {
    let source =
        "pub fun each(count: Int32, action: Callback<Json>) {\n}\n\ninit {\n    each(|\n}\n";
    assert!(items(source).iter().all(|item| !item.snippet));
    let source =
        "pub fun each(count: Int32, action: Callback<Json>) {\n}\n\ninit {\n    each(1, |\n}\n";
    assert_eq!(items(source).iter().filter(|item| item.snippet).count(), 1);
}

fn packages() -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let base = std::env::temp_dir().join(format!(
        "analysis-completion-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&base);
    let package = base.join("util");
    fs::create_dir_all(package.join("src/main/deep")).unwrap();
    fs::write(
        package.join("into-package.json"),
        r#"{"name":"util","alias":"util","version":"0.1.0","LICENCE":"MIT","library":"src/main/lib.into"}"#,
    )
    .unwrap();
    fs::write(
        package.join("src/main/tools.into"),
        "pub fun half(): Int32 {\n    return 1;\n}\n",
    )
    .unwrap();
    fs::write(
        package.join("src/main/deep/leaf.into"),
        "pub fun leaf(): Int32 {\n    return 1;\n}\n",
    )
    .unwrap();
    fs::write(package.join("src/main/deep/notes.txt"), "not a module").unwrap();
    fs::canonicalize(base).unwrap()
}

fn addresses(base: &Path, marked: &str) -> Vec<CompletionItem> {
    let (text, at) = split(marked);
    session_with(Some(base.to_str().unwrap())).complete_after_open(&text, at)
}

#[test]
fn an_address_starts_with_a_scheme() {
    let base = packages();
    let offered: Vec<String> = addresses(&base, "import \"|")
        .into_iter()
        .map(|item| item.label)
        .collect();
    assert_eq!(offered, ["intolang://", "util://"]);
    let offered = addresses(&base, "import \"ut|");
    assert_eq!(offered.len(), 1);
    assert_eq!(
        offered[0].range.end.column - offered[0].range.start.column,
        2
    );
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn an_address_continues_with_the_folders_and_modules_of_the_package() {
    let base = packages();
    let offered = addresses(&base, "import \"util://|\";\n");
    let names: Vec<&str> = offered.iter().map(|item| item.label.as_str()).collect();
    assert_eq!(names, ["deep", "tools"]);
    assert_eq!(offered[0].kind, CompletionKind::Folder);
    assert_eq!(offered[0].text, "deep/");
    assert_eq!(offered[1].kind, CompletionKind::Module);
    assert_eq!(offered[1].text, "tools");
    let offered = addresses(&base, "import \"util://deep/|");
    assert_eq!(offered.len(), 1);
    assert_eq!(offered[0].label, "leaf");
    let offered = addresses(&base, "import \"util://t|");
    assert_eq!(offered.len(), 1);
    assert_eq!(offered[0].label, "tools");
    assert_eq!(
        offered[0].range.end.column - offered[0].range.start.column,
        1
    );
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn an_export_is_completed_like_an_import() {
    let base = packages();
    let offered = addresses(&base, "export \"util://deep/|");
    assert_eq!(offered.len(), 1);
    assert_eq!(offered[0].label, "leaf");
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn only_the_address_of_an_import_is_completed_as_a_path() {
    let base = packages();
    assert!(addresses(&base, "init {\n    into text = \"util://|\n").is_empty());
    fs::remove_dir_all(base).unwrap();
}
