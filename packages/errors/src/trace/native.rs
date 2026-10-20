use super::{Frame, Layer, StackTrace};

/// What a native tool or library reported: the C compiler, the linker, `pkg-config`,
/// `dlerror`, LLVM, or the signal a native call ended with.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NativeStackTrace {
    pub frames: Vec<Frame>,
    pub output: Option<String>,
}

impl NativeStackTrace {
    pub fn new(tool: impl Into<String>) -> Self {
        Self {
            frames: vec![Frame::new(tool)],
            output: None,
        }
    }

    pub fn with_output(mut self, output: impl Into<String>) -> Self {
        let output = output.into();
        self.output = (!output.trim().is_empty()).then_some(output);
        self
    }

    pub fn at(mut self, label: impl Into<String>, location: impl Into<String>) -> Self {
        self.frames.push(Frame::at(label, location));
        self
    }
}

impl StackTrace for NativeStackTrace {
    fn layer(&self) -> Layer {
        Layer::Native
    }

    fn frames(&self) -> Vec<Frame> {
        self.frames.clone()
    }

    fn detail(&self) -> Option<String> {
        self.output.clone()
    }
}
