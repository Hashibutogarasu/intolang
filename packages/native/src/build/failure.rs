use errors::{Error, Errors, NativeStackTrace};

/// The error of a native tool that ran and failed: its output is the native layer of the
/// stack trace.
pub(super) fn tool_failure(kind: Errors, tool: &str, stderr: &[u8]) -> Error {
    Error::compiler(kind)
        .with_native(NativeStackTrace::new(tool).with_output(String::from_utf8_lossy(stderr)))
}

/// The error of a native tool that could not be started.
pub(super) fn tool_not_run(tool: &str, reason: &std::io::Error) -> Error {
    Errors::ToolNotRun {
        tool: tool.to_string(),
        reason: reason.to_string(),
    }
    .into()
}
