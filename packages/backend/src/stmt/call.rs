use anyhow::Result;
use ast::{Expr, Path};
use native::NativeArg;
use types::{Literal, MessageTag};

use crate::context::Lowerer;
use crate::emit;
use crate::expr::base_type_name;

/// The builtin functions that write to the console.
const PRINT: &str = "print";
const PRINTLN: &str = "println";

impl Lowerer<'_> {
    /// Lower an expression used as a statement. Only calls have an effect.
    pub fn lower_expression_statement(&mut self, line: usize, expression: &Expr) -> Result<()> {
        let Expr::Call { callee, args } = expression else {
            return Ok(());
        };
        if callee.is_member() {
            return self.lower_method_call(line, callee, args);
        }
        let env = self.env;
        let name = callee.first();
        let console = name == PRINT || name == PRINTLN;
        if let Some(inner) = env.user_functions.get(name) {
            return self
                .inline_user_function(inner, None, args, 0)
                .map(|_| ())
                .map_err(|message| self.diagnostic(line, &message));
        }
        if env.imported_functions.contains(name)
            && let Some(link) = env.native_links.get(name)
        {
            return self
                .emit_link_call(MessageTag::LinkAssign, link, args.to_vec(), "", false)
                .map_err(|message| self.diagnostic(line, &message));
        }
        if !console && env.imported_functions.contains(name) && args.len() <= 1 {
            return self.lower_native_call(line, name, args.first());
        }
        if console {
            return self.lower_print(line, name == PRINTLN, args);
        }
        Ok(())
    }

    fn lower_method_call(&mut self, line: usize, callee: &Path, args: &[Expr]) -> Result<()> {
        let env = self.env;
        let fail = |this: &Self, message: String| this.diagnostic(line, &message);
        if let Some((class, method)) = self.static_receiver(callee) {
            let key = format!("{class}.{method}");
            if let Some(inner) = env.user_functions.get(&key) {
                return self
                    .inline_user_function(inner, None, args, 0)
                    .map(|_| ())
                    .map_err(|message| fail(self, message));
            }
            let link = env
                .native_links
                .get(&key)
                .ok_or_else(|| fail(self, format!("Undefined static method `{key}`")))?;
            return self
                .emit_link_call(MessageTag::LinkAssign, link, args.to_vec(), "", false)
                .map_err(|message| fail(self, message));
        }
        let (receiver, receiver_type) = self
            .resolve_receiver(callee)
            .map_err(|message| fail(self, message))?;
        let key = format!(
            "{}.{}",
            base_type_name(&receiver_type.name()),
            callee.last()
        );
        if let Some(inner) = env.user_functions.get(&key) {
            return self
                .inline_user_function(inner, Some(receiver), args, 0)
                .map(|_| ())
                .map_err(|message| fail(self, message));
        }
        let link = env
            .native_links
            .get(&key)
            .ok_or_else(|| fail(self, format!("Undefined method `{key}`")))?;
        let mut combined = vec![Expr::ident(receiver)];
        combined.extend_from_slice(args);
        self.emit_link_call(MessageTag::LinkAssign, link, combined, "", true)
            .map_err(|message| fail(self, message))
    }

    /// A call to an imported function without a `#[link]` binding, lowered to
    /// a plain `NATIVE` record that takes one string or 32-bit integer.
    fn lower_native_call(
        &mut self,
        line: usize,
        symbol: &str,
        argument: Option<&Expr>,
    ) -> Result<()> {
        let env = self.env;
        let value = match argument {
            Some(Expr::Literal(Literal::String(text))) => Some(NativeArg::from(text.as_str())),
            Some(Expr::Literal(Literal::Int(number))) => {
                i32::try_from(*number).ok().map(NativeArg::Int)
            }
            Some(Expr::Path(path)) if path.is_member() => {
                let variant = path.segments()[1..].join(".");
                let found = env
                    .enum_values
                    .get(path.first())
                    .and_then(|variants| variants.get(&variant));
                match found {
                    Some(value) => Some(NativeArg::Int(*value)),
                    None => {
                        return Err(
                            self.diagnostic(line, &format!("Undefined enum variant `{path}`"))
                        );
                    }
                }
            }
            Some(Expr::Path(path)) => self
                .state
                .values
                .get(path.first())
                .map(|variable| NativeArg::from(variable.value.as_str())),
            _ => None,
        };
        let Some(value) = value else {
            return Err(self.diagnostic(
                line,
                &format!("Native call `{symbol}` expects a string or 32-bit integer argument"),
            ));
        };
        emit::native_call(&mut self.state.output, symbol, &value);
        Ok(())
    }
}
