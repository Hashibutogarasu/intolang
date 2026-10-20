use console::style;

use crate::label::thread_label;
use crate::style::now;

/// The mark of a line whose work is done.
pub(crate) const DONE: &str = "✓";

/// The mark of a line whose work failed.
pub(crate) const FAILED: &str = "✗";

/// The mark of a line whose work goes on.
pub(crate) const PENDING: &str = " ";

pub(crate) fn line(mark: &str, message: &str) {
    let mark = if mark == DONE {
        style(mark).green()
    } else {
        style(mark).red()
    };
    eprintln!(
        "[{mark}] [{}] [{}] {}",
        style(thread_label()).green(),
        style(now()).white(),
        style(message).white()
    );
}

/// Print one failed line, `[✗] [thread/main] [HH:MM SS.cc] message`, to stderr.
pub fn fail(message: &str) {
    line(FAILED, message);
}

/// Print one finished line, `[✓] [thread/main] [HH:MM SS.cc] message`, to
/// stderr, so the output of the program a command runs stays on stdout.
pub fn report(message: &str) {
    line(DONE, message);
}
