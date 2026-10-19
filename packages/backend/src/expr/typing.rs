use ast::{Expr, Path};
use std::collections::HashSet;
use types::{Literal, Type};

use super::ownership::{BORROW, COPY, MOVE};
use super::type_name::{base_type_name, parse_type};
use crate::context::{EVENT, Lowerer, THREAD};

fn literal_type(literal: &Literal) -> Type {
    match literal {
        Literal::Int(_) => Type::Int32,
        Literal::Float(_) => Type::Float64,
        Literal::String(_) => Type::String,
        Literal::Bool(_) => Type::Boolean,
        Literal::Char(_) => Type::Char,
    }
}

fn spelled(text: &str) -> Type {
    parse_type(text, &HashSet::new()).unwrap_or_else(|_| Type::Named(text.to_owned()))
}

impl Lowerer<'_> {
    /// The type an expression has, without emitting anything.
    pub fn type_of(&self, expression: &Expr) -> Result<Type, String> {
        match expression {
            Expr::Literal(literal) => Ok(literal_type(literal)),
            Expr::Path(path) => self.path_type(path),
            Expr::Call { callee, args } => self.call_type(callee, args),
            Expr::Binary { left, op, right } => {
                let (left, right) = (self.type_of(left)?, self.type_of(right)?);
                if *op == types::ArithmeticOperator::Add
                    && (left == Type::String || right == Type::String)
                    && matches!(left, Type::String | Type::Char)
                    && matches!(right, Type::String | Type::Char)
                {
                    return Ok(Type::String);
                }
                match (left.is_numeric(), right.is_numeric()) {
                    (true, true) if left.is_float() || !right.is_float() => Ok(left),
                    (true, true) => Ok(right),
                    _ => Err(format!(
                        "Arithmetic needs numbers but received `{}` and `{}`",
                        left.name(),
                        right.name()
                    )),
                }
            }
            Expr::Cast { ty, .. } => Ok(spelled(ty.as_str())),
            Expr::Array(items) => self.array_type(items),
            Expr::Compare { .. } | Expr::Logical { .. } | Expr::Not(_) => Ok(Type::Boolean),
            Expr::Thread { .. } => Ok(Type::Named(THREAD.to_owned())),
            Expr::Lambda { .. } => Err("A lambda has no type here".to_owned()),
            Expr::Await(_) => Err("`await` has no value".to_owned()),
        }
    }

    fn path_type(&self, path: &Path) -> Result<Type, String> {
        if let Some(field) = self.static_field(path) {
            return Ok(spelled(&field.ty));
        }
        let first = path.first();
        if let Some(variable) = self.state.values.get(first) {
            let mut current = if variable.native {
                variable.ty.clone()
            } else {
                Type::String
            };
            for segment in &path.segments()[1..] {
                let class = base_type_name(&current.name()).to_owned();
                let field = self
                    .env
                    .class_fields
                    .get(&class)
                    .and_then(|fields| fields.iter().find(|f| !f.is_static && f.name == *segment))
                    .ok_or_else(|| format!("`{class}` has no field `{segment}`"))?;
                let written = self.written_type(&field.ty);
                current = self.bind_arguments(&current, written);
            }
            return Ok(current);
        }
        if path
            .as_ident()
            .is_some_and(|name| self.env.events.contains_key(name))
        {
            return Ok(Type::Named(EVENT.to_owned()));
        }
        Err(format!("Unknown name `{path}`"))
    }

    fn call_type(&self, callee: &Path, args: &[Expr]) -> Result<Type, String> {
        let env = self.env;
        if !callee.is_member() {
            let name = callee.first();
            if env.class_fields.contains_key(base_type_name(name)) {
                return Ok(Type::Named(name.to_owned()));
            }
            return self.function_type(&self.pick_overload(name, args)?);
        }
        if let Some((class, method)) = self.static_receiver(callee) {
            let key = self.method_key(&class, &method);
            return self.function_type(&self.pick_overload(&key, args)?);
        }
        let segments = callee.segments();
        let mut current = self
            .state
            .values
            .get(&segments[0])
            .map(|variable| variable.ty.clone())
            .ok_or_else(|| format!("Unknown receiver `{}`", segments[0]))?;
        for segment in &segments[1..] {
            let class = base_type_name(&current.name()).to_owned();
            if matches!(segment.as_str(), COPY | MOVE | BORROW) {
                continue;
            }
            if let Type::List(element) = current.referent()
                && segment == "get"
            {
                current = (**element).clone();
                continue;
            }
            let key = self.method_key(&class, segment);
            let result = self.function_type(&self.pick_overload(&key, args)?)?;
            current = self.bind_arguments(&current, result);
        }
        Ok(current)
    }

    /// A type as written in a declaration, where the names of type parameters
    /// stand for generic parameters.
    fn written_type(&self, text: &str) -> Type {
        parse_type(text, &self.env.type_params).unwrap_or_else(|_| Type::Named(text.to_owned()))
    }

    /// Replace the type parameters of the class `receiver` is an instance of
    /// by the type arguments it was given.
    pub(crate) fn bind_arguments(&self, receiver: &Type, ty: Type) -> Type {
        let arguments = receiver.arguments();
        let Some(class) = receiver
            .class_name()
            .and_then(|name| self.env.classes.get(name))
        else {
            return ty;
        };
        if arguments.is_empty() || class.type_params.len() != arguments.len() {
            return ty;
        }
        let bindings = class
            .type_params
            .iter()
            .cloned()
            .zip(arguments.iter().cloned())
            .collect();
        ty.substitute(&bindings)
    }

    fn function_type(&self, key: &str) -> Result<Type, String> {
        let env = self.env;
        if let Some(function) = env.user_functions.get(key) {
            return Ok(function
                .return_type
                .as_deref()
                .map_or(Type::Void, |written| self.written_type(written)));
        }
        if let Some(link) = env.native_links.get(key) {
            return Ok(self.written_type(&link.return_type));
        }
        Err(format!("Undefined function `{key}`"))
    }
}
