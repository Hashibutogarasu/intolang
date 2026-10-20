use errors::{Context, Domain, Error, Errors, ReportFormat, Result};

fn failing() -> Result<()> {
    Err(Errors::LldLinkFailed {
        status: "exit status: 1".into(),
    }
    .into())
}

#[test]
fn notes_are_printed_from_the_outermost_to_the_message() {
    let error = failing()
        .context("embedded LLD failed to write executable")
        .context("Compilation failed")
        .unwrap_err();
    let text = error.to_string();
    assert!(
        text.starts_with(
            "Compilation failed: embedded LLD failed to write executable: LLD link failed (exit status: 1)"
        ),
        "{text}"
    );
}

#[test]
fn notes_do_not_change_the_kind_the_code_or_the_domain() {
    let error = failing().with_context(|| "linking").unwrap_err();
    assert_eq!(error.code(), Some(0xA021));
    assert_eq!(error.domain(), Domain::Compiler);
    assert_eq!(error.notes(), ["linking"]);
}

#[test]
fn the_json_report_lists_the_notes_from_the_outermost() {
    let error = failing().context("inner").context("outer").unwrap_err();
    let value: serde_json::Value = serde_json::from_str(&error.report(ReportFormat::Json)).unwrap();
    assert_eq!(value["notes"][0], "outer");
    assert_eq!(value["notes"][1], "inner");
    assert_eq!(value["context"]["code"], 0xA021);
}

#[test]
fn a_missing_value_becomes_an_error_that_the_note_describes() {
    let error: Error = None::<u8>
        .context("Library entry has no parent")
        .unwrap_err();
    assert_eq!(error.code(), Some(0xC001));
    assert_eq!(
        error.to_string().lines().next(),
        Some("Library entry has no parent")
    );
}
