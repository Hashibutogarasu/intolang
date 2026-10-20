use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use package::{Manifest, Package};

fn directory() -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let base = std::env::temp_dir().join(format!(
        "package-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();
    base
}

fn write_package(base: &std::path::Path, directory: &str, manifest: &str) -> PathBuf {
    let root = base.join(directory);
    fs::create_dir_all(root.join("src/main")).unwrap();
    fs::write(root.join("into-package.json"), manifest).unwrap();
    root
}

const MANIFEST: &str = r#"{
    "name": "Bad Apple",
    "alias": "bad-apple",
    "version": "1.2.3",
    "LICENCE": "MIT",
    "runs": "src/main/main.into",
    "builtConfig": { "dist": "./out" },
    "packages": { "ps": "0.1.0" },
    "nativeDependencies": { "src/ffi/a.c": ["zlib"] }
}"#;

#[test]
fn a_manifest_becomes_a_structure() {
    let manifest: Manifest = MANIFEST.parse().unwrap();
    assert_eq!(manifest.name, "Bad Apple");
    assert_eq!(manifest.required_alias().unwrap(), "bad-apple");
    assert_eq!(manifest.licence, "MIT");
    assert_eq!(manifest.built_config.dist, "./out");
    assert_eq!(manifest.packages["ps"], "0.1.0");
    assert_eq!(manifest.dependencies_of("src/ffi/a.c"), ["zlib"]);
    assert!(manifest.dependencies_of("src/ffi/b.c").is_empty());
    assert_eq!(manifest.executable_name(), "bad-apple");
}

#[test]
fn what_a_manifest_leaves_out_has_a_default() {
    let manifest: Manifest = r#"{"name":"Bad Apple"}"#.parse().unwrap();
    assert_eq!(manifest.built_config.dist, "./dist");
    assert_eq!(manifest.executable_name(), "Bad-Apple");
    assert_eq!(manifest.required_alias().unwrap_err().code(), Some(0x800A));
}

#[test]
fn a_package_is_found_by_its_alias_whatever_its_directory_is_called() {
    let base = directory();
    write_package(&base, "intolang", r#"{"name":"Into","alias":"builtin"}"#);
    write_package(&base, "other", r#"{"name":"Other","alias":"other"}"#);
    fs::create_dir_all(base.join("empty")).unwrap();
    let found = Package::find_by_alias(&base, "builtin").unwrap().unwrap();
    assert_eq!(found.root, base.join("intolang"));
    assert_eq!(found.manifest.name, "Into");
    assert!(Package::find_by_alias(&base, "missing").unwrap().is_none());
    assert_eq!(Package::children(&base).unwrap().len(), 2);
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn the_package_that_holds_a_file_is_the_closest_one() {
    let base = directory();
    let root = write_package(&base, "app", MANIFEST);
    let source = root.join("src/main/main.into");
    fs::write(&source, "").unwrap();
    let package = Package::enclosing(&source).unwrap();
    assert_eq!(package.root, root);
    assert_eq!(package.join("src"), root.join("src"));
    assert!(Package::enclosing(&base).is_none());
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn a_manifest_that_cannot_be_read_names_its_file() {
    let base = directory();
    let missing = Manifest::load(&base.join("into-package.json")).unwrap_err();
    assert_eq!(missing.code(), Some(0xB301));
    fs::write(base.join("into-package.json"), "{").unwrap();
    let invalid = Manifest::load(&base.join("into-package.json")).unwrap_err();
    assert_eq!(invalid.code(), Some(0x8009));
    fs::remove_dir_all(base).unwrap();
}
