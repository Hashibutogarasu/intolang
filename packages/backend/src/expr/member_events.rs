use ast::{Expr, Path};
use types::Type;

use super::events::RuntimeCall;
use super::ownership::DESTROY;
use super::type_name::{base_type_name, parse_type};
use crate::context::{EVENT, EventInfo, Lowerer};
use crate::emit;

const ON: &str = "on";
const EMIT: &str = "emit";

impl Lowerer<'_> {
    /// What a declared event, or the event of a class member, declares.
    pub fn event_info(&self, event: &str) -> Result<EventInfo, String> {
        self.state
            .member_events
            .get(event)
            .or_else(|| self.env.events.get(event))
            .cloned()
            .ok_or_else(|| format!("Unknown event `{event}`"))
    }

    /// The event the whole path names when it reaches an event that a class
    /// declares, through the fields of a binding such as `http.on.response`.
    /// The event belongs to the binding, and is named by the path.
    pub fn member_event(&mut self, segments: &[String]) -> Result<Option<String>, String> {
        if segments.len() < 2 {
            return Ok(None);
        }
        let Some(variable) = self.state.values.get(&segments[0]) else {
            return Ok(None);
        };
        if !variable.native {
            return Ok(None);
        }
        let mut current = variable.ty.referent().clone();
        let env = self.env;
        for (position, segment) in segments[1..].iter().enumerate() {
            let class = base_type_name(&current.name()).to_owned();
            if let Some(info) = env
                .class_events
                .get(&class)
                .and_then(|events| events.get(segment))
            {
                if position + 2 != segments.len() {
                    return Ok(None);
                }
                let key = segments.join(".");
                self.reach_member_event(&key, info)?;
                return Ok(Some(key));
            }
            let Some(field) = env.class_fields.get(&class).and_then(|fields| {
                fields
                    .iter()
                    .find(|field| !field.is_static && field.name == *segment)
            }) else {
                return Ok(None);
            };
            let written =
                parse_type(&field.ty, &env.type_params).map_err(|error| error.to_string())?;
            current = self.bind_arguments(&current, written);
        }
        Ok(None)
    }

    /// Remember the event of a member the first time it is reached, and
    /// register the handler its body forms.
    fn reach_member_event(&mut self, key: &str, info: &EventInfo) -> Result<(), String> {
        if self
            .state
            .member_events
            .insert(key.to_owned(), info.clone())
            .is_some()
        {
            return Ok(());
        }
        if info.body.is_empty() {
            return Ok(());
        }
        let handler = format!("__default_{key}");
        let locals: Vec<String> = info.params.iter().map(|param| param.name.clone()).collect();
        self.lower_handler(&handler, &info.params, &locals, &info.body)?;
        emit::event_on(&mut self.state.output, key, "", &handler);
        Ok(())
    }

    /// Lower a call that reaches the event of a class member: the event
    /// called with a lambda (`http.on.response(handler)`), or its `on`, `emit`
    /// and `destroy`. Returns `None` when the path is not such an event.
    pub fn lower_member_event_call(
        &mut self,
        callee: &Path,
        args: &[Expr],
        assigned_to: &str,
    ) -> Result<Option<RuntimeCall>, String> {
        let segments = callee.segments();
        if let Some(event) = self.member_event(segments)? {
            self.ensure_member_alive(&segments[0], &event)?;
            self.register_handler(&event, args, assigned_to)?;
            return Ok(Some(RuntimeCall::Listener(event)));
        }
        let Some((method, prefix)) = segments.split_last() else {
            return Ok(None);
        };
        let Some(event) = self.member_event(prefix)? else {
            return Ok(None);
        };
        self.ensure_member_alive(&segments[0], &event)?;
        match method.as_str() {
            ON => {
                self.register_handler(&event, args, assigned_to)?;
                Ok(Some(RuntimeCall::Listener(event)))
            }
            EMIT => {
                self.emit_event(&event, args)?;
                Ok(Some(RuntimeCall::Done))
            }
            DESTROY => {
                emit::event_destroy(&mut self.state.output, &event);
                self.state.destroyed_events.insert(event);
                Ok(Some(RuntimeCall::Done))
            }
            _ => Ok(None),
        }
    }

    /// `name.call` when `name` is a binding whose class defines a `call`
    /// method, so that `name(args)` runs it.
    pub fn callable_path(&self, name: &str) -> Option<Path> {
        let variable = self
            .state
            .values
            .get(name)
            .filter(|variable| variable.native)?;
        let class = base_type_name(&variable.ty.name()).to_owned();
        let key = format!("{class}.call");
        let defined = self.env.user_functions.contains_key(&key)
            || self.env.native_links.contains_key(&key)
            || self.env.overloads.contains_key(&key);
        defined.then(|| Path::from(vec![name.to_owned(), "call".to_owned()]))
    }

    /// Register the lambda that `event(handler)` gives to a declared event.
    pub fn lower_event_handler_call(&mut self, name: &str, args: &[Expr]) -> Result<bool, String> {
        if self.state.values.contains_key(name)
            || !self.env.events.contains_key(name)
            || !matches!(args, [Expr::Lambda { .. }])
        {
            return Ok(false);
        }
        if self.state.destroyed_events.contains(name) {
            return Err(format!("`{name}` was destroyed"));
        }
        self.register_handler(name, args, "")?;
        Ok(true)
    }

    fn ensure_member_alive(&self, holder: &str, event: &str) -> Result<(), String> {
        self.ensure_usable(holder)?;
        if self.state.destroyed_events.contains(event) {
            return Err(format!("`{event}` was destroyed"));
        }
        Ok(())
    }

    /// The type of a path that ends in the event of a class member.
    pub fn member_event_type(&self, segments: &[String]) -> Option<Type> {
        let variable = self.state.values.get(segments.first()?)?;
        let mut current = variable.ty.referent().clone();
        let env = self.env;
        for (position, segment) in segments[1..].iter().enumerate() {
            let class = base_type_name(&current.name()).to_owned();
            if env
                .class_events
                .get(&class)
                .is_some_and(|events| events.contains_key(segment))
            {
                return (position + 2 == segments.len()).then(|| Type::Named(EVENT.to_owned()));
            }
            let field = env
                .class_fields
                .get(&class)?
                .iter()
                .find(|field| !field.is_static && field.name == *segment)?;
            let written = parse_type(&field.ty, &env.type_params).ok()?;
            current = self.bind_arguments(&current, written);
        }
        None
    }
}
