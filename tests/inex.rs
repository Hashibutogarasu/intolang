use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const INTO: &str = env!("CARGO_BIN_EXE_intolang");

/// A project in a temporary directory that is removed when the test ends.
struct Project {
    root: PathBuf,
}

impl Project {
    fn new(name: &str, manifest: &str, entry: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("intolang-inex-test-{}-{name}", std::process::id()));
        std::fs::create_dir_all(root.join("src/main")).unwrap();
        std::fs::write(root.join("into-package.json"), manifest).unwrap();
        std::fs::write(root.join("src/main/hello.into"), entry).unwrap();
        Self { root }
    }

    fn inex(&self, name: &str) -> PathBuf {
        self.executable(&format!("{name}.inex"))
    }

    fn executable(&self, name: &str) -> PathBuf {
        self.root
            .join("dist")
            .join(std::env::consts::OS)
            .join(std::env::consts::ARCH)
            .join(name)
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).ok();
    }
}

fn into(arguments: &[&Path]) -> Output {
    Command::new(INTO).args(arguments).output().unwrap()
}

fn manifest(name: &str, field: &str) -> String {
    format!(r#"{{"name":"{name}","alias":"{name}","version":"0.3.0","LICENCE":"MIT",{field}}}"#)
}

#[test]
fn build_vm_writes_an_inex_file_that_into_runs() {
    let project = Project::new(
        "runs",
        &manifest("hello", r#""runs":"src/main/hello.into""#),
        r#"import "intolang://builtin/std";
println("Hello VM");
"#,
    );
    let built = into(&[Path::new("build"), Path::new("--vm"), &project.root]);
    assert!(built.status.success(), "{built:?}");
    let file = project.inex("hello");
    let bytes = std::fs::read(&file).unwrap();
    assert!(bytes.starts_with(b"#!/usr/bin/env into\nINEX"));
    let inex = intovm::Inex::try_from(bytes.as_slice()).unwrap();
    for path in ["into-package.json", "src/main/hello.into", "intoc"] {
        assert!(inex.bundle.get(path).is_some(), "{path}");
    }
    assert_eq!(
        inex.bundle.read("src/main/hello.into"),
        Some(
            r#"import "intolang://builtin/std";
println("Hello VM");
"#
            .as_bytes()
        )
    );
    let ran = into(&[&file]);
    assert_eq!(
        String::from_utf8_lossy(&ran.stdout),
        r#"Hello VM
"#
    );
    assert!(ran.status.success(), "{ran:?}");
}

#[test]
fn an_inex_file_runs_without_the_project() {
    let project = Project::new(
        "moved",
        &manifest("hello", r#""runs":"src/main/hello.into""#),
        r#"import "intolang://builtin/std";
println("Hello VM");
"#,
    );
    into(&[Path::new("build"), Path::new("--vm"), &project.root]);
    let elsewhere = std::env::temp_dir().join(format!(
        "intolang-inex-test-{}-moved-copy.inex",
        std::process::id()
    ));
    std::fs::copy(project.inex("hello"), &elsewhere).unwrap();
    drop(project);
    let ran = into(&[Path::new("run"), &elsewhere]);
    std::fs::remove_file(&elsewhere).ok();
    assert_eq!(
        String::from_utf8_lossy(&ran.stdout),
        r#"Hello VM
"#
    );
}

#[test]
fn a_required_directory_travels_with_the_inex_file() {
    let project = Project::new(
        "assets",
        &manifest("hello", r#""runs":"src/main/hello.into""#),
        r#"import "intolang://builtin/std";
import "asset://require";

require("res");
now into data = Assets.read("res/a.txt");
println(data.text());
"#,
    );
    std::fs::create_dir_all(project.root.join("res")).unwrap();
    std::fs::write(project.root.join("res/a.txt"), "asset text").unwrap();
    std::fs::create_dir_all(project.root.join("unused")).unwrap();
    std::fs::write(project.root.join("unused/b.txt"), "left out").unwrap();
    let built = into(&[Path::new("build"), Path::new("--vm"), &project.root]);
    assert!(built.status.success(), "{built:?}");
    let bytes = std::fs::read(project.inex("hello")).unwrap();
    let inex = intovm::Inex::try_from(bytes.as_slice()).unwrap();
    assert_eq!(inex.bundle.read("res/a.txt"), Some(&b"asset text"[..]));
    assert!(inex.bundle.get("unused").is_none());
    let elsewhere = std::env::temp_dir().join(format!(
        "intolang-inex-test-{}-assets-copy.inex",
        std::process::id()
    ));
    std::fs::copy(project.inex("hello"), &elsewhere).unwrap();
    drop(project);
    let ran = into(&[Path::new("run"), &elsewhere]);
    std::fs::remove_file(&elsewhere).ok();
    assert_eq!(
        String::from_utf8_lossy(&ran.stdout),
        r#"asset text
"#,
        "{ran:?}"
    );
}

#[test]
fn a_native_executable_carries_the_required_directory() {
    let project = Project::new(
        "native-assets",
        &manifest("hello", r#""runs":"src/main/hello.into""#),
        r#"import "intolang://builtin/std";
import "asset://require";

require("res");
now into data = Assets.read("res/a.txt");
println(data.text());
now into raw = Assets.read("res/inner/b.bin");
println(raw.length());
println(raw.at(1));
"#,
    );
    std::fs::create_dir_all(project.root.join("res/inner")).unwrap();
    std::fs::write(project.root.join("res/a.txt"), "asset text").unwrap();
    std::fs::write(project.root.join("res/inner/b.bin"), [0u8, 255, 1]).unwrap();
    let built = into(&[Path::new("build"), &project.root]);
    assert!(built.status.success(), "{built:?}");
    let directory = std::env::temp_dir().join(format!(
        "intolang-inex-test-{}-native-assets-copy",
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let elsewhere = directory.join("hello");
    std::fs::copy(project.executable("hello"), &elsewhere).unwrap();
    drop(project);
    let ran = Command::new(&elsewhere).output().unwrap();
    assert_eq!(
        String::from_utf8_lossy(&ran.stdout),
        r#"asset text
3
255
"#,
        "{ran:?}"
    );
    let left: Vec<_> = std::fs::read_dir(&directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(left, ["hello"]);
    std::fs::remove_dir_all(&directory).ok();
}

#[test]
fn a_file_named_by_asset_travels_alone_in_both_executables() {
    let project = Project::new(
        "asset-file",
        &manifest("hello", r#""runs":"src/main/hello.into""#),
        r#"import "intolang://builtin/std";
import "asset://require";

now into data = Assets.read(asset("res/a.txt"));
println(data.text());
"#,
    );
    std::fs::create_dir_all(project.root.join("res")).unwrap();
    std::fs::write(project.root.join("res/a.txt"), "asset text").unwrap();
    std::fs::write(project.root.join("res/other.txt"), "left out").unwrap();
    let built = into(&[Path::new("build"), Path::new("--vm"), &project.root]);
    assert!(built.status.success(), "{built:?}");
    let bytes = std::fs::read(project.inex("hello")).unwrap();
    let inex = intovm::Inex::try_from(bytes.as_slice()).unwrap();
    assert_eq!(inex.bundle.read("res/a.txt"), Some(&b"asset text"[..]));
    assert!(inex.bundle.get("res/other.txt").is_none());
    let built = into(&[Path::new("build"), &project.root]);
    assert!(built.status.success(), "{built:?}");
    let directory = std::env::temp_dir().join(format!(
        "intolang-inex-test-{}-asset-file-copy",
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let inex_copy = directory.join("hello.inex");
    let native_copy = directory.join("hello");
    std::fs::copy(project.inex("hello"), &inex_copy).unwrap();
    std::fs::copy(project.executable("hello"), &native_copy).unwrap();
    drop(project);
    let in_vm = into(&[Path::new("run"), &inex_copy]);
    let native = Command::new(&native_copy).output().unwrap();
    for ran in [in_vm, native] {
        assert_eq!(
            String::from_utf8_lossy(&ran.stdout),
            r#"asset text
"#,
            "{ran:?}"
        );
    }
    std::fs::remove_dir_all(&directory).ok();
}

#[test]
fn a_missing_asset_is_an_error_in_the_vm_and_in_a_native_executable() {
    let project = Project::new(
        "missing-asset",
        &manifest("hello", r#""runs":"src/main/hello.into""#),
        r#"import "intolang://builtin/std";
import "asset://require";

require("res");
now into data = Assets.read("res/missing.txt");
println(data.text());
"#,
    );
    std::fs::create_dir_all(project.root.join("res")).unwrap();
    std::fs::write(project.root.join("res/a.txt"), "asset text").unwrap();
    let ran = into(&[Path::new("run"), Path::new("--vm"), &project.root]);
    assert!(!ran.status.success());
    assert!(
        String::from_utf8_lossy(&ran.stderr).contains("is not in the bundle"),
        "{ran:?}"
    );
    let built = into(&[Path::new("build"), &project.root]);
    assert!(built.status.success(), "{built:?}");
    let native = Command::new(project.executable("hello")).output().unwrap();
    assert!(!native.status.success());
    assert!(
        String::from_utf8_lossy(&native.stderr).contains("is not in the bundle"),
        "{native:?}"
    );
}

#[test]
fn a_required_directory_that_does_not_exist_is_an_error() {
    let project = Project::new(
        "missing-assets",
        &manifest("hello", r#""runs":"src/main/hello.into""#),
        r#"import "intolang://builtin/std";
import "asset://require";

require("res");
"#,
    );
    let built = into(&[Path::new("build"), Path::new("--vm"), &project.root]);
    assert!(!built.status.success());
    assert!(
        String::from_utf8_lossy(&built.stderr).contains("`res` is named to be bundled"),
        "{built:?}"
    );
}

#[test]
fn run_vm_builds_and_runs_in_one_step() {
    let project = Project::new(
        "run-vm",
        &manifest("hello", r#""runs":"src/main/hello.into""#),
        r#"import "intolang://builtin/std";
println("Hello VM");
"#,
    );
    let ran = into(&[Path::new("run"), Path::new("--vm"), &project.root]);
    assert!(
        String::from_utf8_lossy(&ran.stdout).ends_with(
            r#"Hello VM
"#
        ),
        "{ran:?}"
    );
    assert!(project.inex("hello").is_file());
}

#[test]
fn a_library_package_has_no_inex_form() {
    let project = Project::new(
        "library",
        &manifest("tool", r#""library":"src/main/lib.into""#),
        "",
    );
    let built = into(&[Path::new("build"), Path::new("--vm"), &project.root]);
    assert!(!built.status.success());
    assert!(
        String::from_utf8_lossy(&built.stderr).contains("a library package has no .inex form"),
        "{built:?}"
    );
}

#[test]
fn a_file_that_is_not_inex_is_rejected() {
    let path = std::env::temp_dir().join(format!(
        "intolang-inex-test-{}-broken.inex",
        std::process::id()
    ));
    std::fs::write(
        &path,
        r#"#!/usr/bin/env into
ELF 1
"#,
    )
    .unwrap();
    let ran = into(&[&path]);
    std::fs::remove_file(&path).ok();
    assert!(!ran.status.success());
    assert!(
        String::from_utf8_lossy(&ran.stderr).contains("Not an inex file"),
        "{ran:?}"
    );
}

#[test]
fn an_unsupported_version_is_rejected_before_running() {
    let path = std::env::temp_dir().join(format!(
        "intolang-inex-test-{}-future.inex",
        std::process::id()
    ));
    std::fs::write(
        &path,
        r#"#!/usr/bin/env into
INEX 99
"#,
    )
    .unwrap();
    let ran = into(&[&path]);
    std::fs::remove_file(&path).ok();
    assert!(!ran.status.success());
    assert!(
        String::from_utf8_lossy(&ran.stderr).contains("inex version 99 is not supported"),
        "{ran:?}"
    );
}
