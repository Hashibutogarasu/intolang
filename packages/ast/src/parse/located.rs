use types::Span;

/// Where a node was written. It never takes part in comparisons, so two
/// nodes that differ only in their position are equal.
#[derive(Debug, Clone, Copy, Default)]
pub struct Located(pub Span);

impl PartialEq for Located {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Eq for Located {}

impl From<Span> for Located {
    fn from(span: Span) -> Self {
        Self(span)
    }
}
