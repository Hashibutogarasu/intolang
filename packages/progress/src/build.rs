use console::Term;
use indicatif::{ProgressBar, ProgressDrawTarget};
use std::cell::{Cell, RefCell};
use std::time::Duration;

use crate::report::report;
use crate::style::{failed, finished, running};
use crate::{FAILED, PENDING, line};

const TICK: Duration = Duration::from_millis(100);

/// The progress of one piece of work with a known number of steps. On a
/// terminal it draws a spinner and a bar; elsewhere it prints one line per
/// step in the same format. Work that is dropped before `finish` has failed:
/// its line keeps the last message and a red cross replaces the spinner.
pub struct Progress {
    bar: Option<ProgressBar>,
    message: RefCell<String>,
    done: Cell<bool>,
}

impl Progress {
    pub fn start(message: &str, steps: u64) -> Self {
        let message_cell = RefCell::new(message.to_owned());
        if !Term::stderr().is_term() {
            line(PENDING, message);
            return Self {
                bar: None,
                message: message_cell,
                done: Cell::new(false),
            };
        }
        let bar = ProgressBar::with_draw_target(Some(steps), ProgressDrawTarget::stderr());
        bar.set_style(running());
        bar.set_message(message.to_owned());
        bar.enable_steady_tick(TICK);
        Self {
            bar: Some(bar),
            message: message_cell,
            done: Cell::new(false),
        }
    }

    /// Change the number of steps once the work knows how many there are.
    pub fn steps(&self, steps: u64) {
        if let Some(bar) = &self.bar {
            bar.set_length(steps);
        }
    }

    pub fn step(&self, message: &str) {
        *self.message.borrow_mut() = message.to_owned();
        match &self.bar {
            Some(bar) => {
                bar.inc(1);
                bar.set_message(message.to_owned());
            }
            None => line(PENDING, message),
        }
    }

    pub fn finish(self, message: &str) {
        self.done.set(true);
        match &self.bar {
            Some(bar) => {
                bar.set_style(finished());
                bar.finish_with_message(message.to_owned());
            }
            None => report(message),
        }
    }
}

impl Drop for Progress {
    fn drop(&mut self) {
        if self.done.get() {
            return;
        }
        let message = self.message.borrow().clone();
        match &self.bar {
            Some(bar) => {
                bar.set_style(failed());
                bar.abandon_with_message(message);
            }
            None => line(FAILED, &message),
        }
    }
}
