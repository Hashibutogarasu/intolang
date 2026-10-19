use bundle::{Bundle, BundlePath, Entry};
use std::path::PathBuf;

fn temp_directory(name: &str) -> PathBuf {
    let directory =
        std::env::temp_dir().join(format!("intolang-bundle-{name}-{}", std::process::id()));
    std::fs::remove_dir_all(&directory).ok();
    std::fs::create_dir_all(&directory).unwrap();
    directory
}

#[test]
fn a_path_is_relative_and_normalized() {
    assert_eq!(
        BundlePath::try_from("src/main/a.into").unwrap().to_string(),
        "src/main/a.into"
    );
    for rejected in ["", "/src", "a//b", "a/", "a/./b", "a/../b", "..", "a\\b"] {
        assert!(BundlePath::try_from(rejected).is_err(), "{rejected:?}");
    }
}

#[test]
fn a_path_knows_its_parent_and_its_name() {
    let path = BundlePath::try_from("src/main/a.into").unwrap();
    assert_eq!(path.file_name(), "a.into");
    assert_eq!(path.parent().unwrap().as_str(), "src/main");
    assert!(BundlePath::try_from("a").unwrap().parent().is_none());
    assert_eq!(
        path.components().collect::<Vec<_>>(),
        ["src", "main", "a.into"]
    );
}

#[test]
fn a_file_creates_the_directories_above_it() {
    let mut bundle = Bundle::new();
    bundle
        .add_file("src/main/a.into", "init {}", 0o644)
        .unwrap();
    assert_eq!(bundle.get("src"), Some(&Entry::Directory));
    assert_eq!(bundle.get("src/main"), Some(&Entry::Directory));
    assert_eq!(bundle.read("src/main/a.into"), Some(&b"init {}"[..]));
    assert!(matches!(
        bundle.get("src/main/a.into"),
        Some(Entry::File {
            size: 7,
            mode: 0o644,
            ..
        })
    ));
    assert_eq!(bundle.read("src"), None);
    assert_eq!(bundle.get("missing"), None);
    assert_eq!(bundle.get("../escape"), None);
}

#[test]
fn a_path_cannot_be_added_twice() {
    let mut bundle = Bundle::new();
    bundle.add_file("a", "one", 0o644).unwrap();
    assert!(bundle.add_file("a", "two", 0o644).is_err());
    bundle.add_directory("d").unwrap();
    bundle.add_directory("d").unwrap();
    assert!(bundle.add_file("d", "x", 0o644).is_err());
    assert!(bundle.add_directory("a").is_err());
    assert!(bundle.add_file("a/b", "x", 0o644).is_err());
}

#[test]
fn equal_contents_are_stored_once() {
    let mut bundle = Bundle::new();
    bundle.add_file("one", "same", 0o644).unwrap();
    bundle.add_file("two", "same", 0o644).unwrap();
    bundle.add_file("three", "other", 0o644).unwrap();
    assert_eq!(bundle.distinct_contents(), 2);
    assert_eq!(bundle.read("two"), Some(&b"same"[..]));
}

#[test]
fn entries_come_in_a_fixed_order() {
    let mut bundle = Bundle::new();
    bundle.add_file("b", "", 0o644).unwrap();
    bundle.add_file("a/z", "", 0o644).unwrap();
    bundle.add_file("a/y", "", 0o644).unwrap();
    let paths: Vec<String> = bundle.entries().map(|(path, _)| path.to_string()).collect();
    assert_eq!(paths, ["a", "a/y", "a/z", "b"]);
}

#[test]
fn a_directory_lists_what_is_directly_in_it() {
    let mut bundle = Bundle::new();
    bundle.add_file("src/main/a.into", "", 0o644).unwrap();
    bundle.add_file("src/main/b.into", "", 0o644).unwrap();
    bundle.add_file("src/ffi/x.c", "", 0o644).unwrap();
    bundle.add_file("into-package.json", "{}", 0o644).unwrap();
    let names = |path: &str| -> Vec<String> {
        bundle
            .read_dir(path)
            .iter()
            .map(|(name, _)| (*name).to_owned())
            .collect()
    };
    assert_eq!(names(""), ["into-package.json", "src"]);
    assert_eq!(names("src"), ["ffi", "main"]);
    assert_eq!(names("src/main"), ["a.into", "b.into"]);
    assert!(names("missing").is_empty());
}

#[test]
fn a_tree_is_added_with_its_empty_directories_and_in_name_order() {
    let source = temp_directory("tree");
    std::fs::create_dir_all(source.join("res/empty")).unwrap();
    std::fs::create_dir_all(source.join("src")).unwrap();
    std::fs::write(source.join("res/a.bin"), [0u8, 159, 146, 150]).unwrap();
    std::fs::write(source.join("src/b.into"), "init {}").unwrap();
    std::fs::write(source.join("日本語.txt"), "こんにちは").unwrap();
    let mut bundle = Bundle::new();
    bundle.add_tree(&source, "").unwrap();
    let paths: Vec<String> = bundle.entries().map(|(path, _)| path.to_string()).collect();
    assert_eq!(
        paths,
        [
            "res",
            "res/a.bin",
            "res/empty",
            "src",
            "src/b.into",
            "日本語.txt"
        ]
    );
    assert_eq!(bundle.read("res/a.bin"), Some(&[0u8, 159, 146, 150][..]));
    assert_eq!(bundle.get("res/empty"), Some(&Entry::Directory));
    assert_eq!(bundle.read("日本語.txt"), Some("こんにちは".as_bytes()));
    std::fs::remove_dir_all(&source).ok();
}

#[test]
fn a_tree_is_mounted_below_a_path() {
    let source = temp_directory("mount");
    std::fs::write(source.join("a.txt"), "a").unwrap();
    let mut bundle = Bundle::new();
    bundle.add_tree(&source, "res/files").unwrap();
    assert_eq!(bundle.read("res/files/a.txt"), Some(&b"a"[..]));
    assert_eq!(bundle.get("res"), Some(&Entry::Directory));
    std::fs::remove_dir_all(&source).ok();
}

#[test]
fn a_package_is_mounted_without_its_crate_outputs_and_history() {
    let package = temp_directory("package");
    for directory in [
        "src/main",
        "src/ffi",
        "src/rust",
        "dist/linux",
        "target/debug",
        ".git",
    ] {
        std::fs::create_dir_all(package.join(directory)).unwrap();
    }
    std::fs::write(package.join("into-package.json"), "{}").unwrap();
    std::fs::write(package.join("src/main/lib.into"), "").unwrap();
    std::fs::write(package.join("src/ffi/x.c"), "").unwrap();
    std::fs::write(package.join("src/rust/lib.rs"), "").unwrap();
    std::fs::write(package.join("dist/linux/out"), "").unwrap();
    std::fs::write(package.join("target/debug/out"), "").unwrap();
    std::fs::write(package.join(".git/HEAD"), "").unwrap();
    let mut bundle = Bundle::new();
    bundle.mount_package("fs", &package).unwrap();
    let paths: Vec<String> = bundle.entries().map(|(path, _)| path.to_string()).collect();
    assert_eq!(
        paths,
        [
            ".into",
            ".into/packages",
            ".into/packages/fs",
            ".into/packages/fs/into-package.json",
            ".into/packages/fs/src",
            ".into/packages/fs/src/ffi",
            ".into/packages/fs/src/ffi/x.c",
            ".into/packages/fs/src/main",
            ".into/packages/fs/src/main/lib.into",
        ]
    );
    std::fs::remove_dir_all(&package).ok();
}

#[test]
fn a_linked_directory_is_followed() {
    let target = temp_directory("link-target");
    let project = temp_directory("link-project");
    std::fs::write(target.join("a.txt"), "linked").unwrap();
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&target, project.join("shared")).unwrap();
        let mut bundle = Bundle::new();
        bundle.add_tree(&project, "").unwrap();
        assert_eq!(bundle.read("shared/a.txt"), Some(&b"linked"[..]));
    }
    std::fs::remove_dir_all(&target).ok();
    std::fs::remove_dir_all(&project).ok();
}
