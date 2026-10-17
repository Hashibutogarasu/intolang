use anyhow::Result;

use crate::syntax::context::ParseContext;
use crate::syntax::format::Format;
use crate::syntax::parse::{Cursor, Parse};
use crate::syntax::support::emit::emit_module;
use moduler::{parse_export, strip_keyword};

/// `export "scheme://module";` — when the URI is recognised a `Module`
/// instance describing the exported module is created in the lowered message.
pub struct Export;

impl Format for Export {
    fn format(&self, line: &str) -> Option<String> {
        if strip_keyword(line, "export").is_some() {
            Some(line.to_owned())
        } else {
            None
        }
    }
}

impl Parse for Export {
    fn parse(&self, ctx: &mut ParseContext, cursor: &mut Cursor<'_>) -> Result<bool> {
        let Some(line) = cursor.line() else {
            return Ok(false);
        };
        let code = line.code.clone();
        if strip_keyword(&code, "export").is_none() {
            return Ok(false);
        }
        if let Some(spec) = parse_export(&code)
            && let Some(index) = ctx
                .modules
                .iter()
                .position(|module| module.alias == spec.alias && module.name == spec.path)
        {
            let module = ctx.modules[index].clone();
            emit_module(&mut ctx.output, index, &module);
        }
        cursor.advance(1);
        Ok(true)
    }
}
