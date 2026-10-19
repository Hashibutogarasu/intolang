use formatter::format_source;

#[test]
fn keeps_reference_types_attached_to_their_names() {
    assert_eq!(
        format_source("pub fun stop(target:Process&,code:Int32):Process&{return target;}"),
        r#"pub fun stop(target: Process&, code: Int32): Process& {
    return target;
}
"#
    );
}

#[test]
fn formats_reference_annotations() {
    assert_eq!(
        format_source("now into view:Counter&=owner;"),
        r#"now into view: Counter& = owner;
"#
    );
}
