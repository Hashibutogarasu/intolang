use formatter::{FormatOptions, Indent, format_source_with};

#[test]
fn honors_indent_width() {
    let options = FormatOptions {
        indent: Indent::Spaces(2),
        ..FormatOptions::default()
    };
    assert_eq!(
        format_source_with(
            r#"init {


    if (a) {
        f(
            x
        );
    }
}
"#,
            options
        ),
        r#"init {

  if (a) {
    f(
      x
    );
  }
}
"#
    );
}

#[test]
fn honors_tab_indent() {
    let options = FormatOptions {
        indent: Indent::Tab,
        ..FormatOptions::default()
    };
    assert_eq!(
        format_source_with(
            r#"init {


    if (a) {
        f(
            x
        );
    }
}
"#,
            options
        ),
        r#"init {

	if (a) {
		f(
			x
		);
	}
}
"#
    );
}

#[test]
fn honors_blank_line_limit_and_continuation_levels() {
    let options = FormatOptions {
        max_blank_lines: 0,
        continuation_levels: 2,
        ..FormatOptions::default()
    };
    assert_eq!(
        format_source_with(
            r#"init {


    if (a) {
        f(
            x
        );
    }
}
"#,
            options
        ),
        r#"init {
    if (a) {
        f(
                x
        );
    }
}
"#
    );
}
