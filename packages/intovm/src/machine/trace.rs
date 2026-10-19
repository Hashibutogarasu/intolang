/// Something the VM did that a test may want to see in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceEntry {
    /// The binding gave up its instance because it was destroyed.
    Destroyed(String),
    /// The event was emitted.
    Emitted(String),
}
