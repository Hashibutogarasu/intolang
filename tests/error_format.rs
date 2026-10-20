use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const INTO: &str = env!("CARGO_BIN_EXE_intolang");

struct Project {
    root: PathBuf,
}

impl Project {
    fn new(name: &str, entry: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "intolang-error-format-test-{}-{name}",
            std::process::id()
        ));
        std::fs::create_dir_all(root.join("src/main")).unwrap();
        std::fs::write(
            root.join("into-package.json"),
            r#"{"name":"bad","alias":"bad","version":"0.1.0","LICENCE":"MIT","runs":"src/main/bad.into"}"#,
        )
        .unwrap();
        std::fs::write(root.join("src/main/bad.into"), entry).unwrap();
        Self { root }
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).ok();
    }
}

fn into(arguments: &[&Path], format: &str) -> Output {
    Command::new(INTO)
        .args(["--error-format", format])
        .args(arguments)
        .output()
        .unwrap()
}

#[test]
fn the_human_format_writes_the_error_and_its_code_to_stderr() {
    let project = Project::new("human", "init { into x = ; }");
    let output = into(
        &[Path::new("build"), Path::new("--vm"), &project.root],
        "human",
    );
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error code: 0x"), "{stderr}");
    assert!(output.stdout.is_empty(), "{output:?}");
}

#[test]
fn a_build_that_fails_marks_its_last_line_with_a_cross() {
    let project = Project::new("cross", "init { into x = ; }");
    let output = into(
        &[Path::new("build"), Path::new("--vm"), &project.root],
        "human",
    );
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    let cross = stderr
        .lines()
        .find(|line| line.starts_with("[✗]"))
        .unwrap_or_else(|| panic!("{stderr}"));
    assert!(cross.contains("Lowering the Into sources"), "{stderr}");
    assert!(!stderr.contains("[✓]"), "{stderr}");
}

#[test]
fn the_json_format_writes_the_context_to_stdout() {
    let project = Project::new("json", "init { into x = ; }");
    let output = into(
        &[Path::new("build"), Path::new("--vm"), &project.root],
        "json",
    );
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("error code"), "{stderr}");
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(report["context"]["code"].is_u64(), "{report}");
    assert_eq!(report["context"]["domain"], "into_lang", "{report}");
    assert!(report["trace"].is_array(), "{report}");
}

#[test]
fn an_error_of_the_toolchain_has_no_into_layer() {
    let output = into(
        &[Path::new("build"), Path::new("/nonexistent/package")],
        "json",
    );
    assert!(!output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["context"]["domain"], "compiler", "{report}");
    let layers = report["trace"].as_array().unwrap();
    assert!(
        layers.iter().all(|layer| layer["layer"] != "into"),
        "{report}"
    );
}
