use progress::{built, clock, elapsed, platform_name, remaining, thread_label};
use std::path::Path;
use std::time::Duration;

#[test]
fn the_clock_has_hours_minutes_seconds_and_hundredths() {
    assert_eq!(clock(23, 0, 45, 0), "23:00 45.00");
    assert_eq!(clock(5, 9, 7, 65), "05:09 07.65");
}

#[test]
fn elapsed_time_has_minutes_seconds_and_hundredths() {
    assert_eq!(elapsed(Duration::from_millis(11_050)), "00:11 05");
    assert_eq!(elapsed(Duration::from_millis(61_999)), "01:01 99");
    assert_eq!(elapsed(Duration::ZERO), "00:00 00");
}

#[test]
fn remaining_time_has_minutes_and_seconds() {
    assert_eq!(remaining(Duration::from_secs(3)), "00:03");
    assert_eq!(remaining(Duration::from_secs(754)), "12:34");
}

#[test]
fn the_main_thread_is_named_and_others_are_numbered() {
    std::thread::Builder::new()
        .name("main".into())
        .spawn(|| assert_eq!(thread_label(), "thread/main"))
        .unwrap()
        .join()
        .unwrap();
    std::thread::spawn(|| {
        let label = thread_label();
        let number = label.strip_prefix("thread/").unwrap();
        assert!(number.parse::<u64>().is_ok(), "{label}");
    })
    .join()
    .unwrap();
}

#[test]
fn a_finished_build_names_the_platform_the_architecture_and_the_file() {
    let message = built("example", Path::new("/path/to/file"));
    assert_eq!(
        message,
        format!(
            "{} {} example /path/to/file was built.",
            platform_name(),
            std::env::consts::ARCH
        )
    );
    if cfg!(target_os = "linux") {
        assert!(message.starts_with("Linux x86_64 example") || message.starts_with("Linux "));
    }
}
