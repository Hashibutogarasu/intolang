use ast::Expr;
use native::{LinkCall, NativeArg, NativeLink};
use types::{Literal, MessageTag};

use crate::context::{Lowerer, Ownership};
use crate::expr::{MOVE, base_type_name};

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
        let mut handed_over = Vec::new();
        for arg in &mut args {
            let moves = matches!(
                arg,
                Expr::Call { callee, args } if args.is_empty() && callee.last() == MOVE
            );
            *arg = self.materialize_argument(arg.clone())?;
            if moves && let Some(name) = arg.as_ident() {
                handed_over.push(name.to_owned());
            }
        }
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
        for name in handed_over {
            self.set_state(&name, Ownership::Moved);
        }
        Ok(())
    }

    pub fn native_arg(&self, arg: &Expr) -> Result<NativeArg, String> {
        match arg {
            Expr::Literal(Literal::String(value)) => Ok(NativeArg::from(value.as_str())),
            Expr::Literal(Literal::Int(number)) => i32::try_from(*number)
                .map(NativeArg::Int)
                .map_err(|_| format!("Unsupported native argument `{arg}`")),
            Expr::Literal(Literal::Bool(flag)) => Ok(NativeArg::Int(i32::from(*flag))),
            Expr::Path(path) if path.is_member() => {
                if let Some(field) = self.static_field(path) {
                    return match field.default_literal() {
                        Some(Literal::Int(number)) => i32::try_from(*number)
                            .map(NativeArg::Int)
                            .map_err(|_| format!("Unsupported native argument `{arg}`")),
                        Some(Literal::String(value)) => Ok(NativeArg::from(value.as_str())),
                        Some(Literal::Bool(flag)) => Ok(NativeArg::Int(i32::from(*flag))),
                        _ => Err(format!("Static field `{arg}` has no constant default")),
                    };
                }
                if let [name, member] = path.segments()
                    && let Some(variable) = self.state.values.get(name)
                    && variable.native
                {
                    self.ensure_usable(name)?;
                    let class = base_type_name(&variable.ty.name()).to_owned();
                    let position = self.env.class_fields.get(&class).and_then(|fields| {
                        fields
                            .iter()
                            .filter(|field| !field.is_static)
                            .position(|field| field.name == *member)
                    });
                    if let Some(index) = position {
                        return Ok(NativeArg::Field {
                            slot: name.clone(),
                            class,
                            index,
                        });
                    }
                }
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
                self.ensure_usable(name)?;
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
