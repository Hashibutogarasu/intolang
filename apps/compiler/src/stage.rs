/// The stages of making a native executable, reported as each one starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Codegen,
    Native,
    Link,
}

impl Stage {
    pub const COUNT: u64 = 3;

    pub fn message(self) -> &'static str {
        match self {
            Stage::Codegen => "Generating machine code",
            Stage::Native => "Compiling native sources",
            Stage::Link => "Linking the executable",
        }
    }
}
