/// Format Into source with four-space block indentation and stable blank lines.
pub fn format_source(source: &str) -> String {
    let mut output = String::new();
    let mut depth = 0usize;
    let mut previous_blank = true;

    for raw in source.lines() {
        let line = raw.trim();
        if line.is_empty() {
            if !previous_blank && !output.is_empty() {
                output.push('\n');
                previous_blank = true;
            }
            continue;
        }

        let (opens, closes) = brace_counts(line);
        depth = depth.saturating_sub(closes);
        output.push_str(&"    ".repeat(depth));
        output.push_str(line);
        output.push('\n');
        depth = depth.saturating_add(opens);
        previous_blank = false;
    }
    output
}

fn brace_counts(line: &str) -> (usize, usize) {
    let mut opens = 0;
    let mut closes = 0;
    let mut chars = line.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;
    while let Some(ch) = chars.next() {
        if !in_string && ch == '/' && chars.peek() == Some(&'/') { break; }
        if in_string {
            if escaped { escaped = false; }
            else if ch == '\\' { escaped = true; }
            else if ch == '"' { in_string = false; }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '{' => opens += 1,
            '}' => closes += 1,
            _ => {}
        }
    }
    (opens, closes)
}
