use anyhow::Result;

use crate::syntax::context::ParseContext;
use crate::syntax::format::Format;
use crate::syntax::parse::{Cursor, Parse};
use crate::syntax::support::emit::emit_module;
use moduler::{parse_import, strip_keyword};

/// `import "scheme://module";` — resolved during preparation. When the URI is
/// recognised a `Module` instance describing the resolved module is created in
/// the lowered message.
pub struct Import;

impl Format for Import {
    fn format(&self, line: &str) -> Option<String> {
        if strip_keyword(line, "import").is_some() {
            Some(line.to_owned())
        } else {
            None
        }
    }
}

impl Parse for Import {
    fn parse(&self, ctx: &mut ParseContext, cursor: &mut Cursor<'_>) -> Result<bool> {
        let Some(line) = cursor.line() else {
            return Ok(false);
        };
        let code = line.code.clone();
        if code.is_empty() {
            cursor.advance(1);
            return Ok(true);
        }
        if strip_keyword(&code, "import").is_none() {
            return Ok(false);
        }
        if let Some(spec) = parse_import(&code)
            && let Some(index) = ctx
                .modules
                .iter()
                .position(|module| module.alias == spec.alias && module.name == spec.module)
        {
            let module = ctx.modules[index].clone();
            emit_module(&mut ctx.output, index, &module);
        }
        cursor.advance(1);
        Ok(true)
    }
}
