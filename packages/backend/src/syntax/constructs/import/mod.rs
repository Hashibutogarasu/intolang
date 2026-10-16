use anyhow::Result;

use crate::syntax::context::ParseContext;
use crate::syntax::format::Format;
use crate::syntax::parse::{Cursor, Parse};

/// `import "scheme://module";` — resolved during preparation, ignored here.
pub struct Import;

impl Format for Import {
    fn format(&self, line: &str) -> Option<String> {
        if line.starts_with("import ") {
            Some(line.to_owned())
        } else {
            None
        }
    }
}

impl Parse for Import {
    fn parse(&self, _ctx: &mut ParseContext, cursor: &mut Cursor<'_>) -> Result<bool> {
        match cursor.line() {
            Some(line) if line.code.starts_with("import ") || line.code.is_empty() => {
                cursor.advance(1);
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}
