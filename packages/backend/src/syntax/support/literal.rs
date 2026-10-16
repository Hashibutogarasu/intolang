use crate::syntax::support::text::decode_string_literal;

pub enum Literal {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
}

pub fn parse_literal(text: &str) -> Option<Literal> {
    let text = text.trim();
    if text.starts_with('"') && text.ends_with('"') && text.len() >= 2 {
        return Some(Literal::String(decode_string_literal(
            &text[1..text.len() - 1],
        )));
    }
    if text == "true" {
        return Some(Literal::Bool(true));
    }
    if text == "false" {
        return Some(Literal::Bool(false));
    }
    if text.contains('.') {
        if let Ok(value) = text.parse::<f64>() {
            return Some(Literal::Float(value));
        }
    }
    if let Ok(value) = text.parse::<i64>() {
        return Some(Literal::Int(value));
    }
    None
}
