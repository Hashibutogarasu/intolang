use std::time::Duration;

/// A time of day as `HH:MM SS.cc`.
pub fn clock(hour: u32, minute: u32, second: u32, centiseconds: u32) -> String {
    format!("{hour:02}:{minute:02} {second:02}.{centiseconds:02}")
}

/// A span as `MM:SS cc`, minutes, seconds and hundredths of a second.
pub fn elapsed(span: Duration) -> String {
    let seconds = span.as_secs();
    format!(
        "{:02}:{:02} {:02}",
        seconds / 60,
        seconds % 60,
        span.subsec_millis() / 10
    )
}

/// A span as `MM:SS`.
pub fn remaining(span: Duration) -> String {
    let seconds = span.as_secs();
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}
