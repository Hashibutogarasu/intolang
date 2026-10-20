use errors::Result;
use std::fmt;
use types::Punct;

use crate::Cursor;

/// A dotted name such as `wayland.window.create`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Path(Vec<String>);

impl Path {
    pub fn single(name: impl Into<String>) -> Self {
        Self(vec![name.into()])
    }

    pub fn segments(&self) -> &[String] {
        &self.0
    }

    pub fn first(&self) -> &str {
        &self.0[0]
    }

    pub fn last(&self) -> &str {
        &self.0[self.0.len() - 1]
    }

    /// The name when the path has exactly one segment.
    pub fn as_ident(&self) -> Option<&str> {
        match self.0.as_slice() {
            [name] => Some(name),
            _ => None,
        }
    }

    pub fn is_member(&self) -> bool {
        self.0.len() > 1
    }

    /// This path with its first segment replaced by `receiver`'s segments.
    pub fn rebased(&self, receiver: &Path) -> Path {
        let mut segments = receiver.0.clone();
        segments.extend(self.0[1..].iter().cloned());
        Path(segments)
    }
}

impl From<Vec<String>> for Path {
    fn from(segments: Vec<String>) -> Self {
        Self(segments)
    }
}

impl TryFrom<&mut Cursor<'_>> for Path {
    type Error = errors::Error;

    fn try_from(cursor: &mut Cursor<'_>) -> Result<Self> {
        let (first, _) = cursor.expect_ident("a name")?;
        let mut segments = vec![first];
        while cursor.is_punct(Punct::Dot) && cursor.peek_word(1).is_some() {
            cursor.bump();
            let (segment, _) = cursor.expect_ident("a member name")?;
            segments.push(segment);
        }
        Ok(Self(segments))
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0.join("."))
    }
}
