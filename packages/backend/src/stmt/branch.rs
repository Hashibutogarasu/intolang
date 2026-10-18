use anyhow::Result;
use ast::{Expr, Stmt};
use types::MessageTag;

use crate::context::Lowerer;
use crate::emit;

impl Lowerer<'_> {
    pub fn lower_if(
        &mut self,
        line: usize,
        condition: &Expr,
        then_body: &[Stmt],
        else_body: Option<&[Stmt]>,
    ) -> Result<()> {
        self.begin_condition(line, "if", MessageTag::IfBegin, MessageTag::IfCall, condition)?;
        self.lower_block(then_body)?;
        if let Some(else_body) = else_body {
            emit::bare(&mut self.state.output, MessageTag::IfElse);
            self.lower_block(else_body)?;
        }
        emit::bare(&mut self.state.output, MessageTag::IfEnd);
        Ok(())
    }

    pub fn lower_while(&mut self, line: usize, condition: &Expr, body: &[Stmt]) -> Result<()> {
        self.begin_condition(
            line,
            "while",
            MessageTag::WhileBegin,
            MessageTag::LoopBegin,
            condition,
        )?;
        self.state.loop_depth += 1;
        let lowered = self.lower_block(body);
        self.state.loop_depth -= 1;
        lowered?;
        emit::bare(&mut self.state.output, MessageTag::LoopEnd);
        Ok(())
    }

    /// Emit the opening record of an `if` or `while`: a numeric comparison
    /// (`compare_tag`) or a call to a function returning `Boolean`
    /// (`call_tag`).
    fn begin_condition(
        &mut self,
        line: usize,
        label: &str,
        compare_tag: MessageTag,
        call_tag: MessageTag,
        condition: &Expr,
    ) -> Result<()> {
        let env = self.env;
        match condition {
            Expr::Compare { left, op, right } => self
                .begin_comparison(compare_tag, label, left, *op, right)
                .map_err(|message| self.diagnostic(line, &message)),
            Expr::Call { callee, args } => {
                let (link, combined, has_receiver) = if callee.is_member() {
                    let (link, combined) = self
                        .resolve_method(callee, args)
                        .map_err(|message| self.diagnostic(line, &message))?;
                    (link, combined, true)
                } else {
                    let name = callee.first();
                    if !env.imported_functions.contains(name) {
                        return Err(
                            self.diagnostic(line, &format!("Function `{name}` is not imported"))
                        );
                    }
                    let link = env.native_links.get(name).ok_or_else(|| {
                        self.diagnostic(line, &format!("Function `{name}` has no native link"))
                    })?;
                    (link, args.to_vec(), false)
                };
                if link.return_type != "Boolean" {
                    return Err(self.diagnostic(
                        line,
                        &format!("{label} condition `{callee}` must return Boolean"),
                    ));
                }
                self.emit_link_call(call_tag, link, combined, "", has_receiver)
                    .map_err(|message| self.diagnostic(line, &message))
            }
            _ => Err(self.diagnostic(
                line,
                &format!("{label} condition must compare numeric values or call a Boolean function"),
            )),
        }
    }
}
