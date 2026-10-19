use ast::Program;
use moduler::resolve_links;
use std::path::PathBuf;

fn temp_package_root(name: &str) -> PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir().join(format!(
        "intolang-moduler-{name}-{}-{unique}",
        std::process::id()
    ))
}

fn write_package(root: &PathBuf, module: &str) {
    let package = root.join("files");
    let main = package.join("src/main");
    std::fs::create_dir_all(&main).unwrap();
    std::fs::write(
        package.join("into-package.json"),
        "{\"name\":\"Files\",\"alias\":\"files\",\"version\":\"1.0.0\",\"LICENCE\":\"MIT\"}",
    )
    .unwrap();
    std::fs::write(main.join("into-info.into"), "export \"files://reader\";\n").unwrap();
    std::fs::write(main.join("reader.into"), module).unwrap();
}

const ENTRY: &str = "import \"files://reader\";\ninit {\n}\n";

#[test]
fn links_that_differ_in_parameter_types_get_their_own_keys() {
    let root = temp_package_root("overloads");
    write_package(
        &root,
        "#[link(\"c\", \"src/ffi/read.c\", \"read_text\")]\npub fun read(path: String): String;\n#[link(\"c\", \"src/ffi/read.c\", \"read_path\")]\npub fun read(path: Path): String;\n#[link(\"c\", \"src/ffi/read.c\", \"single\")]\npub fun once(path: String): String;\n",
    );
    let entry: Program = ENTRY.parse().unwrap();
    let links = resolve_links(&entry, &root).unwrap();
    assert!(!links.contains_key("read"));
    assert_eq!(links["read$String"].target, "read_text");
    assert_eq!(links["read$Path"].target, "read_path");
    assert_eq!(links["once"].target, "single");
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn methods_that_differ_in_parameter_types_get_their_own_keys() {
    let root = temp_package_root("method-overloads");
    write_package(
        &root,
        "pub File() {\n    #[link(\"c\", \"src/ffi/file.c\", \"file_read_text\")]\n    pub fun read(path: String): String;\n    #[link(\"c\", \"src/ffi/file.c\", \"file_read_path\")]\n    pub fun read(path: Path): String;\n    #[link(\"c\", \"src/ffi/file.c\", \"file_close\")]\n    pub fun close(): Void;\n}\n",
    );
    let entry: Program = ENTRY.parse().unwrap();
    let links = resolve_links(&entry, &root).unwrap();
    assert_eq!(links["File.read$String"].target, "file_read_text");
    assert_eq!(links["File.read$Path"].target, "file_read_path");
    assert_eq!(links["File.close"].target, "file_close");
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn the_parameters_of_an_untyped_function_are_written_with_underscores() {
    let key = ast::overload_key("log", &[]);
    assert_eq!(key, "log$");
}
