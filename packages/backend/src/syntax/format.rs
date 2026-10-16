/// Source formatting responsibility for a single syntactic construct.
///
/// Each construct reports the canonical text of the lines it owns, keeping
/// formatting rules next to the parsing rules for that syntax.
pub trait Format {
    /// Return the canonical text for `line` (already trimmed) when this
    /// construct recognises it, or `None` to defer to another construct.
    fn format(&self, line: &str) -> Option<String>;
}
