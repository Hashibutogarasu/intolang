use std::path::{Path, PathBuf};

use package::{MemoryFs, Package, Vfs};

const MANIFEST: &str = r#"{
    "name": "Into Language",
    "alias": "builtin",
    "version": "0.0.1",
    "LICENCE": "MIT"
}"#;

fn packages() -> MemoryFs {
    [
        ("/virtual/packages/intolang/into-package.json", MANIFEST),
        (
            "/virtual/packages/intolang/src/main/object.into",
            "pub abstract Object() {}",
        ),
        ("/virtual/packages/stray.txt", "not a package"),
    ]
    .into_iter()
    .collect()
}

#[test]
fn files_and_directories_are_told_apart() {
    let files = packages();
    assert!(files.is_file(Path::new("/virtual/packages/intolang/src/main/object.into")));
    assert!(!files.is_file(Path::new("/virtual/packages/intolang")));
    assert!(files.is_dir(Path::new("/virtual/packages/intolang")));
    assert!(!files.is_dir(Path::new("/virtual/packages/intolang/src/main/object.into")));
    assert!(!files.is_dir(Path::new("/virtual/missing")));
}

#[test]
fn a_directory_lists_what_is_directly_inside_it() {
    let files = packages();
    assert_eq!(
        files.read_dir(Path::new("/virtual/packages")).unwrap(),
        [
            PathBuf::from("/virtual/packages/intolang"),
            PathBuf::from("/virtual/packages/stray.txt"),
        ]
    );
    assert!(files.read_dir(Path::new("/virtual/missing")).is_err());
}

#[test]
fn dots_in_a_path_are_resolved() {
    let files = packages();
    assert_eq!(
        files
            .read_to_string(Path::new(
                "/virtual/packages/./intolang/src/../into-package.json"
            ))
            .unwrap(),
        MANIFEST
    );
}

#[test]
fn a_package_is_found_in_memory() {
    let files = packages();
    let found = Package::find_by_alias_in(&files, Path::new("/virtual/packages"), "builtin")
        .unwrap()
        .expect("a package");
    assert_eq!(found.root, Path::new("/virtual/packages/intolang"));
    assert_eq!(found.manifest.name, "Into Language");
    let enclosing = Package::enclosing_in(
        &files,
        Path::new("/virtual/packages/intolang/src/main/object.into"),
    )
    .expect("a package");
    assert_eq!(enclosing.root, found.root);
}
