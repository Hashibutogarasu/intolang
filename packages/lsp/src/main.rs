fn main() {
    if let Err(error) = lsp::run_stdio() {
        eprintln!("into-lsp: {error}");
        std::process::exit(1);
    }
}
