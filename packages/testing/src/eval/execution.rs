/// What a compiled Into program did when it ran.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Execution {
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
    pub code: Option<i32>,
}
