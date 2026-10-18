use ast::{Expr, Path, Stmt, StmtKind};
use std::collections::HashSet;
use types::{Literal, MessageTag, Storage, Type};

use super::type_name::{base_type_name, parse_type};
use crate::context::{Lowerer, UserFunction, Variable};
use crate::emit;

/// How deep inlined calls may nest before the call is rejected as recursive.
const MAX_INLINE_DEPTH: usize = 32;

type Returned = Option<(String, Type)>;

impl<'a> Lowerer<'a> {
    /// Lower a call to a user function by substituting its arguments into a
    /// copy of its body. Returns the variable the body returns, if any.
    pub fn inline_user_function(
        &mut self,
        function: &UserFunction,
        receiver: Option<String>,
        args: &[Expr],
        depth: usize,
    ) -> Result<Returned, String> {
        if depth > MAX_INLINE_DEPTH {
            return Err(format!(
                "Recursive call to `{}` is not supported",
                function.name
            ));
        }
        let mut arguments = args.to_vec();
        if arguments.len() > function.params.len() {
            return Err(format!(
                "Function `{}` expects {} argument(s) but received {}",
                function.name,
                function.params.len(),
                arguments.len()
            ));
        }
        for param in &function.params[arguments.len()..] {
            match param.default_argument() {
                Some(default) => arguments.push(default),
                None => return Err(format!("Missing required argument `{}`", param.name)),
            }
        }
        let mut substitutions: Vec<(String, Expr)> = Vec::new();
        match (function.receiver.as_ref(), receiver) {
            _ if function.is_static => {}
            (Some(_), Some(receiver)) => {
                substitutions.push(("this".to_owned(), Expr::ident(receiver)))
            }
            (Some(_), None) => {
                return Err(format!("Method `{}` requires a receiver", function.name));
            }
            (None, Some(_)) => {
                return Err(format!("Function `{}` is not a method", function.name));
            }
            (None, None) => {}
        }
        for (param, argument) in function.params.iter().zip(arguments) {
            substitutions.push((param.name.clone(), argument));
        }
        let mut returned = None;
        for stmt in &function.body {
            let stmt = stmt.substitute(&substitutions);
            self.lower_inline_statement(&stmt, depth, &mut returned)?;
        }
        Ok(returned)
    }

    fn lower_inline_statement(
        &mut self,
        stmt: &Stmt,
        depth: usize,
        returned: &mut Returned,
    ) -> Result<(), String> {
        match &stmt.kind {
            StmtKind::Break | StmtKind::Return(None) => Ok(()),
            StmtKind::Return(Some(expression)) => {
                let name = expression.to_string();
                let variable = self
                    .state
                    .values
                    .get(&name)
                    .ok_or_else(|| format!("Unsupported return value `{name}`"))?;
                *returned = Some((name, variable.ty.clone()));
                Ok(())
            }
            StmtKind::Declaration(declaration) => self.inline_declaration(declaration),
            StmtKind::If {
                condition,
                then_body,
                else_body,
            } => self.inline_if(condition, then_body, else_body.as_deref(), depth, returned),
            StmtKind::Expr(Expr::Call { callee, args }) => {
                match (callee.as_ident(), args.as_slice()) {
                    (Some(name @ ("print" | "println")), _) => {
                        self.inline_print(name == "println", args)
                    }
                    _ => self.inline_call(callee, args, depth),
                }
            }
            other => Err(format!(
                "Unsupported statement in function body: `{}`",
                describe(other)
            )),
        }
    }

    fn inline_declaration(&mut self, declaration: &ast::Declaration) -> Result<(), String> {
        let Some(value) = &declaration.value else {
            return Ok(());
        };
        let none = HashSet::new();
        let annotation = declaration
            .annotation
            .as_ref()
            .map(|ty| parse_type(ty.as_str(), &none))
            .transpose()
            .map_err(|error| error.to_string())?;
        let annotation = match (declaration.pointer, annotation) {
            (true, None) => Some(Type::Pointer),
            (_, annotation) => annotation,
        };
        let (text, ty, native) =
            self.declaration_value(value, annotation, &none, &declaration.name)?;
        self.state.values.insert(
            declaration.name.clone(),
            Variable {
                value: text,
                ty,
                memory_backed: declaration.storage != Storage::Local,
                allocated: declaration.storage == Storage::Now,
                lazy: declaration.storage == Storage::Late,
                unloaded: false,
                declared_in_init: false,
                native,
            },
        );
        Ok(())
    }

    fn inline_if(
        &mut self,
        condition: &Expr,
        then_body: &[Stmt],
        else_body: Option<&[Stmt]>,
        depth: usize,
        returned: &mut Returned,
    ) -> Result<(), String> {
        match condition {
            Expr::Compare { left, op, right } => {
                self.begin_comparison(MessageTag::IfBegin, "if", left, *op, right)?;
            }
            Expr::Literal(Literal::Bool(flag)) => emit::if_const(&mut self.state.output, *flag),
            Expr::Literal(Literal::Int(number @ (0 | 1))) => {
                emit::if_const(&mut self.state.output, *number == 1)
            }
            other => {
                return Err(format!(
                    "Unsupported if condition in function body: `{other}`"
                ));
            }
        }
        for stmt in then_body {
            self.lower_inline_statement(stmt, depth, returned)?;
        }
        if let Some(else_body) = else_body {
            emit::bare(&mut self.state.output, MessageTag::IfElse);
            for stmt in else_body {
                self.lower_inline_statement(stmt, depth, returned)?;
            }
        }
        emit::bare(&mut self.state.output, MessageTag::IfEnd);
        Ok(())
    }

    fn inline_print(&mut self, newline: bool, args: &[Expr]) -> Result<(), String> {
        let [argument] = args else {
            let text = args.iter().map(ToString::to_string).collect::<Vec<_>>();
            return Err(format!("Unsupported print argument `{}`", text.join(", ")));
        };
        if self.print_compile_time(argument, newline) {
            return Ok(());
        }
        if let Some(variable) = argument
            .as_ident()
            .and_then(|name| self.state.values.get(name).map(|v| (name, v.clone())))
        {
            let (name, variable) = variable;
            if variable.native {
                emit::print(&mut self.state.output, name, &variable.ty, newline);
            } else {
                emit::text(&mut self.state.output, &variable.value, newline);
            }
            return Ok(());
        }
        if let Expr::Literal(literal) = argument {
            self.print_literal(literal, newline);
            return Ok(());
        }
        Err(format!("Unsupported print argument `{argument}`"))
    }

    fn inline_call(&mut self, callee: &Path, args: &[Expr], depth: usize) -> Result<(), String> {
        let env = self.env;
        if callee.is_member() {
            if let Some((class, method)) = self.static_receiver(callee) {
                let key = format!("{class}.{method}");
                if let Some(inner) = env.user_functions.get(&key) {
                    return self
                        .inline_user_function(inner, None, args, depth + 1)
                        .map(|_| ());
                }
                let link = env
                    .native_links
                    .get(&key)
                    .ok_or_else(|| format!("Undefined static method `{key}`"))?;
                return self.emit_link_call(MessageTag::LinkAssign, link, args.to_vec(), "", false);
            }
            let (receiver, receiver_type) = self.resolve_receiver(callee)?;
            let key = format!(
                "{}.{}",
                base_type_name(&receiver_type.name()),
                callee.last()
            );
            if let Some(inner) = env.user_functions.get(&key) {
                return self
                    .inline_user_function(inner, Some(receiver), args, depth + 1)
                    .map(|_| ());
            }
            let link = env
                .native_links
                .get(&key)
                .ok_or_else(|| format!("Undefined method `{key}`"))?;
            let mut combined = vec![Expr::ident(receiver)];
            combined.extend_from_slice(args);
            return self.emit_link_call(MessageTag::LinkAssign, link, combined, "", true);
        }
        let name = callee.first();
        if let Some(inner) = env.user_functions.get(name) {
            return self
                .inline_user_function(inner, None, args, depth + 1)
                .map(|_| ());
        }
        let link = env
            .native_links
            .get(name)
            .ok_or_else(|| format!("Undefined function `{name}`"))?;
        self.emit_link_call(MessageTag::LinkAssign, link, args.to_vec(), "", false)
    }
}

fn describe(kind: &StmtKind) -> String {
    match kind {
        StmtKind::Declaration(declaration) => format!("into {}", declaration.name),
        StmtKind::Assign { target, value } => format!("{target} = {value}"),
        StmtKind::Unload(name) => format!("unload {name}"),
        StmtKind::If { condition, .. } => format!("if {condition}"),
        StmtKind::While { condition, .. } => format!("while {condition}"),
        StmtKind::Break => "break".to_owned(),
        StmtKind::Return(Some(value)) => format!("return {value}"),
        StmtKind::Return(None) => "return".to_owned(),
        StmtKind::Throw(value) => format!("throw {value}"),
        StmtKind::Init(_) => "init".to_owned(),
        StmtKind::Expr(expression) => expression.to_string(),
    }
}
