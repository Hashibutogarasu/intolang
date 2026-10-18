use formatter::{FormatOptions, Indent, format_source_with};

const SOURCE: &str = "init {\n\n\n    if (a) {\n        f(\n            x\n        );\n    }\n}\n";

#[test]
fn honors_indent_width() {
    let options = FormatOptions {
        indent: Indent::Spaces(2),
        ..FormatOptions::default()
    };
    assert_eq!(
        format_source_with(SOURCE, options),
        "init {\n\n  if (a) {\n    f(\n      x\n    );\n  }\n}\n"
    );
}

#[test]
fn honors_tab_indent() {
    let options = FormatOptions {
        indent: Indent::Tab,
        ..FormatOptions::default()
    };
    assert_eq!(
        format_source_with(SOURCE, options),
        "init {\n\n\tif (a) {\n\t\tf(\n\t\t\tx\n\t\t);\n\t}\n}\n"
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
        format_source_with(SOURCE, options),
        "init {\n    if (a) {\n        f(\n                x\n        );\n    }\n}\n"
    );
}
