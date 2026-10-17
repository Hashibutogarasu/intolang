/// Remove a leading `keyword` from `text` when it is followed by whitespace,
/// returning the remainder with leading whitespace removed.
pub fn strip_keyword<'a>(text: &'a str, keyword: &str) -> Option<&'a str> {
    let rest = text.strip_prefix(keyword)?;
    rest.starts_with(char::is_whitespace)
        .then(|| rest.trim_start())
}

/// Whether `text` is a single identifier token.
pub fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    match chars.next() {
        Some(ch) if ch.is_ascii_alphabetic() || ch == '_' => {}
        _ => return false,
    }
    text.chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

/// Strip a `//` line comment, ignoring comment markers inside string literals.
pub fn strip_line_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut quoted = false;
    let mut escaped = false;
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' if quoted => escaped = !escaped,
            b'"' if !escaped => quoted = !quoted,
            b'/' if !quoted && bytes.get(index + 1) == Some(&b'/') => return &line[..index],
            _ if escaped => escaped = false,
            _ => {}
        }
        if bytes[index] != b'\\' {
            escaped = false;
        }
        index += 1;
    }
    line
}

/// The net brace balance of a line, ignoring braces inside string literals.
pub fn brace_delta(line: &str) -> i32 {
    let line = strip_line_comment(line);
    let mut delta = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    for ch in line.chars() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '{' => delta += 1,
            '}' => delta -= 1,
            _ => {}
        }
    }
    delta
}
