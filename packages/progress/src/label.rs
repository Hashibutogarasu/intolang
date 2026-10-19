use std::path::Path;

/// The thread the caller runs on as `thread/<name>`, or `thread/<number>` for
/// a thread without a name.
pub fn thread_label() -> String {
    let current = std::thread::current();
    match current.name() {
        Some(name) => format!("thread/{name}"),
        None => {
            let identifier = format!("{:?}", current.id());
            let number = identifier
                .trim_start_matches("ThreadId(")
                .trim_end_matches(')');
            format!("thread/{number}")
        }
    }
}

/// The name of the operating system this tool runs on, capitalized.
pub fn platform_name() -> String {
    let mut name = std::env::consts::OS.to_owned();
    if let Some(first) = name.get_mut(..1) {
        first.make_ascii_uppercase();
    }
    name
}

/// The message for a finished build, such as
/// `Linux x86_64 example /path/to/file was built.`.
pub fn built(name: &str, output: &Path) -> String {
    format!(
        "{} {} {name} {} was built.",
        platform_name(),
        std::env::consts::ARCH,
        output.display()
    )
}
