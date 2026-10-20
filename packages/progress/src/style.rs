use chrono::{Local, Timelike};
use console::style;
use indicatif::{ProgressState, ProgressStyle};
use std::fmt::Write;

use crate::clock::{clock, elapsed, remaining};
use crate::label::thread_label;

const SPINNER: [&str; 5] = ["/", "-", "\\", "|", crate::report::DONE];

const FAILURE: [&str; 2] = [crate::report::FAILED, crate::report::FAILED];

pub(crate) fn now() -> String {
    let time = Local::now();
    clock(
        time.hour(),
        time.minute(),
        time.second(),
        time.nanosecond() / 10_000_000,
    )
}

fn white(text: impl std::fmt::Display) -> String {
    style(text).white().to_string()
}

fn current_time(_: &ProgressState, out: &mut dyn Write) {
    write!(out, "{}", white(now())).ok();
}

fn elapsed_time(state: &ProgressState, out: &mut dyn Write) {
    write!(out, "{}", white(elapsed(state.elapsed()))).ok();
}

fn remaining_time(state: &ProgressState, out: &mut dyn Write) {
    write!(out, "{}", white(remaining(state.eta()))).ok();
}

fn keyed(template: &str, ticks: &[&str]) -> ProgressStyle {
    let label = thread_label();
    ProgressStyle::with_template(template)
        .expect("the progress templates are fixed")
        .tick_strings(ticks)
        .progress_chars("=> ")
        .with_key("thread", move |_: &ProgressState, out: &mut dyn Write| {
            write!(out, "{}", style(&label).green()).ok();
        })
        .with_key("now", current_time)
        .with_key("elapsed_cs", elapsed_time)
        .with_key("eta_short", remaining_time)
}

/// The style of a running build: spinner, thread, time, message, bar, elapsed
/// time and the estimated time left. The thread shown is the one that makes
/// the style, not the one that draws it.
pub(crate) fn running() -> ProgressStyle {
    keyed(
        "[{spinner:.red}] [{thread}] [{now}] {msg:.white} {bar:20.blue} [{elapsed_cs}] [ETA {eta_short}]",
        &SPINNER,
    )
}

/// The style of a line that is left behind when the work is done, with the
/// check mark of the spinner in green.
pub(crate) fn finished() -> ProgressStyle {
    keyed(
        "[{spinner:.green}] [{thread}] [{now}] {msg:.white}",
        &SPINNER,
    )
}

/// The style of a line that is left behind when the work failed, with a red
/// cross where the spinner was.
pub(crate) fn failed() -> ProgressStyle {
    keyed("[{spinner:.red}] [{thread}] [{now}] {msg:.white}", &FAILURE)
}
