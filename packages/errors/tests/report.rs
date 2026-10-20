use errors::{
    Domain, Error, ErrorContext, Errors, IntoLangStackTrace, ReportFormat, Severity, SourceSpan,
    SyntaxContext,
};

fn syntax_error() -> Error {
    let kind = Errors::Syntax {
        message: "Expected a type name".into(),
    };
    let context = ErrorContext::of(&kind)
        .at(
            "main.into",
            SourceSpan {
                line: 2,
                end_line: 2,
                start: 4,
                end: 9,
            },
        )
        .with_syntax(
            SyntaxContext::in_scope("module")
                .expecting("a type name")
                .found("`;`")
                .hint("write the type after the colon"),
        );
    Error::into_lang(kind, IntoLangStackTrace::at("module", "main.into", 2)).with_context(context)
}

#[test]
fn the_json_report_has_a_fixed_schema() {
    let text = syntax_error().report(ReportFormat::Json);
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(value["context"]["code"], 1);
    assert_eq!(value["context"]["domain"], "into_lang");
    assert_eq!(value["context"]["severity"], "error");
    assert_eq!(value["context"]["message"], "Expected a type name");
    assert_eq!(value["context"]["path"], "main.into");
    assert_eq!(value["context"]["span"]["line"], 2);
    assert_eq!(value["context"]["span"]["start"], 4);
    assert_eq!(value["context"]["span"]["end"], 9);
    assert_eq!(value["context"]["syntax"]["scope"], "module");
    assert_eq!(value["context"]["syntax"]["expected"][0], "a type name");
    assert_eq!(value["context"]["syntax"]["found"], "`;`");
    assert_eq!(value["trace"][0]["layer"], "into");
    assert_eq!(value["trace"][0]["frames"][0]["label"], "module");
    assert_eq!(value["trace"][0]["frames"][0]["location"], "main.into:2");
}

#[test]
fn the_context_survives_a_round_trip() {
    let context = syntax_error().resolved_context();
    let text = serde_json::to_string(&context).unwrap();
    let back: ErrorContext = serde_json::from_str(&text).unwrap();
    assert_eq!(back, context);
    assert_eq!(back.domain, Domain::IntoLang);
    assert_eq!(back.severity, Severity::Error);
}

#[test]
fn both_reports_describe_the_same_error() {
    let error = syntax_error();
    let human = error.report(ReportFormat::Human);
    let json = error.report(ReportFormat::Json);
    assert_eq!(human, error.to_string());
    assert!(human.contains("Expected a type name"));
    assert!(json.contains("Expected a type name"));
}

#[test]
fn an_error_without_a_context_gets_the_one_its_kind_implies() {
    let error = Error::new(Errors::Unknown);
    let context = error.resolved_context();
    assert_eq!(context.code, None);
    assert_eq!(context.domain, Domain::Compiler);
    assert_eq!(context.message, "Unknown error");
}
