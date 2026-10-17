//! Module resolution for the Into language.
//!
//! This crate owns everything concerned with `import`/`export` statements:
//! locating packages by alias, following imports transitively, collecting the
//! public names a module exposes, and resolving `#[link]` metadata. A resolved
//! module is represented by [`Module`], which pairs the [`PackageInfo`] it came
//! from with the path of its source file.

mod items;
mod library;
mod module;
mod package;
mod resolve;
mod text;

pub use items::{ClassSpan, public_items, scan_classes};
pub use library::{NativeFunction, lower_library};
pub use module::{ExportSpec, ImportSpec, Module, parse_export, parse_import};
pub use package::{PackageInfo, find_by_alias};
pub use resolve::{
    LoadedModule, NativeLink, Param, collect_modules, imported_module_sources, module_exports,
    parse_params, resolve, resolve_links,
};
pub use text::{brace_delta, is_identifier, strip_keyword, strip_line_comment};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_import_uris() {
        assert_eq!(
            parse_import("import \"json://value\";"),
            Some(ImportSpec {
                alias: "json".into(),
                module: "value".into(),
            })
        );
        assert_eq!(
            parse_import("import \"intolang://builtin/std\";"),
            Some(ImportSpec {
                alias: "builtin".into(),
                module: "std".into(),
            })
        );
        assert_eq!(
            parse_import("import  \"fs://file\";"),
            Some(ImportSpec {
                alias: "fs".into(),
                module: "file".into(),
            })
        );
        assert!(parse_import("init {").is_none());
        assert!(parse_import("import \"nonsense\";").is_none());
    }

    #[test]
    fn parses_export_uris() {
        assert_eq!(
            parse_export("export \"fs://file\";"),
            Some(ExportSpec {
                alias: "fs".into(),
                path: "file".into(),
            })
        );
        assert_eq!(
            parse_export("export  \"builtin://tests/assert/assert\";"),
            Some(ExportSpec {
                alias: "builtin".into(),
                path: "tests/assert/assert".into(),
            })
        );
        assert!(parse_export("import \"fs://file\";").is_none());
    }

    #[test]
    fn resolves_modules_and_hides_private_items() {
        let root = temp_package_root("resolve");
        let package = root.join("greet");
        let main = package.join("src/main");
        std::fs::create_dir_all(&main).unwrap();
        std::fs::write(
            package.join("into-package.json"),
            "{\"name\":\"Greet\",\"alias\":\"greet\",\"version\":\"1.0.0\",\"LICENCE\":\"MIT\"}",
        )
        .unwrap();
        std::fs::write(main.join("into-info.into"), "export \"greet://hello\";\n").unwrap();
        std::fs::write(
            main.join("hello.into"),
            "pub fun hello(): Void;\nfun secret(): Void;\n",
        )
        .unwrap();

        let source = "import \"greet://hello\";\ninit {\n    hello();\n}\n";
        let modules = collect_modules(source, &root).unwrap();
        assert_eq!(modules.len(), 1);
        let module = &modules[0].module;
        assert_eq!(module.alias, "greet");
        assert_eq!(module.name, "hello");
        assert_eq!(module.package.name, "Greet");
        assert_eq!(module.package.version, "1.0.0");
        assert_eq!(module.package.licence, "MIT");
        assert!(module.source_path.ends_with("src/main/hello.into"));

        let public = resolve(source, &root).unwrap();
        assert!(public.contains("hello"), "{public:?}");
        assert!(!public.contains("secret"), "{public:?}");

        std::fs::remove_dir_all(&root).ok();
    }

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
}
