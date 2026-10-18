/// How a declared variable is allocated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Storage {
    /// `into x = ...`: scope-local.
    Local,
    /// `now into x = ...` (also `const now into`): allocated immediately.
    Now,
    /// `late into x = ...`: lazily allocated.
    Late,
}
