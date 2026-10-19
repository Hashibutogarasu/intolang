use anyhow::{Result, anyhow, bail};
use std::collections::HashMap;
use std::mem;

use super::frame::Frame;
use super::vm::Vm;
use crate::value::Value;

/// A virtual thread. Only one runs at a time and the running one keeps its
/// frames in the VM; the others keep them here.
#[derive(Debug, Default)]
pub struct Thread {
    pub frames: Vec<Frame>,
    pub waiting: Option<usize>,
    pub done: bool,
}

impl Vm {
    pub(super) fn thread_run(&mut self, task: &str, function: &str) -> Result<()> {
        let function = self
            .program
            .functions
            .get(function)
            .ok_or_else(|| anyhow!("Unknown thread function `{function}`"))?
            .clone();
        let id = self.threads.len();
        self.threads.push(Thread {
            frames: vec![Frame::function(
                function.entry,
                HashMap::new(),
                function.captures,
            )],
            ..Thread::default()
        });
        if !task.is_empty() {
            self.store(task, Value::Int(id as i64));
        }
        Ok(())
    }

    /// Whether the thread the task names has finished; when it has not, the
    /// current thread waits for it and another one runs.
    pub(super) fn thread_await(&mut self, task: &str) -> Result<bool> {
        let id = self.load_slot(task)?.int().unwrap_or(-1);
        let target = self
            .threads
            .get(id as usize)
            .ok_or_else(|| anyhow!("`{task}` holds no thread"))?;
        if target.done {
            return Ok(true);
        }
        self.threads[self.current].waiting = Some(id as usize);
        self.switch_thread()?;
        Ok(false)
    }

    /// The thread after the current one, in round-robin order, that can run.
    fn next_runnable(&self) -> Option<usize> {
        let count = self.threads.len();
        (1..=count)
            .map(|offset| (self.current + offset) % count)
            .find(|&index| {
                let thread = &self.threads[index];
                !thread.done
                    && thread
                        .waiting
                        .is_none_or(|target| self.threads[target].done)
            })
    }

    pub(super) fn switch_thread(&mut self) -> Result<()> {
        let Some(next) = self.next_runnable() else {
            bail!("Every thread is waiting: deadlock");
        };
        if next != self.current {
            self.threads[self.current].frames = mem::take(&mut self.frames);
            self.current = next;
            self.frames = mem::take(&mut self.threads[next].frames);
        }
        self.threads[next].waiting = None;
        Ok(())
    }

    /// The current thread has no frame left.
    pub(super) fn thread_finished(&mut self) -> Result<bool> {
        self.threads[self.current].done = true;
        if self.current == 0 {
            return Ok(true);
        }
        self.switch_thread()?;
        Ok(false)
    }
}
