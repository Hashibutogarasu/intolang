/// The analyzer counts columns in characters; the editor counts UTF-16 code
/// units. These convert a column within the given line of `text`.
pub fn to_lsp(text: &str, line: u32, column: u32) -> u32 {
    let Some(content) = text.lines().nth(line as usize) else {
        return column;
    };
    content
        .chars()
        .take(column as usize)
        .map(|character| character.len_utf16() as u32)
        .sum::<u32>()
        + column.saturating_sub(content.chars().count() as u32)
}

pub fn from_lsp(text: &str, line: u32, column: u32) -> u32 {
    let Some(content) = text.lines().nth(line as usize) else {
        return column;
    };
    let mut units = 0;
    let mut characters = 0;
    for character in content.chars() {
        if units >= column {
            return characters;
        }
        units += character.len_utf16() as u32;
        characters += 1;
    }
    characters + column.saturating_sub(units)
}
