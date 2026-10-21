use types::Keyword;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeywordDefinition {
    pub keyword: Keyword,
    pub spelling: &'static str,
    pub detail: &'static str,
}

/// A word that is a value rather than a reserved word, such as `true`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValueDefinition {
    pub spelling: &'static str,
    pub detail: &'static str,
}
