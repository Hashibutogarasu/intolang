use ast::{Expr, Param, Path, Stmt, StmtKind};
use native::OwnershipChange;
use std::collections::HashSet;
use types::{Literal, MessageTag, Storage, Type};

use super::condition::fold_comparison;
use super::events::RuntimeCall;
use super::ownership::{BORROW, COPY, DESTROY, MOVE};
use super::type_name::{base_type_name, parse_type};
use crate::context::{InlineFrame, Lowerer, Tracking, UserFunction, Variable};
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
        if self.is_runtime_function(function) {
            return self.call_runtime_function(function, receiver, args);
        }
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
        let scope_base = self.state.scopes.len();
        self.state.scopes.push(Vec::new());
        self.state.frames.push(InlineFrame {
            scope_base,
            returns_reference: function.returns_reference(),
        });
        let mut views = Vec::new();
        let mut returned = None;
        let lowered = self.inline_body(
            function,
            arguments,
            substitutions,
            depth,
            &mut views,
            &mut returned,
        );
        self.state.frames.pop();
        for (name, tracking) in views {
            if let Some(variable) = self.state.values.get_mut(&name) {
                variable.tracking = tracking;
            }
        }
        let names = self.state.scopes.pop().unwrap_or_default();
        lowered?;
        let keep = returned.as_ref().map(|(name, _)| name.clone());
        self.release_scope(&names, keep.as_deref())?;
        Ok(returned)
    }

    /// Lower the body of `function` with its arguments substituted. A binding
    /// passed to a reference parameter is a reference until the call ends; its
    /// earlier tracking is pushed to `views` to be restored then.
    fn inline_body(
        &mut self,
        function: &UserFunction,
        arguments: Vec<Expr>,
        mut substitutions: Vec<(String, Expr)>,
        depth: usize,
        views: &mut Vec<(String, Tracking)>,
        returned: &mut Returned,
    ) -> Result<(), String> {
        for (param, argument) in function.params.iter().zip(arguments) {
            let argument = if function.inspects {
                argument
            } else {
                self.prepare_argument(param, argument, views)?
            };
            substitutions.push((param.name.clone(), argument));
        }
        function
            .body
            .iter()
            .map(|stmt| stmt.substitute(&substitutions))
            .try_for_each(|stmt| self.lower_inline_statement(&stmt, depth, returned))
    }

    /// Turn `x.copy()`, `x.move()` and `x.borrow()` arguments into bindings of
    /// the call, and apply the reference rules to a binding argument.
    fn prepare_argument(
        &mut self,
        param: &Param,
        argument: Expr,
        views: &mut Vec<(String, Tracking)>,
    ) -> Result<Expr, String> {
        let argument = self.materialize_argument(argument)?;
        let reference_param = param
            .ty
            .as_ref()
            .is_some_and(|ty| ty.as_str().ends_with('&'));
        let value_param = param.ty.is_some() && !reference_param;
        let Some(name) = argument.as_ident().map(str::to_owned) else {
            return Ok(argument);
        };
        let Some(variable) = self.state.values.get(&name) else {
            return Ok(argument);
        };
        if !variable.native
            || !self.is_ownership_method(base_type_name(&variable.ty.name()), DESTROY)
        {
            return Ok(argument);
        }
        self.ensure_usable(&name)?;
        let variable = &self.state.values[&name];
        if value_param && variable.ty.is_reference() {
            return Err(format!(
                "`{name}` is a reference; pass `{name}.copy()` where `{}` expects an owned value",
                param.name
            ));
        }
        if reference_param && !variable.tracking.reference {
            views.push((name.clone(), variable.tracking.clone()));
            if let Some(variable) = self.state.values.get_mut(&name) {
                variable.tracking.reference = true;
            }
        }
        Ok(argument)
    }

    /// Lower an ownership call used as an argument into a temporary binding
    /// that the call's scope owns.
    pub fn materialize_argument(&mut self, argument: Expr) -> Result<Expr, String> {
        let Expr::Call { callee, args } = &argument else {
            return Ok(argument);
        };
        let [receiver, method] = callee.segments() else {
            return Ok(argument);
        };
        if !args.is_empty() || !matches!(method.as_str(), COPY | MOVE | BORROW) {
            return Ok(argument);
        }
        let Some(source) = self.state.values.get(receiver) else {
            return Ok(argument);
        };
        let receiver_type = source.ty.clone();
        let temporary = format!("__arg{}", self.state.temp_counter);
        self.state.temp_counter += 1;
        let ty = self.lower_ownership_value(receiver, &receiver_type, method, &temporary)?;
        let tracking = self.state.pending_tracking.take().unwrap_or_default();
        let mut binding = Variable::temporary(ty);
        binding.tracking = tracking;
        self.state.values.insert(temporary.clone(), binding);
        self.declare_in_scope(&temporary);
        Ok(Expr::ident(temporary))
    }

    /// Lower the statements of an inlined body as one scope. What the scope
    /// returns is handed to the caller instead of being destroyed.
    fn inline_block(
        &mut self,
        body: &[Stmt],
        depth: usize,
        returned: &mut Returned,
    ) -> Result<(), String> {
        self.state.scopes.push(Vec::new());
        let lowered = body
            .iter()
            .try_for_each(|stmt| self.lower_inline_statement(stmt, depth, returned));
        let names = self.state.scopes.pop().unwrap_or_default();
        lowered?;
        let keep = returned.as_ref().map(|(name, _)| name.clone());
        self.release_scope(&names, keep.as_deref())
    }

    fn lower_inline_statement(
        &mut self,
        stmt: &Stmt,
        depth: usize,
        returned: &mut Returned,
    ) -> Result<(), String> {
        match &stmt.kind {
            StmtKind::Break | StmtKind::Return(None) => Ok(()),
            StmtKind::Throw(message) => {
                self.lower_throw(message);
                Ok(())
            }
            StmtKind::Return(Some(expression)) => {
                let name = match expression.as_ident() {
                    Some(name) if self.state.values.contains_key(name) => name.to_owned(),
                    _ => self.materialize_return(expression)?,
                };
                let variable = self
                    .state
                    .values
                    .get(&name)
                    .ok_or_else(|| format!("Unsupported return value `{name}`"))?;
                let ty = variable.ty.clone();
                if let Some(frame) = self.state.frames.last()
                    && frame.returns_reference
                    && self.state.scopes[frame.scope_base..]
                        .iter()
                        .any(|names| names.contains(&name))
                {
                    return Err(format!("returns a reference to a local (`{name}`)"));
                }
                *returned = Some((name, ty));
                Ok(())
            }
            StmtKind::Declaration(declaration) => self.inline_declaration(declaration),
            StmtKind::While { .. } | StmtKind::Assign { .. } => {
                self.lower_stmt(stmt).map_err(|error| error.to_string())
            }
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
        self.state.pending_tracking = None;
        let (text, ty, native) =
            self.declaration_value(value, annotation, &none, &declaration.name)?;
        let mut tracking = self.state.pending_tracking.take().unwrap_or_default();
        tracking.reference |= ty.is_reference();
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
                tracking,
            },
        );
        self.emit_change(OwnershipChange::Declare {
            name: declaration.name.clone(),
            storage: declaration.storage,
        });
        self.declare_in_scope(&declaration.name);
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
        self.materialize_assigned_texts(then_body);
        self.materialize_assigned_texts(else_body.unwrap_or_default());
        let folded = fold_comparison(condition);
        match condition {
            _ if folded.is_some() => {
                emit::if_const(&mut self.state.output, folded.unwrap_or_default())
            }
            _ if self.begin_state_check(condition) => {}
            Expr::Compare { left, op, right } if self.is_plain_comparison(left, right) => {
                self.begin_comparison(MessageTag::IfBegin, "if", left, *op, right)?;
            }
            Expr::Literal(Literal::Bool(flag)) => emit::if_const(&mut self.state.output, *flag),
            Expr::Literal(Literal::Int(number @ (0 | 1))) => {
                emit::if_const(&mut self.state.output, *number == 1)
            }
            other => {
                self.begin_boolean_branch(MessageTag::IfBegin, other)
                    .map_err(|message| {
                        format!("Unsupported if condition in function body: {message}")
                    })?;
            }
        }
        self.inline_block(then_body, depth, returned)?;
        if let Some(else_body) = else_body {
            emit::bare(&mut self.state.output, MessageTag::IfElse);
            self.inline_block(else_body, depth, returned)?;
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
        if matches!(argument, Expr::Path(path) if path.is_member())
            || matches!(
                argument,
                Expr::Call { .. }
                    | Expr::Binary { .. }
                    | Expr::Cast { .. }
                    | Expr::Compare { .. }
                    | Expr::Logical { .. }
                    | Expr::Not(_)
            )
        {
            return self
                .lower_print(0, newline, args)
                .map_err(|error| error.to_string());
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

    pub fn inline_call(
        &mut self,
        callee: &Path,
        args: &[Expr],
        depth: usize,
    ) -> Result<(), String> {
        let env = self.env;
        if callee.is_member() {
            if !matches!(
                self.lower_event_call(callee, args, "")?,
                RuntimeCall::NotRuntime
            ) {
                return Ok(());
            }
            if let Some((class, method)) = self.static_receiver(callee) {
                let key = self.method_key(&class, &method);
                let key = self.pick_overload(&key, args)?;
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
            let class = base_type_name(&receiver_type.name()).to_owned();
            let method = callee.last();
            if self
                .lower_list_method(&receiver, &receiver_type, method, args, "")?
                .is_some()
            {
                return Ok(());
            }
            self.check_receiver_call(&receiver, method)?;
            if self.give_back_reference(&receiver, method) {
                return Ok(());
            }
            let key = self.method_key(&class, method);
            let key = self.pick_overload(&key, args)?;
            if let Some(inner) = env.user_functions.get(&key) {
                self.inline_user_function(inner, Some(receiver.clone()), args, depth + 1)?;
                self.apply_receiver_call(&receiver, method);
                return Ok(());
            }
            let Some(link) = env.native_links.get(&key) else {
                if self.is_ownership_method(&class, method) {
                    self.apply_receiver_call(&receiver, method);
                    return Ok(());
                }
                return Err(format!("Undefined method `{key}`"));
            };
            let mut combined = vec![Expr::ident(receiver.clone())];
            combined.extend_from_slice(args);
            self.emit_link_call(MessageTag::LinkAssign, link, combined, "", true)?;
            self.apply_receiver_call(&receiver, method);
            return Ok(());
        }
        let name = callee.first();
        if self.lower_indirect_call(name, args)? {
            return Ok(());
        }
        let key = self.pick_overload(name, args)?;
        if let Some(inner) = env.user_functions.get(&key) {
            return self
                .inline_user_function(inner, None, args, depth + 1)
                .map(|_| ());
        }
        let link = env
            .native_links
            .get(&key)
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
