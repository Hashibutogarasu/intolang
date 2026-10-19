use ast::Program;
use moduler::{collect_modules, public_items, resolve, resolve_links};
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

fn write_greet_package(root: &PathBuf, module: &str) {
    let package = root.join("greet");
    let main = package.join("src/main");
    std::fs::create_dir_all(&main).unwrap();
    std::fs::write(
        package.join("into-package.json"),
        r#"{"name":"Greet","alias":"greet","version":"1.0.0","LICENCE":"MIT"}"#,
    )
    .unwrap();
    std::fs::write(
        main.join("into-info.into"),
        r#"export "greet://hello";
"#,
    )
    .unwrap();
    std::fs::write(main.join("hello.into"), module).unwrap();
}

fn entry() -> Program {
    r#"import "greet://hello";
init {
    hello();
}
"#
    .parse()
    .unwrap()
}

#[test]
fn resolves_modules_and_hides_private_items() {
    let root = temp_package_root("resolve");
    write_greet_package(
        &root,
        r#"pub fun hello(): Void;
fun secret(): Void;
"#,
    );

    let entry = entry();
    let modules = collect_modules(&entry, &root).unwrap();
    assert_eq!(modules.len(), 1);
    let module = &modules[0].module;
    assert_eq!(module.alias, "greet");
    assert_eq!(module.name, "hello");
    assert_eq!(module.package.name, "Greet");
    assert_eq!(module.package.version, "1.0.0");
    assert_eq!(module.package.licence, "MIT");
    assert!(module.source_path.ends_with("src/main/hello.into"));

    let public = resolve(&entry, &root).unwrap();
    assert!(public.contains("hello"), "{public:?}");
    assert!(!public.contains("secret"), "{public:?}");

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn resolves_link_metadata_for_public_declarations_only() {
    let root = temp_package_root("links");
    write_greet_package(
        &root,
        r#"#[link("c", "src/ffi/hello.c", "greet_hello")]
pub c fun hello(name: String?): Void;
#[link("c", "src/ffi/hello.c", "hidden")]
c fun secret(): Void;
"#,
    );

    let entry = entry();
    let links = resolve_links(&entry, &root).unwrap();
    assert!(!links.contains_key("secret"));
    let link = &links["hello"];
    assert_eq!(link.target, "greet_hello");
    assert_eq!(link.package_alias, "greet");
    assert_eq!(link.return_type, "Void");
    assert!(link.params[0].optional);

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn rejects_link_sources_outside_the_package() {
    let root = temp_package_root("escape");
    write_greet_package(
        &root,
        r#"#[link("c", "../outside.c", "greet_hello")]
pub c fun hello(): Void;
"#,
    );

    let entry = entry();
    let error = resolve_links(&entry, &root).unwrap_err();
    assert!(error.to_string().contains("must remain inside"), "{error}");

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn collects_only_public_declarations() {
    let program: Program = r#"pub fun visible() {}
fun hidden() {}
pub Thing() {
    pub fun new(): Thing;
    fun secret(): Thing;
}
pub macro test {
    #[test]
}
"#
    .parse()
    .unwrap();
    let items = public_items(&program);
    assert!(items.contains("visible"), "{items:?}");
    assert!(!items.contains("hidden"), "{items:?}");
    assert!(items.contains("Thing"), "{items:?}");
    assert!(items.contains("Thing.new"), "{items:?}");
    assert!(items.contains("test"), "{items:?}");
    assert!(!items.contains("Thing.secret"), "{items:?}");
    assert!(!items.contains("secret"), "{items:?}");
}
