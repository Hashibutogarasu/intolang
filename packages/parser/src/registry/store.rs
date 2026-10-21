use std::sync::OnceLock;

use types::Keyword;

use super::bootstrap::bootstrap;
use super::definition::{KeywordDefinition, ValueDefinition};

/// The definitions of the grammar. It holds them and answers questions about
/// them; what it holds is registered by `bootstrap`, in one place.
#[derive(Debug, Default)]
pub struct Registry {
    keywords: Vec<KeywordDefinition>,
    values: Vec<ValueDefinition>,
}

impl Registry {
    /// The registry the compiler and the tools share, built once by `bootstrap`.
    pub fn global() -> &'static Registry {
        static GLOBAL: OnceLock<Registry> = OnceLock::new();
        GLOBAL.get_or_init(|| {
            let mut registry = Registry::default();
            bootstrap(&mut registry);
            registry
        })
    }

    pub fn register_keyword(&mut self, definition: KeywordDefinition) {
        self.keywords.push(definition);
    }

    pub fn register_value(&mut self, definition: ValueDefinition) {
        self.values.push(definition);
    }

    pub fn values(&self) -> &[ValueDefinition] {
        &self.values
    }

    /// Every reserved word, in the order they were registered.
    pub fn keywords(&self) -> &[KeywordDefinition] {
        &self.keywords
    }

    pub fn keyword_spelled(&self, spelling: &str) -> Option<&KeywordDefinition> {
        self.keywords
            .iter()
            .find(|definition| definition.spelling == spelling)
    }

    pub fn keyword(&self, keyword: Keyword) -> Option<&KeywordDefinition> {
        self.keywords
            .iter()
            .find(|definition| definition.keyword == keyword)
    }
}
