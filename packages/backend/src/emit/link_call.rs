use ast::Expr;
use errors::{Errors, Result};
use native::{LinkCall, NativeArg, NativeLink};
use types::{Literal, MessageTag};

use crate::context::{Lowerer, Ownership};
use crate::expr::{MOVE, base_type_name, parse_type};

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
    ) -> Result<()> {
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
            return Err(Errors::FunctionArgumentCount {
                function: link.target.clone(),
                expected: link.params.len(),
                received: provided,
            }
            .into());
        }
        for param in &link.params[provided..] {
            match param.default_argument() {
                Some(default) => args.push(default),
                None => {
                    return Err(Errors::MissingArgument {
                        name: param.name.clone(),
                    }
                    .into());
                }
            }
        }
        let mut native = Vec::new();
        for (position, arg) in args.iter().enumerate() {
            let computed = matches!(
                arg,
                Expr::Binary { .. }
                    | Expr::Cast { .. }
                    | Expr::Compare { .. }
                    | Expr::Logical { .. }
                    | Expr::Not(_)
                    | Expr::Call { .. }
            );
            let param = position
                .checked_sub(receivers)
                .and_then(|index| link.params.get(index));
            let written = param.and_then(|param| param.ty.as_ref());
            native.push(match (computed, written) {
                (true, Some(written)) => {
                    let ty = parse_type(written.as_str(), &self.env.type_params)?;
                    self.runtime_argument(arg, &ty)?
                }
                _ => self.native_arg(arg)?,
            });
        }
        let call = LinkCall {
            package_alias: link.package_alias.clone(),
            language: link.language,
            source_path: link.source_path.clone(),
            target: link.target.clone(),
            return_type: link.return_type.clone(),
            assigned_to: assigned_to.to_owned(),
            args: native,
        };
        self.state.output.push_str(&call.to_record(tag).to_string());
        for name in handed_over {
            self.set_state(&name, Ownership::Moved);
        }
        Ok(())
    }

    pub fn native_arg(&self, arg: &Expr) -> Result<NativeArg> {
        match arg {
            Expr::Literal(Literal::String(value)) => Ok(NativeArg::from(value.as_str())),
            Expr::Literal(Literal::Int(number)) => {
                i32::try_from(*number).map(NativeArg::Int).map_err(|_| {
                    errors::Error::from(Errors::UnsupportedNativeArgument {
                        arg: arg.to_string(),
                    })
                })
            }
            Expr::Literal(Literal::Bool(flag)) => Ok(NativeArg::Int(i32::from(*flag))),
            Expr::Literal(Literal::Char(value)) => i32::try_from(u32::from(*value))
                .map(NativeArg::Int)
                .map_err(|_| {
                    errors::Error::from(Errors::UnsupportedNativeArgument {
                        arg: arg.to_string(),
                    })
                }),
            Expr::Path(path) if path.is_member() => {
                if let Some(field) = self.static_field(path) {
                    return match field.default_literal() {
                        Some(Literal::Int(number)) => {
                            i32::try_from(*number).map(NativeArg::Int).map_err(|_| {
                                errors::Error::from(Errors::UnsupportedNativeArgument {
                                    arg: arg.to_string(),
                                })
                            })
                        }
                        Some(Literal::String(value)) => Ok(NativeArg::from(value.as_str())),
                        Some(Literal::Bool(flag)) => Ok(NativeArg::Int(i32::from(*flag))),
                        _ => Err(Errors::StaticFieldNoDefault {
                            field: arg.to_string(),
                        }
                        .into()),
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
                    .ok_or_else(|| {
                        errors::Error::from(Errors::UndefinedEnumVariant {
                            arg: arg.to_string(),
                        })
                    })
            }
            Expr::Path(path) => {
                let name = path.first();
                self.ensure_usable(name)?;
                match self.state.values.get(name) {
                    Some(variable) if variable.native => Ok(NativeArg::Slot(name.to_owned())),
                    Some(variable) => Ok(NativeArg::from(variable.value.as_str())),
                    None => Err(Errors::UnsupportedNativeArgument {
                        arg: arg.to_string(),
                    }
                    .into()),
                }
            }
            _ => Err(Errors::UnsupportedNativeArgument {
                arg: arg.to_string(),
            }
            .into()),
        }
    }
}
