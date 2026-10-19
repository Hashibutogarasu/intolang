use anyhow::Result;
use ast::{Expr, Stmt};
use types::{MessageTag, Type};

use crate::context::Lowerer;
use crate::emit;
use crate::expr::state_check;

impl Lowerer<'_> {
    pub fn lower_if(
        &mut self,
        line: usize,
        condition: &Expr,
        then_body: &[Stmt],
        else_body: Option<&[Stmt]>,
    ) -> Result<()> {
        self.begin_condition(
            line,
            "if",
            MessageTag::IfBegin,
            MessageTag::IfCall,
            condition,
        )?;
        self.lower_scope(line, then_body)?;
        if let Some(else_body) = else_body {
            emit::bare(&mut self.state.output, MessageTag::IfElse);
            self.lower_scope(line, else_body)?;
        }
        emit::bare(&mut self.state.output, MessageTag::IfEnd);
        Ok(())
    }

    pub fn lower_while(&mut self, line: usize, condition: &Expr, body: &[Stmt]) -> Result<()> {
        let general = self.is_general_condition(condition);
        let flag = if general {
            Some(
                self.begin_boolean_branch(MessageTag::WhileBegin, condition)
                    .map_err(|message| self.diagnostic(line, &message))?,
            )
        } else {
            self.begin_condition(
                line,
                "while",
                MessageTag::WhileBegin,
                MessageTag::LoopBegin,
                condition,
            )?;
            None
        };
        self.state.loop_depth += 1;
        self.state.loop_scopes.push(self.state.scopes.len());
        let lowered = self.lower_scope(line, body);
        self.state.loop_scopes.pop();
        self.state.loop_depth -= 1;
        lowered?;
        if let Some(flag) = flag {
            let next = self
                .lower_boolean(condition)
                .map_err(|message| self.diagnostic(line, &message))?;
            emit::assign(&mut self.state.output, &flag, &Type::Boolean, "p", &next);
        }
        emit::bare(&mut self.state.output, MessageTag::LoopEnd);
        Ok(())
    }

    /// Whether a comparison names a variable and a literal or another
    /// variable, which a comparison record can express directly.
    pub(crate) fn is_plain_comparison(left: &Expr, right: &Expr) -> bool {
        let simple = |expression: &Expr| {
            expression.as_ident().is_some() || matches!(expression, Expr::Literal(_))
        };
        simple(left) && simple(right) && (left.as_ident().is_some() || right.as_ident().is_some())
    }

    /// Whether the condition has to be computed into a `Boolean` slot first.
    fn is_general_condition(&self, condition: &Expr) -> bool {
        if state_check(condition).is_some() {
            return false;
        }
        match condition {
            Expr::Compare { left, right, .. } => !Self::is_plain_comparison(left, right),
            Expr::Call { .. } => false,
            _ => true,
        }
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
            _ if self.begin_state_check(condition) => Ok(()),
            Expr::Compare { left, op, right } if Self::is_plain_comparison(left, right) => self
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
            _ => self
                .begin_boolean_branch(compare_tag, condition)
                .map(|_| ())
                .map_err(|message| self.diagnostic(line, &format!("{label} condition: {message}"))),
        }
    }
}
