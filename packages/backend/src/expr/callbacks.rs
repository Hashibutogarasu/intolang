use ast::{Expr, Param, Stmt, TypeRef};
use native::{FunctionRef, IndirectCall};
use types::Type;

use super::captures::mentioned_names;
use super::type_name::parse_type;
use crate::context::Lowerer;
use crate::emit;

/// The classes whose instances are functions: their type arguments are the
/// types of the parameters of the function.
const CALLBACK_CLASSES: [&str; 2] = ["Callback", "BiCallback"];

/// Whether a written type names a class of functions.
pub fn names_callback(written: &str) -> bool {
    let base = written.split('<').next().unwrap_or_default().trim();
    CALLBACK_CLASSES.contains(&base)
}

impl Lowerer<'_> {
    /// Whether the type is a class of functions, `Callback<String>`.
    pub fn is_function_type(ty: &Type) -> bool {
        ty.referent()
            .class_name()
            .is_some_and(|name| CALLBACK_CLASSES.contains(&name))
    }

    /// Lower a lambda to a function and store a reference to it in `target`,
    /// or in a new slot. `expected` gives the types of the parameters.
    pub fn lower_lambda_value(
        &mut self,
        params: &[Param],
        body: &[Stmt],
        expected: &Type,
        target: Option<&str>,
    ) -> Result<String, String> {
        let expected = expected.referent();
        let types = expected.arguments();
        if params.len() != types.len() {
            return Err(format!(
                "`{}` takes {} parameter(s) but the lambda declares {}",
                expected.name(),
                types.len(),
                params.len()
            ));
        }
        let mut declared = Vec::new();
        for (param, ty) in params.iter().zip(types) {
            let written = match &param.ty {
                Some(written) => {
                    let own = parse_type(written.as_str(), &self.env.type_params)
                        .map_err(|error| error.to_string())?;
                    if !ty.is_assignable_to(&own) {
                        return Err(format!(
                            "The parameter `{}` is a `{}` but `{}` passes a `{}`",
                            param.name,
                            own.name(),
                            expected.name(),
                            ty.name()
                        ));
                    }
                    written.clone()
                }
                None => TypeRef::from(ty.name().as_str()),
            };
            declared.push(Param {
                name: param.name.clone(),
                ty: Some(written),
                optional: false,
                variadic: false,
                default: None,
            });
        }
        let locals: Vec<String> = params.iter().map(|param| param.name.clone()).collect();
        if !self.state.returning.is_empty() {
            let borrowed: Vec<String> = mentioned_names(body)
                .into_iter()
                .filter(|mentioned| {
                    !locals.contains(mentioned)
                        && self
                            .state
                            .values
                            .get(mentioned)
                            .is_some_and(|variable| variable.native)
                })
                .collect();
            if !borrowed.is_empty() {
                return Err(format!(
                    "A lambda inside a function cannot use the names of that function: {}",
                    borrowed
                        .iter()
                        .map(|name| format!("`{name}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }
        let name = format!("__lambda_{}", self.state.temp_counter);
        self.state.temp_counter += 1;
        self.lower_handler(&name, &declared, &locals, body)?;
        let slot = match target {
            Some(target) => target.to_owned(),
            None => self.fresh_slot(expected),
        };
        emit::function_ref(
            &mut self.state.output,
            &FunctionRef {
                target: slot.clone(),
                name,
            },
        );
        Ok(slot)
    }

    /// Lower `name(args)` when `name` holds a function. Returns whether it
    /// did.
    pub fn lower_indirect_call(&mut self, name: &str, args: &[Expr]) -> Result<bool, String> {
        let Some(variable) = self.state.values.get(name) else {
            return Ok(false);
        };
        if !variable.native || !Self::is_function_type(&variable.ty) {
            return Ok(false);
        }
        let function_type = variable.ty.clone();
        self.ensure_usable(name)?;
        let types = function_type.referent().arguments().to_vec();
        if args.len() != types.len() {
            return Err(format!(
                "`{name}` takes {} argument(s) but received {}",
                types.len(),
                args.len()
            ));
        }
        let mut native = Vec::new();
        for (argument, ty) in args.iter().zip(&types) {
            native.push(self.runtime_argument(argument, ty)?);
        }
        emit::call_indirect(
            &mut self.state.output,
            &IndirectCall {
                slot: name.to_owned(),
                args: native,
            },
        );
        Ok(true)
    }
}
