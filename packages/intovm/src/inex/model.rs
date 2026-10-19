use anyhow::{Result, anyhow};
use bundle::Bundle;

use super::layout::INTOC;

/// An Into executable the VM runs: the files of the project, among them the
/// program `intoc`, and the tool that wrote them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inex {
    pub generator: String,
    pub bundle: Bundle,
}

impl Inex {
    /// The program the VM runs, the lowered message of the project.
    pub fn intoc(&self) -> Result<&str> {
        let bytes = self
            .bundle
            .read(INTOC)
            .ok_or_else(|| anyhow!("inex has no intoc"))?;
        std::str::from_utf8(bytes).map_err(|_| anyhow!("The intoc of the inex is not text"))
    }
}
