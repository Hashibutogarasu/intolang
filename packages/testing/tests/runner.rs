use std::path::Path;
use testing::{Package, Workspace, run_tests};

const IMPORTS: &str = "import \"intolang://builtin/tests/macros/test\";\nimport \"intolang://builtin/tests/assert\";\nimport \"intolang://builtin/tests/memory-check\";\n";

fn builtin() -> Package {
    Package::new("builtin")
        .file(
            "src/main/into-info.into",
            "export \"builtin://tests/macros/test/test\";\nexport \"builtin://tests/assert/assert\";\nexport \"builtin://tests/memory-check/memoryExists\";\nexport \"builtin://tests/memory-check/memoryNotExists\";\n",
        )
        .file(
            "src/main/builtin/tests/macros/test.into",
            "pub macro test {\n    #[test]\n}\n",
        )
        .file(
            "src/main/builtin/tests/assert.into",
            "pub fun assert(condition: Boolean) {\n    if condition == false {\n        throw \"assertion failed\";\n    }\n}\n",
        )
        .file(
            "src/main/builtin/tests/memory-check.into",
            "pub fun memoryExists(value: Object): Boolean {\n    return value.isInMemory;\n}\n\npub fun memoryNotExists(value: Object): Boolean {\n    return !memoryExists(value);\n}\n",
        )
}

fn run(source: &str) -> anyhow::Result<testing::TestSummary> {
    let workspace = Workspace::with_packages(&[builtin()])?;
    run_tests(source, Path::new("memory.into"), workspace.path())
}

#[test]
fn runs_passing_tests() {
    let source = format!(
        "{IMPORTS}\n#[entrypoint]\npub MemoryTest() {{\n    #[test]\n    pub fun nowIsAllocated() {{\n        now into mem1 = 0;\n        assert(memoryExists(mem1));\n    }}\n\n    #[test]\n    pub fun lateIsAllocatedOnUse() {{\n        late into mem1 = 0;\n        assert(memoryExists(mem1));\n    }}\n\n    #[test]\n    pub fun localExistsOnlyInInit() {{\n        into mem1 = 0;\n        assert(memoryNotExists(mem1));\n        init {{\n            assert(memoryExists(mem1));\n        }}\n    }}\n}}\n"
    );
    let summary = run(&source).unwrap();
    assert_eq!((summary.passed, summary.failed), (3, 0));
}

#[test]
fn counts_failed_assertions() {
    let source = format!(
        "{IMPORTS}\n#[test]\npub fun failing() {{\n    into mem1 = 0;\n    assert(memoryExists(mem1));\n}}\n\n#[test]\npub fun unloaded() {{\n    now into mem2 = 1;\n    unload mem2;\n    assert(memoryNotExists(mem2));\n}}\n"
    );
    let summary = run(&source).unwrap();
    assert_eq!((summary.passed, summary.failed), (1, 1));
}

#[test]
fn requires_the_test_helpers_to_be_imported() {
    let error = run("#[test]\npub fun nothing() {}\n").unwrap_err();
    assert!(
        error
            .to_string()
            .contains("test module must import `assert`"),
        "{error}"
    );
    let imports = "import \"intolang://builtin/tests/assert\";\nimport \"intolang://builtin/tests/memory-check\";\n";
    let error = run(&format!("{imports}#[test]\npub fun nothing() {{}}\n")).unwrap_err();
    assert!(
        error.to_string().contains("must import the test macro"),
        "{error}"
    );
}

#[test]
fn rejects_files_without_tests() {
    let error = run(&format!("{IMPORTS}pub fun helper() {{}}\n")).unwrap_err();
    assert!(
        error.to_string().contains("no #[test] functions found"),
        "{error}"
    );
}

#[test]
fn reports_syntax_errors_with_the_file_path() {
    let error = run(&format!("{IMPORTS}#[test]\npub fun broken() {{\n")).unwrap_err();
    assert!(error.to_string().contains("memory.into"), "{error}");
}
