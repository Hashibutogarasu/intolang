use anyhow::Result;
use ast::{Expr, Path};
use native::NativeArg;
use types::{Literal, MessageTag};

use crate::context::Lowerer;
use crate::emit;
use crate::expr::{RuntimeCall, base_type_name};

/// The builtin functions that write to the console.
const PRINT: &str = "print";
const PRINTLN: &str = "println";

impl Lowerer<'_> {
    /// Lower an expression used as a statement. Only calls have an effect.
    pub fn lower_expression_statement(&mut self, line: usize, expression: &Expr) -> Result<()> {
        if let Expr::Await(awaited) = expression {
            return self
                .lower_await(awaited)
                .map_err(|message| self.diagnostic(line, &message));
        }
        let Expr::Call { callee, args } = expression else {
            return Ok(());
        };
        if callee.is_member() {
            return self.lower_method_call(line, callee, args);
        }
        let env = self.env;
        let name = callee.first();
        let console = name == PRINT || name == PRINTLN;
        if self
            .lower_indirect_call(name, args)
            .map_err(|message| self.diagnostic(line, &message))?
        {
            return Ok(());
        }
        if let Some(call) = self.callable_path(name) {
            return self.lower_method_call(line, &call, args);
        }
        if self
            .lower_event_handler_call(name, args)
            .map_err(|message| self.diagnostic(line, &message))?
        {
            return Ok(());
        }
        let key = self
            .pick_overload(name, args)
            .map_err(|message| self.diagnostic(line, &message))?;
        if let Some(inner) = env.user_functions.get(&key) {
            return self
                .inline_user_function(inner, None, args, 0)
                .map(|_| ())
                .map_err(|message| self.diagnostic(line, &message));
        }
        if env.imported_functions.contains(name)
            && let Some(link) = env.native_links.get(&key)
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
        if env.strict_calls {
            return Err(self.diagnostic(line, &format!("Function `{name}` is not imported")));
        }
        Ok(())
    }

    fn lower_method_call(&mut self, line: usize, callee: &Path, args: &[Expr]) -> Result<()> {
        let env = self.env;
        let fail = |this: &Self, message: String| this.diagnostic(line, &message);
        let handled = self
            .lower_event_call(callee, args, "")
            .map_err(|message| fail(self, message))?;
        if !matches!(handled, RuntimeCall::NotRuntime) {
            return Ok(());
        }
        if let Some((class, method)) = self.static_receiver(callee) {
            let key = self.method_key(&class, &method);
            let key = self
                .pick_overload(&key, args)
                .map_err(|message| fail(self, message))?;
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
        let class = base_type_name(&receiver_type.name()).to_owned();
        let method = callee.last();
        if self
            .lower_list_method(&receiver, &receiver_type, method, args, "")
            .map_err(|message| fail(self, message))?
            .is_some()
        {
            return Ok(());
        }
        if self
            .lower_pointer_method(&receiver, &receiver_type, method, args, "")
            .map_err(|message| fail(self, message))?
            .is_some()
        {
            return Ok(());
        }
        self.check_receiver_call(&receiver, method)
            .map_err(|message| fail(self, message))?;
        if self.give_back_reference(&receiver, method) {
            return Ok(());
        }
        let key = self.method_key(&class, method);
        let key = self
            .pick_overload(&key, args)
            .map_err(|message| fail(self, message))?;
        if let Some(inner) = env.user_functions.get(&key) {
            self.inline_user_function(inner, Some(receiver.clone()), args, 0)
                .map_err(|message| fail(self, message))?;
            self.apply_receiver_call(&receiver, method);
            return Ok(());
        }
        let Some(link) = env.native_links.get(&key) else {
            if self.is_ownership_method(&class, method) {
                self.apply_receiver_call(&receiver, method);
                return Ok(());
            }
            return Err(fail(self, format!("Undefined method `{key}`")));
        };
        let mut combined = vec![Expr::ident(receiver.clone())];
        combined.extend_from_slice(args);
        self.emit_link_call(MessageTag::LinkAssign, link, combined, "", true)
            .map_err(|message| fail(self, message))?;
        self.apply_receiver_call(&receiver, method);
        Ok(())
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
