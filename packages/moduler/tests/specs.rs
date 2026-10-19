use ast::{Item, Program};
use moduler::{ExportSpec, ImportSpec};

fn import(source: &str) -> Option<ImportSpec> {
    let program: Program = source.parse().unwrap();
    program
        .imports()
        .next()
        .and_then(|decl| ImportSpec::try_from(decl).ok())
}

fn export(source: &str) -> Option<ExportSpec> {
    let program: Program = source.parse().unwrap();
    program
        .exports()
        .next()
        .and_then(|decl| ExportSpec::try_from(decl).ok())
}

#[test]
fn parses_import_uris() {
    assert_eq!(
        import(r#"import "json://value";"#),
        Some(ImportSpec {
            alias: "json".into(),
            module: "value".into(),
        })
    );
    assert_eq!(
        import(r#"import "intolang://builtin/std";"#),
        Some(ImportSpec {
            alias: "builtin".into(),
            module: "std".into(),
        })
    );
    assert_eq!(
        import(r#"import   "fs://file";"#),
        Some(ImportSpec {
            alias: "fs".into(),
            module: "file".into(),
        })
    );
    assert!(import("init { }").is_none());
    assert!(import(r#"import "nonsense";"#).is_none());
}

#[test]
fn parses_export_uris() {
    assert_eq!(
        export(r#"export "fs://file";"#),
        Some(ExportSpec {
            alias: "fs".into(),
            path: "file".into(),
        })
    );
    assert_eq!(
        export(r#"export   "builtin://tests/assert/assert";"#),
        Some(ExportSpec {
            alias: "builtin".into(),
            path: "tests/assert/assert".into(),
        })
    );
    assert!(export(r#"import "fs://file";"#).is_none());
}

#[test]
fn display_returns_the_canonical_uri() {
    let spec: ImportSpec = "intolang://builtin/std".parse().unwrap();
    assert_eq!(spec.to_string(), "builtin://std");
    let program: Program = r#"import "a://b";"#.parse().unwrap();
    assert!(matches!(program.items[0], Item::Import(_)));
}
