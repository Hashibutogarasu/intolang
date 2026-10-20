use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use ast::Program;
use moduler::{SourceProvider, collect_modules_from, import_path, unresolved_imports};

struct Buffers(HashMap<PathBuf, String>);

impl SourceProvider for Buffers {
    fn read(&self, path: &Path) -> Option<String> {
        self.0.get(path).cloned()
    }
}

fn environment() -> (PathBuf, PathBuf) {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let base = std::env::temp_dir().join(format!(
        "moduler-sources-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&base);
    let package = base.join("util");
    fs::create_dir_all(package.join("src/main")).unwrap();
    fs::write(
        package.join("into-package.json"),
        r#"{"name":"util","alias":"util","version":"0.1.0","licence":"MIT","library":"src/main/lib.into"}"#,
    )
    .unwrap();
    fs::write(package.join("src/main/tools.into"), "pub fun on_disk() {}").unwrap();
    (base, package.join("src/main/tools.into"))
}

#[test]
fn an_open_buffer_wins_over_the_file() {
    let (base, module) = environment();
    let program: Program = r#"import "util://tools";"#.parse().unwrap();
    let buffers = Buffers(HashMap::from([(
        module.clone(),
        "pub fun in_buffer() {}".to_owned(),
    )]));
    let modules = collect_modules_from(&program, &base, &buffers).unwrap();
    assert_eq!(modules.len(), 1);
    assert!(modules[0].contents.contains("in_buffer"));
    assert_eq!(modules[0].module.source_path, module);
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn a_module_a_package_exports_without_a_file_resolves() {
    let (base, _) = environment();
    let info = base.join("util/src/main/into-info.into");
    fs::write(info, "export \"util://std/print\";\n").unwrap();
    let program: Program = "import \"util://std\";\nimport \"util://stdout\";"
        .parse()
        .unwrap();
    let unresolved = unresolved_imports(&program, &base);
    assert_eq!(unresolved.len(), 1);
    assert_eq!(unresolved[0].0.uri, "util://stdout");
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn unresolved_imports_name_the_missing_package_or_module() {
    let (base, module) = environment();
    let program: Program = r#"
        import "util://tools";
        import "util://missing";
        import "nowhere://tools";
        import "not a uri";
    "#
    .parse()
    .unwrap();
    let unresolved = unresolved_imports(&program, &base);
    let uris: Vec<&str> = unresolved
        .iter()
        .map(|(decl, _)| decl.uri.as_str())
        .collect();
    assert_eq!(uris, ["util://missing", "nowhere://tools", "not a uri"]);
    assert_eq!(unresolved[0].1.code(), Some(0x10AA));
    let first = program.imports().next().unwrap();
    assert_eq!(import_path(first, &base).unwrap(), Some(module));
    fs::remove_dir_all(base).unwrap();
}
