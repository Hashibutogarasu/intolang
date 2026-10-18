use parser::Record;

/// The result of evaluating Into source: the lowered message the compiler
/// consumes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evaluation {
    pub message: String,
}

impl Evaluation {
    /// Whether the message contains `text`.
    pub fn contains(&self, text: &str) -> bool {
        self.message.contains(text)
    }

    /// The compiler directive lines of the message, parsed.
    pub fn records(&self) -> Vec<Record> {
        self.message
            .split_inclusive('\n')
            .filter_map(|line| line.parse::<Record>().ok())
            .collect()
    }

    /// The program output the message prints without running anything: every
    /// line that is not a compiler directive.
    pub fn text(&self) -> String {
        self.message
            .split_inclusive('\n')
            .filter(|line| line.parse::<Record>().is_err())
            .collect()
    }
}

impl std::fmt::Display for Evaluation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
