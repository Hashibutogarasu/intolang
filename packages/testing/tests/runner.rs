use std::path::Path;
use testing::{Package, Workspace, run_tests};

const IMPORTS: &str = "import \"intolang://builtin/tests/macros/test\";\nimport \"intolang://builtin/tests/assert\";\nimport \"intolang://builtin/tests/memory-check\";\nimport \"intolang://builtin/tests/ownership-check\";\n";

const CHECKS: [(&str, &str, bool); 10] = [
    ("assertOwned", "isOwned", false),
    ("assertNotOwned", "isOwned", true),
    ("assertReference", "isReference", false),
    ("assertNotReference", "isReference", true),
    ("assertBorrowed", "isBorrowed", false),
    ("assertNotBorrowed", "isBorrowed", true),
    ("assertMoved", "isMoved", false),
    ("assertNotMoved", "isMoved", true),
    ("assertDestroyed", "isDestroyed", false),
    ("assertNotDestroyed", "isDestroyed", true),
];

fn ownership_check() -> String {
    CHECKS
        .iter()
        .map(|(name, property, fails_when)| {
            format!(
                "pub fun {name}(value: Object) {{\n    if value.{property} == {fails_when} {{\n        throw \"{name} failed\";\n    }}\n}}\n\n"
            )
        })
        .collect()
}

fn builtin() -> Package {
    let mut info = String::from(
        "export \"builtin://tests/macros/test/test\";\nexport \"builtin://tests/assert/assert\";\nexport \"builtin://tests/assert/assertTrue\";\nexport \"builtin://tests/assert/assertEquals\";\nexport \"builtin://tests/memory-check/assertMemoryExists\";\nexport \"builtin://tests/memory-check/assertMemoryNotExists\";\n",
    );
    for (name, _, _) in CHECKS {
        info.push_str(&format!(
            "export \"builtin://tests/ownership-check/{name}\";\n"
        ));
    }
    Package::new("builtin")
        .file("src/main/into-info.into", info)
        .file(
            "src/main/builtin/tests/macros/test.into",
            "pub macro test {\n    #[test]\n}\n",
        )
        .file(
            "src/main/builtin/tests/assert.into",
            "pub fun assert(condition: Boolean) {\n    if condition == false {\n        throw \"assertion failed\";\n    }\n}\n\npub fun assertTrue(condition: Boolean) {\n    if condition == false {\n        throw \"expected true\";\n    }\n}\n\npub fun assertEquals(expected: Int32, actual: Int32) {\n    if expected != actual {\n        throw \"values are not equal\";\n    }\n}\n",
        )
        .file(
            "src/main/builtin/tests/memory-check.into",
            "pub fun assertMemoryExists(value: Object) {\n    if value.isInMemory == false {\n        throw \"memory does not exist\";\n    }\n}\n\npub fun assertMemoryNotExists(value: Object) {\n    if value.isInMemory == true {\n        throw \"memory exists\";\n    }\n}\n",
        )
        .file("src/main/builtin/tests/ownership-check.into", ownership_check())
}

fn run(source: &str) -> anyhow::Result<testing::TestSummary> {
    let workspace = Workspace::with_packages(&[builtin()])?;
    run_tests(source, Path::new("memory.into"), workspace.path())
}

fn outcome(body: &str) -> (usize, usize) {
    let source = format!("{IMPORTS}\n#[test]\npub fun checked() {{\n{body}}}\n");
    let summary = run(&source).unwrap();
    (summary.passed, summary.failed)
}

#[test]
fn runs_passing_tests() {
    let source = format!(
        "{IMPORTS}\n#[entrypoint]\npub MemoryTest() {{\n    #[test]\n    pub fun nowIsAllocated() {{\n        now into mem1 = 0;\n        assertMemoryExists(mem1);\n    }}\n\n    #[test]\n    pub fun lateIsAllocatedOnUse() {{\n        late into mem1 = 0;\n        assertMemoryExists(mem1);\n    }}\n\n    #[test]\n    pub fun localExistsOnlyInInit() {{\n        into mem1 = 0;\n        assertMemoryNotExists(mem1);\n        init {{\n            assertMemoryExists(mem1);\n        }}\n    }}\n}}\n"
    );
    let summary = run(&source).unwrap();
    assert_eq!((summary.passed, summary.failed), (3, 0));
}

#[test]
fn counts_failed_assertions() {
    let source = format!(
        "{IMPORTS}\n#[test]\npub fun failing() {{\n    into mem1 = 0;\n    assertMemoryExists(mem1);\n}}\n\n#[test]\npub fun unloaded() {{\n    now into mem2 = 1;\n    unload mem2;\n    assertMemoryNotExists(mem2);\n}}\n"
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
fn helpers_that_are_not_imported_cannot_be_called() {
    let imports = "import \"intolang://builtin/tests/macros/test\";\nimport \"intolang://builtin/tests/assert\";\n";
    let source = format!(
        "{imports}#[test]\npub fun missing() {{\n    now into mem1 = 0;\n    assertMemoryExists(mem1);\n}}\n"
    );
    let summary = run(&source).unwrap();
    assert_eq!((summary.passed, summary.failed), (0, 1));
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

#[test]
fn fresh_bindings_are_owned() {
    assert_eq!(
        outcome(
            "    now into dog = 1;\n    assertOwned(dog);\n    assertNotReference(dog);\n    assertNotBorrowed(dog);\n    assertNotMoved(dog);\n    assertNotDestroyed(dog);\n"
        ),
        (1, 0)
    );
    assert_eq!(
        outcome("    now into dog = 1;\n    assertNotOwned(dog);\n"),
        (0, 1)
    );
}

#[test]
fn move_hands_ownership_to_the_new_binding() {
    assert_eq!(
        outcome(
            "    now into dog = 1;\n    now into moved = dog.move();\n    assertMoved(dog);\n    assertNotOwned(dog);\n    assertOwned(moved);\n    assertMemoryNotExists(dog);\n    assertMemoryExists(moved);\n"
        ),
        (1, 0)
    );
}

#[test]
fn copy_creates_an_independent_owner() {
    assert_eq!(
        outcome(
            "    now into dog = 1;\n    now into twin = dog.copy();\n    assertOwned(dog);\n    assertOwned(twin);\n    twin.destroy();\n    assertDestroyed(twin);\n    assertNotDestroyed(dog);\n"
        ),
        (1, 0)
    );
}

#[test]
fn borrow_gives_a_reference_that_owns_nothing() {
    assert_eq!(
        outcome(
            "    now into dog = 1;\n    now into view = dog.borrow();\n    assertBorrowed(dog);\n    assertReference(view);\n    assertNotOwned(view);\n    assertOwned(dog);\n    view.destroy();\n    assertNotBorrowed(dog);\n    assertNotDestroyed(dog);\n"
        ),
        (1, 0)
    );
}

#[test]
fn destroy_removes_the_memory() {
    assert_eq!(
        outcome(
            "    now into dog = 1;\n    dog.destroy();\n    assertDestroyed(dog);\n    assertMemoryNotExists(dog);\n    assertNotOwned(dog);\n"
        ),
        (1, 0)
    );
}

#[test]
fn each_ownership_assertion_fails_when_it_does_not_hold() {
    for body in [
        "    now into dog = 1;\n    assertMoved(dog);\n",
        "    now into dog = 1;\n    assertDestroyed(dog);\n",
        "    now into dog = 1;\n    assertBorrowed(dog);\n",
        "    now into dog = 1;\n    assertReference(dog);\n",
        "    now into dog = 1;\n    now into moved = dog.move();\n    assertNotMoved(dog);\n",
        "    now into dog = 1;\n    dog.destroy();\n    assertNotDestroyed(dog);\n",
        "    now into dog = 1;\n    now into view = dog.borrow();\n    assertNotBorrowed(dog);\n",
        "    now into dog = 1;\n    now into view = dog.borrow();\n    assertNotReference(view);\n",
    ] {
        assert_eq!(outcome(body), (0, 1), "{body}");
    }
}

#[test]
fn using_a_moved_binding_fails_the_test() {
    assert_eq!(
        outcome(
            "    now into dog = 1;\n    now into moved = dog.move();\n    now into again = dog.move();\n"
        ),
        (0, 1)
    );
    assert_eq!(
        outcome(
            "    now into dog = 1;\n    now into view = dog.borrow();\n    now into moved = dog.move();\n"
        ),
        (0, 1)
    );
}

#[test]
fn assert_true_and_assert_equals_compare_values() {
    assert_eq!(
        outcome(
            "    assertTrue(true);\n    assertEquals(3, 3);\n    now into count = 4;\n    assertEquals(4, count);\n"
        ),
        (1, 0)
    );
    assert_eq!(outcome("    assertTrue(false);\n"), (0, 1));
    assert_eq!(outcome("    assertEquals(3, 4);\n"), (0, 1));
}
