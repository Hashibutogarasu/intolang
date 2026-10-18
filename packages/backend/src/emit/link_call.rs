use ast::Expr;
use native::{LinkCall, NativeArg, NativeLink};
use types::{Literal, MessageTag};

use crate::context::Lowerer;

impl Lowerer<'_> {
    /// Emit a call to the native function `link` binds. `args` holds the
    /// receiver first when `has_receiver` is set; omitted trailing arguments
    /// take their parameters' defaults. `tag` selects the record kind, such as
    /// `LinkAssign` for a plain call or `IfCall` for a branch condition.
    pub fn emit_link_call(
        &mut self,
        tag: MessageTag,
        link: &NativeLink,
        mut args: Vec<Expr>,
        assigned_to: &str,
        has_receiver: bool,
    ) -> Result<(), String> {
        let receivers = usize::from(has_receiver);
        let provided = args.len().saturating_sub(receivers);
        if provided > link.params.len() {
            return Err(format!(
                "Function `{}` expects {} argument(s) but received {}",
                link.target,
                link.params.len(),
                provided
            ));
        }
        for param in &link.params[provided..] {
            match param.default_argument() {
                Some(default) => args.push(default),
                None => return Err(format!("Missing required argument `{}`", param.name)),
            }
        }
        let call = LinkCall {
            package_alias: link.package_alias.clone(),
            language: link.language,
            source_path: link.source_path.clone(),
            target: link.target.clone(),
            return_type: link.return_type.clone(),
            assigned_to: assigned_to.to_owned(),
            args: args
                .iter()
                .map(|arg| self.native_arg(arg))
                .collect::<Result<_, _>>()?,
        };
        self.state.output.push_str(&call.to_record(tag).to_string());
        Ok(())
    }

    fn native_arg(&self, arg: &Expr) -> Result<NativeArg, String> {
        match arg {
            Expr::Literal(Literal::String(value)) => Ok(NativeArg::from(value.as_str())),
            Expr::Literal(Literal::Int(number)) => i32::try_from(*number)
                .map(NativeArg::Int)
                .map_err(|_| format!("Unsupported native argument `{arg}`")),
            Expr::Literal(Literal::Bool(flag)) => Ok(NativeArg::Int(i32::from(*flag))),
            Expr::Path(path) if path.is_member() => {
                let variant = path.segments()[1..].join(".");
                self.env
                    .enum_values
                    .get(path.first())
                    .and_then(|variants| variants.get(&variant))
                    .map(|value| NativeArg::Int(*value))
                    .ok_or_else(|| format!("Undefined enum variant `{arg}`"))
            }
            Expr::Path(path) => {
                let name = path.first();
                match self.state.values.get(name) {
                    Some(variable) if variable.native => Ok(NativeArg::Slot(name.to_owned())),
                    Some(variable) => Ok(NativeArg::from(variable.value.as_str())),
                    None => Err(format!("Unsupported native argument `{arg}`")),
                }
            }
            _ => Err(format!("Unsupported native argument `{arg}`")),
        }
    }
}
