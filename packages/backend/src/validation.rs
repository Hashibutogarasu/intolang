use anyhow::{Result, anyhow};

use crate::lexer::{TokenKind, tokenize};

/// Reject nested function declarations, which Into does not permit.
pub(super) fn function_scopes(source: &str, source_path: &str) -> Result<()> {
    let tokens = tokenize(source)?;
    let mut brace_depth = 0usize;
    let mut function_depths = Vec::new();
    let mut function_body_pending = false;
    for token in tokens {
        match token.kind {
            TokenKind::Identifier(name) if name == "fun" => {
                if !function_depths.is_empty() {
                    return Err(anyhow!(
                        "Function declarations cannot be nested inside another function\n\nInto language backtrace:\n  0: function body at {source_path}:{}",
                        token.line,
                    ));
                }
                function_body_pending = true;
            }
            TokenKind::Symbol('{') => {
                brace_depth += 1;
                if function_body_pending {
                    function_depths.push(brace_depth);
                    function_body_pending = false;
                }
            }
            TokenKind::Symbol('}') => {
                if function_depths.last() == Some(&brace_depth) { function_depths.pop(); }
                brace_depth = brace_depth.saturating_sub(1);
            }
            _ => {}
        }
    }
    Ok(())
}
