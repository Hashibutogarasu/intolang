use anyhow::{Result, anyhow};
use native::NativeArg;
use std::collections::HashMap;

use super::frame::Frame;
use super::vm::Vm;
use crate::value::Value;

/// How many handlers one event keeps; registrations beyond it are ignored.
const EVENT_CAPACITY: usize = 64;

#[derive(Debug, Clone)]
pub struct EventTable {
    next: i64,
    handlers: Vec<(i64, String)>,
}

impl Default for EventTable {
    fn default() -> Self {
        Self {
            next: 1,
            handlers: Vec::new(),
        }
    }
}

impl Vm {
    pub(super) fn event_on(&mut self, event: &str, listener: &str, handler: &str) -> Result<()> {
        if !self.program.functions.contains_key(handler) {
            return Err(anyhow!("Unknown event handler `{handler}`"));
        }
        if !listener.is_empty() {
            self.store(listener, Value::Int(0));
        }
        let table = self.events.entry(event.to_owned()).or_default();
        if table.handlers.len() >= EVENT_CAPACITY {
            return Ok(());
        }
        let id = table.next;
        table.next += 1;
        table.handlers.push((id, handler.to_owned()));
        if !listener.is_empty() {
            self.store(listener, Value::Int(id));
        }
        Ok(())
    }

    pub(super) fn event_off(&mut self, event: &str, listener: &str) -> Result<()> {
        let id = self.load_slot(listener)?.int().unwrap_or(0);
        if let Some(table) = self.events.get_mut(event) {
            table.handlers.retain(|(registered, _)| *registered != id);
        }
        Ok(())
    }

    pub(super) fn event_destroy(&mut self, event: &str) {
        if let Some(table) = self.events.get_mut(event) {
            table.handlers.clear();
        }
    }

    /// Queue a call of every handler registered now; the first registered
    /// runs first and handlers registered while it runs wait for the next
    /// emit. The calling frame has to be past the emit already.
    pub(super) fn emit(&mut self, event: &str, args: &[NativeArg]) -> Result<()> {
        let values = args
            .iter()
            .map(|arg| self.argument(arg))
            .collect::<Result<Vec<_>>>()?;
        let handlers: Vec<String> = self
            .events
            .get(event)
            .map(|table| {
                table
                    .handlers
                    .iter()
                    .map(|(_, name)| name.clone())
                    .collect()
            })
            .unwrap_or_default();
        let program = self.program.clone();
        for name in handlers.iter().rev() {
            let function = &program.functions[name];
            let locals: HashMap<String, Value> = function
                .params
                .iter()
                .cloned()
                .zip(values.iter().cloned())
                .collect();
            self.frames.push(Frame::function(
                function.entry,
                locals,
                function.captures.clone(),
            ));
        }
        Ok(())
    }
}
