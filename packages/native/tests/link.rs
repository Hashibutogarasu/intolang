use ast::{FunctionDecl, Program};
use native::NativeLink;
use types::Language;

fn function(source: &str) -> FunctionDecl {
    let program: Program = source.parse().unwrap();
    program.functions().next().unwrap().clone()
}

#[test]
fn reads_link_metadata() {
    let function = function(
        "#[link(\"cpp\", \"src/native/a.cpp\", \"sym\")]\npub cpp fun f(a: Int32, b: String?): Boolean;",
    );
    let link = NativeLink::from_function(&function, "pkg")
        .unwrap()
        .unwrap();
    assert_eq!(link.language, Language::Cpp);
    assert_eq!(link.package_alias, "pkg");
    assert_eq!(link.source_path, "src/native/a.cpp");
    assert_eq!(link.target, "sym");
    assert_eq!(link.return_type, "Boolean");
    assert_eq!(link.params.len(), 2);
}

#[test]
fn returns_nothing_without_a_link_attribute() {
    let function = function("pub fun f(): Void;");
    assert!(
        NativeLink::from_function(&function, "pkg")
            .unwrap()
            .is_none()
    );
}

#[test]
fn defaults_the_return_type_to_void() {
    let function = function("#[link(\"c\", \"a.c\", \"sym\")]\npub c fun f();");
    let link = NativeLink::from_function(&function, "pkg")
        .unwrap()
        .unwrap();
    assert_eq!(link.return_type, "Void");
}

#[test]
fn rejects_invalid_link_attributes() {
    for (source, expected) in [
        (
            "#[link(\"c\", \"a.c\")]\nfun f();",
            "expects language, source path, and target",
        ),
        (
            "#[link(\"rust\", \"a.c\", \"sym\")]\nfun f();",
            "Unsupported link language `rust`",
        ),
        (
            "#[link(\"c\", \"/abs/a.c\", \"sym\")]\nfun f();",
            "must remain inside",
        ),
        (
            "#[link(\"c\", \"../a.c\", \"sym\")]\nfun f();",
            "must remain inside",
        ),
        ("#[link(\"c\", \"a.c\", \"\")]\nfun f();", "cannot be empty"),
        (
            "#[link(\"c\", 1, \"sym\")]\nfun f();",
            "expects language, source path, and target",
        ),
    ] {
        let error = NativeLink::from_function(&function(source), "pkg").unwrap_err();
        assert!(error.to_string().contains(expected), "{source}: {error}");
    }
}
