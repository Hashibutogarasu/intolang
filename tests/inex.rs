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
        self.root
            .join("dist")
            .join(std::env::consts::OS)
            .join(std::env::consts::ARCH)
            .join(format!("{name}.inex"))
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
    format!(
        "{{\"name\":\"{name}\",\"alias\":\"{name}\",\"version\":\"0.3.0\",\"LICENCE\":\"MIT\",{field}}}"
    )
}

const HELLO: &str = "import \"intolang://builtin/std\";\nprintln(\"Hello VM\");\n";

#[test]
fn build_vm_writes_an_inex_file_that_into_runs() {
    let project = Project::new(
        "runs",
        &manifest("hello", "\"runs\":\"src/main/hello.into\""),
        HELLO,
    );
    let built = into(&[Path::new("build"), Path::new("--vm"), &project.root]);
    assert!(built.status.success(), "{built:?}");
    let file = project.inex("hello");
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(
        text.starts_with(
            "#!/usr/bin/env into\nINEX 1\nname:\thello\nalias:\thello\nversion:\t0.3.0\n"
        ),
        "{text}"
    );
    let ran = into(&[&file]);
    assert_eq!(String::from_utf8_lossy(&ran.stdout), "Hello VM\n");
    assert!(ran.status.success(), "{ran:?}");
}

#[test]
fn an_inex_file_runs_without_the_project() {
    let project = Project::new(
        "moved",
        &manifest("hello", "\"runs\":\"src/main/hello.into\""),
        HELLO,
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
    assert_eq!(String::from_utf8_lossy(&ran.stdout), "Hello VM\n");
}

#[test]
fn run_vm_builds_and_runs_in_one_step() {
    let project = Project::new(
        "run-vm",
        &manifest("hello", "\"runs\":\"src/main/hello.into\""),
        HELLO,
    );
    let ran = into(&[Path::new("run"), Path::new("--vm"), &project.root]);
    assert!(
        String::from_utf8_lossy(&ran.stdout).ends_with("Hello VM\n"),
        "{ran:?}"
    );
    assert!(project.inex("hello").is_file());
}

#[test]
fn a_library_package_has_no_inex_form() {
    let project = Project::new(
        "library",
        &manifest("tool", "\"library\":\"src/main/lib.into\""),
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
    std::fs::write(&path, "#!/usr/bin/env into\nELF 1\n").unwrap();
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
    std::fs::write(&path, "#!/usr/bin/env into\nINEX 99\n").unwrap();
    let ran = into(&[&path]);
    std::fs::remove_file(&path).ok();
    assert!(!ran.status.success());
    assert!(
        String::from_utf8_lossy(&ran.stderr).contains("inex version 99 is not supported"),
        "{ran:?}"
    );
}
