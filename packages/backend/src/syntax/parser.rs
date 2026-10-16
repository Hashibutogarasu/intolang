use anyhow::Result;

use crate::syntax::context::ParseContext;
use crate::syntax::format::Format;
use crate::syntax::parse::{Cursor, Line, Parse};

/// A parser is nothing more than an ordered list of syntax constructs.
///
/// It runs the list over the prepared source, lets each construct consume the
/// lines it owns, and returns the accumulated lowered message.
pub struct Parser {
    constructs: Vec<Box<dyn Parse>>,
}

impl Parser {
    pub fn new(constructs: Vec<Box<dyn Parse>>) -> Self {
        Self { constructs }
    }

    pub fn parse(&self, ctx: &mut ParseContext, lines: &[Line]) -> Result<()> {
        let mut cursor = Cursor::new(lines);
        while !cursor.at_end() {
            let mut matched = false;
            for construct in &self.constructs {
                if construct.parse(ctx, &mut cursor)? {
                    matched = true;
                    break;
                }
            }
            if !matched {
                cursor.advance(1);
            }
        }
        Ok(())
    }

    /// Format a single source line using the first construct that owns it.
    pub fn format(&self, line: &str) -> Option<String> {
        self.constructs
            .iter()
            .find_map(|construct| format_construct(construct.as_ref(), line))
    }
}

fn format_construct(construct: &dyn Parse, line: &str) -> Option<String> {
    (construct as &dyn Format).format(line)
}
