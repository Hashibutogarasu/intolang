use crate::errors::{Domain, ENTRIES};

/// One declared error: its code, domain and variant name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CatalogEntry {
    pub code: u16,
    pub domain: Domain,
    pub name: &'static str,
}

/// Every declared error except `Unknown`.
pub const CATALOG: &[CatalogEntry] = ENTRIES;
