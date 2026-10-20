use ast::Expr;
use errors::{Errors, Result};
use native::{CastAssign, FunctionCall, FunctionDefinition, FunctionReturn, NativeArg, Operand};
use parser::Hex;
use std::mem;
use types::Type;

use super::numeric::Kind;
use super::text::Text;
use super::type_name::parse_type;
use crate::context::{Lowerer, Tracking, UserFunction, Variable};
use crate::emit;

/// The name a function called at run time has in the message.
fn runtime_name(key: &str) -> String {
    format!("__fn_{key}")
}

fn is_scalar(ty: &Type) -> bool {
    ty.is_numeric() || matches!(ty, Type::Boolean | Type::Char)
}

impl Lowerer<'_> {
    /// Whether the function is called at run time instead of being inlined.
    pub fn is_runtime_function(&self, function: &UserFunction) -> bool {
        self.env.runtime_functions.contains(&function.name)
    }

    /// The type the function returns, `Void` for none.
    fn runtime_return_type(&self, function: &UserFunction) -> Result<Type> {
        match function.return_type.as_deref() {
            None => Ok(Type::Void),
            Some(written) => parse_type(written, &self.env.type_params),
        }
    }

    /// The parameters of the function in order, the receiver first.
    fn runtime_params(&self, function: &UserFunction) -> Result<Vec<(String, Type)>> {
        let mut params = Vec::new();
        if let (Some(class), false) = (&function.receiver, function.is_static) {
            params.push(("this".to_owned(), Type::Named(class.clone())));
        }
        for param in &function.params {
            let written = param.ty.as_ref().ok_or_else(|| {
                errors::Error::from(Errors::ParameterNeedsType {
                    param: param.name.clone(),
                    function: function.name.clone(),
                })
            })?;
            let ty = parse_type(written.as_str(), &self.env.type_params)?;
            params.push((param.name.clone(), ty));
        }
        Ok(params)
    }

    /// Lower a call of a function that is called at run time. Returns the slot
    /// that holds the result, if the function has one.
    pub fn call_runtime_function(
        &mut self,
        function: &UserFunction,
        receiver: Option<String>,
        args: &[Expr],
    ) -> Result<Option<(String, Type)>> {
        self.define_runtime_function(function)?;
        let params = self.runtime_params(function)?;
        let mut arguments: Vec<Expr> = receiver.into_iter().map(Expr::ident).collect();
        arguments.extend_from_slice(args);
        if arguments.len() > params.len() {
            return Err(Errors::FunctionArgumentCount {
                function: function.name.clone(),
                expected: params.len(),
                received: arguments.len(),
            }
            .into());
        }
        let offset = params.len() - function.params.len();
        for param in &function.params[arguments.len().saturating_sub(offset)..] {
            match param.default_argument() {
                Some(default) => arguments.push(default),
                None => {
                    return Err(Errors::MissingArgument {
                        name: param.name.clone(),
                    }
                    .into());
                }
            }
        }
        let mut native = Vec::new();
        for ((_, ty), argument) in params.iter().zip(&arguments) {
            native.push(self.runtime_argument(argument, ty)?);
        }
        let return_type = self.runtime_return_type(function)?;
        let (target, returned) = if return_type == Type::Void {
            (String::new(), None)
        } else {
            let slot = self.fresh_slot(&return_type);
            (slot.clone(), Some((slot, return_type.clone())))
        };
        emit::function_call(
            &mut self.state.output,
            &FunctionCall {
                name: runtime_name(&function.name),
                target,
                return_type: return_type.name(),
                args: native,
            },
        );
        Ok(returned)
    }

    /// The argument that passes `argument` to a parameter of type `ty`.
    pub(crate) fn runtime_argument(&mut self, argument: &Expr, ty: &Type) -> Result<NativeArg> {
        if let Expr::Lambda { params, body } = argument {
            if !Self::is_function_type(ty) {
                return Err(Errors::LambdaNotAllowed {
                    expected: ty.name(),
                }
                .into());
            }
            return Ok(NativeArg::Slot(
                self.lower_lambda_value(params, body, ty, None)?,
            ));
        }
        if is_scalar(ty) {
            return Ok(NativeArg::Slot(self.scalar_slot(argument, ty)?));
        }
        if matches!(ty, Type::String | Type::Path) {
            let actual = self.type_of(argument).ok();
            let fits =
                actual == Some(Type::String) || (*ty == Type::Path && actual == Some(Type::Path));
            if !fits {
                return Err(Errors::ExpectedArgumentType {
                    expected: ty.name(),
                    argument: argument.to_string(),
                }
                .into());
            }
            return Ok(match self.lower_text(argument, None)? {
                Text::Const(text) => NativeArg::from(text.as_str()),
                Text::Slot(slot) => NativeArg::Slot(slot),
            });
        }
        if matches!(argument, Expr::Literal(_)) || argument.as_ident().is_some() {
            return self.native_arg(argument);
        }
        if let Ok(direct) = self.native_arg(argument) {
            return Ok(direct);
        }
        let env = self.env;
        let slot = self.fresh_slot(ty);
        let (_, built, native) =
            self.declaration_value(argument, Some(ty.clone()), &env.type_params, &slot)?;
        if !native {
            return Err(Errors::ExpectedArgumentType {
                expected: ty.name(),
                argument: argument.to_string(),
            }
            .into());
        }
        self.state
            .values
            .insert(slot.clone(), Variable::temporary(built));
        Ok(NativeArg::Slot(slot))
    }

    /// The slot that holds `value` converted to the scalar type `ty`.
    pub fn scalar_slot(&mut self, value: &Expr, ty: &Type) -> Result<String> {
        let declared = self.type_of(value)?;
        if !is_scalar(&declared) && declared != Type::Boolean {
            return Err(Errors::ExpectedType {
                expected: ty.name(),
                received: value.to_string(),
            }
            .into());
        }
        let numeric = self.lower_numeric(value)?;
        let received = match &numeric.kind {
            Kind::Typed(received) => received.clone(),
            Kind::IntLiteral => {
                if !ty.is_numeric() {
                    return Err(Errors::ExpectedType {
                        expected: ty.name(),
                        received: value.to_string(),
                    }
                    .into());
                }
                ty.clone()
            }
            Kind::FloatLiteral => {
                if !ty.is_float() {
                    return Err(Errors::ExpectedType {
                        expected: ty.name(),
                        received: value.to_string(),
                    }
                    .into());
                }
                ty.clone()
            }
        };
        let slot = self.slot_of(&numeric, &received);
        if received == *ty {
            return Ok(slot);
        }
        let convertible = (is_scalar(&received) || received == Type::Boolean) && is_scalar(ty);
        if !convertible || (received == Type::Char) != (*ty == Type::Char) {
            return Err(Errors::ExpectedTypeReceived {
                expected: ty.name(),
                received: received.name(),
            }
            .into());
        }
        let converted = self.fresh_slot(ty);
        emit::cast_assign(
            &mut self.state.output,
            &CastAssign {
                target: converted.clone(),
                source: slot,
                from: received.name(),
                to: ty.name(),
            },
        );
        Ok(converted)
    }

    /// Put the definition of a function called at run time in the output, once.
    fn define_runtime_function(&mut self, function: &UserFunction) -> Result<()> {
        if !self.state.defined_functions.insert(function.name.clone()) {
            return Ok(());
        }
        let params = self.runtime_params(function)?;
        let return_type = self.runtime_return_type(function)?;
        emit::function_begin(
            &mut self.state.output,
            &FunctionDefinition {
                name: runtime_name(&function.name),
                return_type: return_type.name(),
                params: params
                    .iter()
                    .map(|(name, ty)| (name.clone(), ty.name()))
                    .collect(),
            },
        );
        let values = mem::take(&mut self.state.values);
        let scopes = mem::take(&mut self.state.scopes);
        let loop_depth = mem::take(&mut self.state.loop_depth);
        let loop_scopes = mem::take(&mut self.state.loop_scopes);
        let frames = mem::take(&mut self.state.frames);
        let pending = self.state.pending_tracking.take();
        for (name, ty) in &params {
            let mut variable = Variable::temporary(ty.clone());
            variable.memory_backed = true;
            variable.tracking = Tracking {
                reference: true,
                ..Tracking::default()
            };
            self.state.values.insert(name.clone(), variable);
        }
        self.state.returning.push(return_type);
        self.state.scopes.push(Vec::new());
        let lowered = self.lower_block(&function.body);
        let owned = self.state.scopes.pop().unwrap_or_default();
        let released = match lowered {
            Ok(()) => self.release_scope(&owned, None),
            Err(error) => Err(error),
        };
        self.state.returning.pop();
        self.state.values = values;
        self.state.scopes = scopes;
        self.state.loop_depth = loop_depth;
        self.state.loop_scopes = loop_scopes;
        self.state.frames = frames;
        self.state.pending_tracking = pending;
        emit::handler_end(&mut self.state.output);
        released
    }

    /// Compute the value of a `return` of an inlined function into a slot of
    /// its own, and give the name of the slot.
    pub(crate) fn materialize_return(&mut self, expression: &Expr) -> Result<String> {
        let slot = format!("__return{}", self.state.temp_counter);
        self.state.temp_counter += 1;
        if self.is_text(expression) {
            match self.lower_text(expression, Some(&slot))? {
                Text::Const(text) => {
                    let hex = Hex::from(text.as_str()).to_string();
                    emit::assign(&mut self.state.output, &slot, &Type::String, "s", &hex);
                }
                Text::Slot(existing) if existing != slot => return Ok(existing),
                Text::Slot(_) => {}
            }
            self.state
                .values
                .insert(slot.clone(), Variable::temporary(Type::String));
            return Ok(slot);
        }
        let env = self.env;
        let (_, ty, native) = self.declaration_value(expression, None, &env.type_params, &slot)?;
        if !native {
            return Err(Errors::UnsupportedReturnValue {
                expression: expression.to_string(),
            }
            .into());
        }
        self.state
            .values
            .insert(slot.clone(), Variable::temporary(ty));
        Ok(slot)
    }

    /// Lower `return value;` inside a function called at run time.
    pub fn lower_function_return(&mut self, value: Option<&Expr>) -> Result<()> {
        let Some(ty) = self.state.returning.last().cloned() else {
            return Ok(());
        };
        let operand = match value {
            None if ty == Type::Void => None,
            None => {
                return Err(Errors::ReturnNeedsValue { ty: ty.name() }.into());
            }
            Some(_) if ty == Type::Void => {
                return Err(Errors::VoidReturnsValue.into());
            }
            Some(value) if is_scalar(&ty) => Some(Operand::Slot(self.scalar_slot(value, &ty)?)),
            Some(value) if matches!(ty, Type::String | Type::Path) => {
                if !self.is_text(value) {
                    return Err(Errors::ExpectedType {
                        expected: ty.name(),
                        received: value.to_string(),
                    }
                    .into());
                }
                Some(match self.lower_text(value, None)? {
                    Text::Const(text) => Operand::Str(text.into_bytes()),
                    Text::Slot(slot) => Operand::Slot(slot),
                })
            }
            Some(value) => match value.as_ident() {
                Some(name) => {
                    self.ensure_usable(name)?;
                    Some(Operand::Slot(name.to_owned()))
                }
                None => {
                    let slot = self.materialize_return(value)?;
                    Some(Operand::Slot(slot))
                }
            },
        };
        emit::function_return(
            &mut self.state.output,
            &FunctionReturn {
                ty: ty.name(),
                operand,
            },
        );
        Ok(())
    }
}
