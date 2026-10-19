use ast::{Expr, Param, Path, Stmt};
use std::mem;
use types::Type;

use super::captures::mentioned_names;
use super::ownership::DESTROY;
use super::type_name::parse_type;
use crate::collect::ancestors;
use crate::context::{EVENT, LISTENER, Lowerer, Tracking, Variable};
use crate::emit;

const ON: &str = "on";
const EMIT: &str = "emit";

/// What a method call on an event, listener or thread turned out to be.
pub enum RuntimeCall {
    NotRuntime,
    Done,
    Listener(String),
    Task,
}

impl Lowerer<'_> {
    /// Lower `event.on(lambda)`, `event.emit(args)`, `event.destroy()` and
    /// `listener.destroy()`. The registration number of `on` goes to the slot
    /// `assigned_to`.
    pub fn lower_event_call(
        &mut self,
        callee: &Path,
        args: &[Expr],
        assigned_to: &str,
    ) -> Result<RuntimeCall, String> {
        if callee.segments().len() > 2 {
            return self.lower_signal_call(callee, args, assigned_to);
        }
        let [receiver, method] = callee.segments() else {
            return Ok(RuntimeCall::NotRuntime);
        };
        if let Some(event) = self.listener_event(receiver) {
            if method != DESTROY {
                return Ok(RuntimeCall::NotRuntime);
            }
            self.destroy_listener(receiver, &event)?;
            return Ok(RuntimeCall::Done);
        }
        let Some(event) = self.event_of(receiver) else {
            return match self.lower_thread_call(callee, assigned_to)? {
                RuntimeCall::NotRuntime => self.lower_signal_call(callee, args, assigned_to),
                handled => Ok(handled),
            };
        };
        match method.as_str() {
            ON => {
                self.ensure_event_alive(receiver, &event)?;
                self.register_handler(&event, args, assigned_to)?;
                Ok(RuntimeCall::Listener(event))
            }
            EMIT => {
                self.ensure_event_alive(receiver, &event)?;
                self.emit_event(&event, args)?;
                Ok(RuntimeCall::Done)
            }
            DESTROY => {
                self.ensure_event_alive(receiver, &event)?;
                emit::event_destroy(&mut self.state.output, &event);
                self.state.destroyed_events.insert(event);
                if self.state.values.contains_key(receiver) {
                    self.apply_receiver_call(receiver, DESTROY);
                }
                Ok(RuntimeCall::Done)
            }
            _ => Ok(RuntimeCall::NotRuntime),
        }
    }

    /// Register the default handlers the bodies of declared events form.
    pub fn register_default_handlers(&mut self) -> Result<(), String> {
        let env = self.env;
        let mut names: Vec<&String> = env.events.keys().collect();
        names.sort();
        for name in names {
            let info = &env.events[name];
            if info.body.is_empty() {
                continue;
            }
            let handler = format!("__default_{name}");
            let locals: Vec<String> = info.params.iter().map(|param| param.name.clone()).collect();
            self.lower_handler(&handler, &info.params, &locals, &info.body)?;
            emit::event_on(&mut self.state.output, name, "", &handler);
        }
        Ok(())
    }

    /// The event a binding or declared name stands for.
    pub fn event_of(&self, name: &str) -> Option<String> {
        match self.state.values.get(name) {
            Some(variable) => (variable.ty.name() == EVENT).then(|| variable.value.clone()),
            None => self.env.events.contains_key(name).then(|| name.to_owned()),
        }
    }

    fn listener_event(&self, name: &str) -> Option<String> {
        self.state
            .values
            .get(name)
            .filter(|variable| variable.ty.name() == LISTENER)
            .map(|variable| variable.value.clone())
    }

    fn ensure_event_alive(&self, receiver: &str, event: &str) -> Result<(), String> {
        self.ensure_usable(receiver)?;
        if self.state.destroyed_events.contains(event) {
            return Err(format!("`{receiver}` was destroyed"));
        }
        Ok(())
    }

    fn destroy_listener(&mut self, receiver: &str, event: &str) -> Result<(), String> {
        self.check_receiver_call(receiver, DESTROY)?;
        emit::event_off(&mut self.state.output, event, receiver);
        self.apply_receiver_call(receiver, DESTROY);
        Ok(())
    }

    pub fn register_handler(
        &mut self,
        event: &str,
        args: &[Expr],
        assigned_to: &str,
    ) -> Result<(), String> {
        let [Expr::Lambda { params, body }] = args else {
            return Err(format!("`{event}.on` expects one lambda handler"));
        };
        let info = &self.env.events[event];
        if params.len() != info.params.len() {
            return Err(format!(
                "The handler of `{event}` takes {} parameter(s) but the lambda declares {}",
                info.params.len(),
                params.len()
            ));
        }
        let declared = info.params.clone();
        let locals: Vec<String> = params.iter().map(|param| param.name.clone()).collect();
        let handler = format!("__handler_{event}_{}", self.state.temp_counter);
        self.state.temp_counter += 1;
        self.lower_handler(&handler, &declared, &locals, body)?;
        emit::event_on(&mut self.state.output, event, assigned_to, &handler);
        Ok(())
    }

    /// Lower `body` as the function `name`. `declared` gives the types of the
    /// parameters, which the body calls `locals`.
    pub fn lower_handler(
        &mut self,
        name: &str,
        declared: &[Param],
        locals: &[String],
        body: &[Stmt],
    ) -> Result<(), String> {
        let env = self.env;
        let mut params = Vec::new();
        for (param, local) in declared.iter().zip(locals) {
            let written = param
                .ty
                .as_ref()
                .ok_or_else(|| format!("The event parameter `{}` needs a type", param.name))?;
            let ty = parse_type(written.as_str(), &env.type_params).map_err(|e| e.to_string())?;
            params.push((local.clone(), ty));
        }
        let captures: Vec<String> = mentioned_names(body)
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
        emit::handler_begin(&mut self.state.output, name, &captures, &params);

        let shadowed: Vec<(String, Option<Variable>)> = params
            .iter()
            .map(|(local, _)| (local.clone(), self.state.values.remove(local)))
            .collect();
        for (local, ty) in &params {
            let mut variable = Variable::temporary(ty.clone());
            variable.memory_backed = true;
            variable.tracking = Tracking {
                reference: ty.is_reference(),
                ..Tracking::default()
            };
            self.state.values.insert(local.clone(), variable);
        }
        let loop_depth = mem::take(&mut self.state.loop_depth);
        let loop_scopes = mem::take(&mut self.state.loop_scopes);
        let frames = mem::take(&mut self.state.frames);
        self.state.scopes.push(Vec::new());

        self.state.returning.push(Type::Void);
        let lowered = self.lower_block(body).map_err(|error| error.to_string());
        self.state.returning.pop();
        let owned = self.state.scopes.pop().unwrap_or_default();
        lowered?;
        self.release_scope(&owned, None)?;

        self.state.loop_depth = loop_depth;
        self.state.loop_scopes = loop_scopes;
        self.state.frames = frames;
        for (local, previous) in shadowed {
            self.state.values.remove(&local);
            if let Some(previous) = previous {
                self.state.values.insert(local, previous);
            }
        }
        emit::handler_end(&mut self.state.output);
        Ok(())
    }

    fn emit_event(&mut self, event: &str, args: &[Expr]) -> Result<(), String> {
        let declared = self.env.events[event].params.clone();
        if args.len() != declared.len() {
            return Err(format!(
                "`{event}` takes {} argument(s) but received {}",
                declared.len(),
                args.len()
            ));
        }
        let mut native = Vec::new();
        for (param, arg) in declared.iter().zip(args) {
            self.check_event_argument(event, param, arg)?;
            native.push(self.native_arg(arg)?);
        }
        emit::event_emit(&mut self.state.output, event, &native);
        Ok(())
    }

    /// Whether `actual` is a class that has `expected` among its ancestors, so
    /// that an instance of it can be used where `expected` is written.
    fn inherits_from(&self, actual: &Type, expected: &Type) -> bool {
        let (Some(actual), Some(expected)) = (actual.class_name(), expected.class_name()) else {
            return false;
        };
        ancestors(&self.env.classes, actual)
            .is_ok_and(|chain| chain.iter().any(|ancestor| ancestor.name == *expected))
    }

    fn check_event_argument(&self, event: &str, param: &Param, arg: &Expr) -> Result<(), String> {
        let written = param
            .ty
            .as_ref()
            .ok_or_else(|| format!("The event parameter `{}` needs a type", param.name))?;
        let expected =
            parse_type(written.as_str(), &self.env.type_params).map_err(|e| e.to_string())?;
        let expected = expected.referent();
        let actual = self
            .type_of(arg)
            .map_err(|_| format!("Unknown event argument `{arg}`"))?
            .referent()
            .clone();
        if &actual != expected
            && !(actual == Type::String && *expected == Type::Path)
            && !self.inherits_from(&actual, expected)
        {
            return Err(format!(
                "`{event}` expects `{}` for `{}` but received `{}`",
                expected.name(),
                param.name,
                actual.name()
            ));
        }
        Ok(())
    }
}

/// The value of a declared event name: the `Event` that stands for it.
pub fn event_value(event: &str) -> (String, Type, bool) {
    (event.to_owned(), Type::Named(EVENT.to_owned()), false)
}

/// The value `event.on(handler)` binds: a `Listener` of the event.
pub fn listener_value(event: &str) -> (String, Type, bool) {
    (event.to_owned(), Type::Named(LISTENER.to_owned()), true)
}
