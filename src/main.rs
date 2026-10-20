mod commands;

fn main() -> std::process::ExitCode {
    errors::install_panic_hook();
    commands::run()
}
