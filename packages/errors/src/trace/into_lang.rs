use super::{Frame, Layer, StackTrace};

/// A frame of the Into language backtrace: the scope, the source path and the line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntoFrame {
    pub scope: String,
    pub path: String,
    pub line: usize,
}

/// The frames an error has in the Into program, innermost first.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IntoLangStackTrace {
    pub frames: Vec<IntoFrame>,
}

impl IntoLangStackTrace {
    pub fn at(scope: impl Into<String>, path: impl Into<String>, line: usize) -> Self {
        Self {
            frames: vec![IntoFrame {
                scope: scope.into(),
                path: path.into(),
                line,
            }],
        }
    }

    pub fn push(&mut self, scope: impl Into<String>, path: impl Into<String>, line: usize) {
        self.frames.push(IntoFrame {
            scope: scope.into(),
            path: path.into(),
            line,
        });
    }
}

impl StackTrace for IntoLangStackTrace {
    fn layer(&self) -> Layer {
        Layer::Into
    }

    fn frames(&self) -> Vec<Frame> {
        self.frames
            .iter()
            .map(|frame| Frame::at(&frame.scope, format!("{}:{}", frame.path, frame.line)))
            .collect()
    }
}
