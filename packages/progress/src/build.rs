use console::Term;
use indicatif::{ProgressBar, ProgressDrawTarget};
use std::time::Duration;

use crate::report::report;
use crate::style::{finished, running};
use crate::{PENDING, line};

const TICK: Duration = Duration::from_millis(100);

/// The progress of one piece of work with a known number of steps. On a
/// terminal it draws a spinner and a bar; elsewhere it prints one line per
/// step in the same format.
pub struct Progress {
    bar: Option<ProgressBar>,
}

impl Progress {
    pub fn start(message: &str, steps: u64) -> Self {
        if !Term::stderr().is_term() {
            line(PENDING, message);
            return Self { bar: None };
        }
        let bar = ProgressBar::with_draw_target(Some(steps), ProgressDrawTarget::stderr());
        bar.set_style(running());
        bar.set_message(message.to_owned());
        bar.enable_steady_tick(TICK);
        Self { bar: Some(bar) }
    }

    pub fn step(&self, message: &str) {
        match &self.bar {
            Some(bar) => {
                bar.inc(1);
                bar.set_message(message.to_owned());
            }
            None => line(PENDING, message),
        }
    }

    pub fn finish(self, message: &str) {
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
        if let Some(bar) = &self.bar
            && !bar.is_finished()
        {
            bar.abandon();
        }
    }
}
