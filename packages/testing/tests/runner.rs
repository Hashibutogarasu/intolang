use std::path::Path;
use testing::{Package, TestSummary, Workspace, run_tests};

fn builtin() -> Package {
    Package::new("builtin")
        .file(
            "src/main/into-info.into",
            r#"export "builtin://tests/macros/test/test";
export "builtin://tests/assert/assert";
export "builtin://tests/assert/assertTrue";
export "builtin://tests/assert/assertEquals";
export "builtin://tests/memory-check/assertMemoryExists";
export "builtin://tests/memory-check/assertMemoryNotExists";
export "builtin://tests/ownership-check/assertOwned";
export "builtin://tests/ownership-check/assertNotOwned";
export "builtin://tests/ownership-check/assertReference";
export "builtin://tests/ownership-check/assertNotReference";
export "builtin://tests/ownership-check/assertBorrowed";
export "builtin://tests/ownership-check/assertNotBorrowed";
export "builtin://tests/ownership-check/assertMoved";
export "builtin://tests/ownership-check/assertNotMoved";
export "builtin://tests/ownership-check/assertDestroyed";
export "builtin://tests/ownership-check/assertNotDestroyed";
"#,
        )
        .file(
            "src/main/tests/macros/test.into",
            r#"pub macro test {
    #[test]
}
"#,
        )
        .file(
            "src/main/tests/assert.into",
            r#"pub fun assert(condition: Boolean) {
    if condition == false {
        throw "assertion failed";
    }
}

pub fun assertTrue(condition: Boolean) {
    if condition == false {
        throw "expected true";
    }
}

pub fun assertEquals(expected: Int32, actual: Int32) {
    if expected != actual {
        throw "values are not equal";
    }
}
"#,
        )
        .file(
            "src/main/tests/memory-check.into",
            r#"#[inspects]
pub fun assertMemoryExists(value: Object) {
    if value.isInMemory == false {
        throw "memory does not exist";
    }
}

#[inspects]
pub fun assertMemoryNotExists(value: Object) {
    if value.isInMemory == true {
        throw "memory exists";
    }
}
"#,
        )
        .file(
            "src/main/tests/ownership-check.into",
            r#"#[inspects]
pub fun assertOwned(value: Object) {
    if value.isOwned == false {
        throw "assertOwned failed";
    }
}

#[inspects]
pub fun assertNotOwned(value: Object) {
    if value.isOwned == true {
        throw "assertNotOwned failed";
    }
}

#[inspects]
pub fun assertReference(value: Object) {
    if value.isReference == false {
        throw "assertReference failed";
    }
}

#[inspects]
pub fun assertNotReference(value: Object) {
    if value.isReference == true {
        throw "assertNotReference failed";
    }
}

#[inspects]
pub fun assertBorrowed(value: Object) {
    if value.isBorrowed == false {
        throw "assertBorrowed failed";
    }
}

#[inspects]
pub fun assertNotBorrowed(value: Object) {
    if value.isBorrowed == true {
        throw "assertNotBorrowed failed";
    }
}

#[inspects]
pub fun assertMoved(value: Object) {
    if value.isMoved == false {
        throw "assertMoved failed";
    }
}

#[inspects]
pub fun assertNotMoved(value: Object) {
    if value.isMoved == true {
        throw "assertNotMoved failed";
    }
}

#[inspects]
pub fun assertDestroyed(value: Object) {
    if value.isDestroyed == false {
        throw "assertDestroyed failed";
    }
}

#[inspects]
pub fun assertNotDestroyed(value: Object) {
    if value.isDestroyed == true {
        throw "assertNotDestroyed failed";
    }
}
"#,
        )
}

fn run(source: &str) -> errors::Result<TestSummary> {
    let workspace = Workspace::with_packages(&[builtin()])?;
    run_tests(source, Path::new("memory.into"), workspace.path())
}

fn counts(source: &str) -> (usize, usize) {
    let summary = run(source).unwrap();
    (summary.passed, summary.failed)
}

#[test]
fn runs_passing_tests() {
    let summary = run(r#"
import "intolang://builtin/tests/macros/test";
import "intolang://builtin/tests/assert";
import "intolang://builtin/tests/memory-check";
import "intolang://builtin/tests/ownership-check";

#[entrypoint]
pub MemoryTest() {
    #[test]
    pub fun nowIsAllocated() {
        now into mem1 = 0;
        assertMemoryExists(mem1);
    }

    #[test]
    pub fun lateIsAllocatedOnUse() {
        late into mem1 = 0;
        assertMemoryExists(mem1);
    }

    #[test]
    pub fun localExistsOnlyInInit() {
        into mem1 = 0;
        assertMemoryNotExists(mem1);
        init {
            assertMemoryExists(mem1);
        }
    }
}
"#)
    .unwrap();
    assert_eq!((summary.passed, summary.failed), (3, 0));
}

#[test]
fn counts_failed_assertions() {
    assert_eq!(
        counts(
            r#"
import "intolang://builtin/tests/macros/test";
import "intolang://builtin/tests/assert";
import "intolang://builtin/tests/memory-check";
import "intolang://builtin/tests/ownership-check";

#[test]
pub fun failing() {
    into mem1 = 0;
    assertMemoryExists(mem1);
}

#[test]
pub fun unloaded() {
    now into mem2 = 1;
    unload mem2;
    assertMemoryNotExists(mem2);
}
"#
        ),
        (1, 1)
    );
}

#[test]
fn requires_the_test_helpers_to_be_imported() {
    let error = run(r#"
#[test]
pub fun nothing() {}
"#)
    .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("test module must import `assert`"),
        "{error}"
    );
    let error = run(r#"
import "intolang://builtin/tests/assert";
import "intolang://builtin/tests/memory-check";
#[test]
pub fun nothing() {}
"#)
    .unwrap_err();
    assert!(
        error.to_string().contains("must import the test macro"),
        "{error}"
    );
}

#[test]
fn helpers_that_are_not_imported_cannot_be_called() {
    assert_eq!(
        counts(
            r#"
import "intolang://builtin/tests/macros/test";
import "intolang://builtin/tests/assert";
#[test]
pub fun missing() {
    now into mem1 = 0;
    assertMemoryExists(mem1);
}
"#
        ),
        (0, 1)
    );
}

#[test]
fn rejects_files_without_tests() {
    let error = run(r#"
import "intolang://builtin/tests/macros/test";
import "intolang://builtin/tests/assert";
import "intolang://builtin/tests/memory-check";
import "intolang://builtin/tests/ownership-check";
pub fun helper() {}
"#)
    .unwrap_err();
    assert!(
        error.to_string().contains("no #[test] functions found"),
        "{error}"
    );
}

#[test]
fn reports_syntax_errors_with_the_file_path() {
    let error = run(r#"
import "intolang://builtin/tests/macros/test";
import "intolang://builtin/tests/assert";
import "intolang://builtin/tests/memory-check";
import "intolang://builtin/tests/ownership-check";
#[test]
pub fun broken() {
"#)
    .unwrap_err();
    assert!(error.to_string().contains("memory.into"), "{error}");
}

#[test]
fn fresh_bindings_are_owned() {
    assert_eq!(
        counts(
            r#"
import "intolang://builtin/tests/macros/test";
import "intolang://builtin/tests/assert";
import "intolang://builtin/tests/memory-check";
import "intolang://builtin/tests/ownership-check";

pub Dog() {
    pub into: Int32 age = 1;
}

#[test]
pub fun checked() {
    now into dog = Dog();
    assertOwned(dog);
    assertNotReference(dog);
    assertNotBorrowed(dog);
    assertNotMoved(dog);
    assertNotDestroyed(dog);
}
"#
        ),
        (1, 0)
    );
    assert_eq!(
        counts(
            r#"
import "intolang://builtin/tests/macros/test";
import "intolang://builtin/tests/assert";
import "intolang://builtin/tests/memory-check";
import "intolang://builtin/tests/ownership-check";

pub Dog() {
    pub into: Int32 age = 1;
}

#[test]
pub fun checked() {
    now into dog = Dog();
    assertNotOwned(dog);
}
"#
        ),
        (0, 1)
    );
}

#[test]
fn move_hands_ownership_to_the_new_binding() {
    assert_eq!(
        counts(
            r#"
import "intolang://builtin/tests/macros/test";
import "intolang://builtin/tests/assert";
import "intolang://builtin/tests/memory-check";
import "intolang://builtin/tests/ownership-check";

pub Dog() {
    pub into: Int32 age = 1;
}

#[test]
pub fun checked() {
    now into dog = Dog();
    now into moved = dog.move();
    assertMoved(dog);
    assertNotOwned(dog);
    assertOwned(moved);
    assertMemoryNotExists(dog);
    assertMemoryExists(moved);
}
"#
        ),
        (1, 0)
    );
}

#[test]
fn copy_creates_an_independent_owner() {
    assert_eq!(
        counts(
            r#"
import "intolang://builtin/tests/macros/test";
import "intolang://builtin/tests/assert";
import "intolang://builtin/tests/memory-check";
import "intolang://builtin/tests/ownership-check";

pub Dog() {
    pub into: Int32 age = 1;
}

#[test]
pub fun checked() {
    now into dog = Dog();
    now into twin = dog.copy();
    assertOwned(dog);
    assertOwned(twin);
    twin.destroy();
    assertDestroyed(twin);
    assertNotDestroyed(dog);
}
"#
        ),
        (1, 0)
    );
}

#[test]
fn borrow_gives_a_reference_that_owns_nothing() {
    assert_eq!(
        counts(
            r#"
import "intolang://builtin/tests/macros/test";
import "intolang://builtin/tests/assert";
import "intolang://builtin/tests/memory-check";
import "intolang://builtin/tests/ownership-check";

pub Dog() {
    pub into: Int32 age = 1;
}

#[test]
pub fun checked() {
    now into dog = Dog();
    now into view = dog.borrow();
    assertBorrowed(dog);
    assertReference(view);
    assertNotOwned(view);
    assertOwned(dog);
    view.destroy();
    assertNotBorrowed(dog);
    assertNotDestroyed(dog);
}
"#
        ),
        (1, 0)
    );
}

#[test]
fn destroy_removes_the_memory() {
    assert_eq!(
        counts(
            r#"
import "intolang://builtin/tests/macros/test";
import "intolang://builtin/tests/assert";
import "intolang://builtin/tests/memory-check";
import "intolang://builtin/tests/ownership-check";

pub Dog() {
    pub into: Int32 age = 1;
}

#[test]
pub fun checked() {
    now into dog = Dog();
    dog.destroy();
    assertDestroyed(dog);
    assertMemoryNotExists(dog);
    assertNotOwned(dog);
}
"#
        ),
        (1, 0)
    );
}

#[test]
fn each_ownership_assertion_fails_when_it_does_not_hold() {
    for body in [
        r#"
    now into dog = Dog();
    assertMoved(dog);
"#,
        r#"
    now into dog = Dog();
    assertDestroyed(dog);
"#,
        r#"
    now into dog = Dog();
    assertBorrowed(dog);
"#,
        r#"
    now into dog = Dog();
    assertReference(dog);
"#,
        r#"
    now into dog = Dog();
    now into moved = dog.move();
    assertNotMoved(dog);
"#,
        r#"
    now into dog = Dog();
    dog.destroy();
    assertNotDestroyed(dog);
"#,
        r#"
    now into dog = Dog();
    now into view = dog.borrow();
    assertNotBorrowed(dog);
"#,
        r#"
    now into dog = Dog();
    now into view = dog.borrow();
    assertNotReference(view);
"#,
    ] {
        let source = format!(
            r#"
import "intolang://builtin/tests/macros/test";
import "intolang://builtin/tests/assert";
import "intolang://builtin/tests/memory-check";
import "intolang://builtin/tests/ownership-check";

pub Dog() {{
    pub into: Int32 age = 1;
}}

#[test]
pub fun checked() {{{body}}}
"#
        );
        assert_eq!(counts(&source), (0, 1), "{body}");
    }
}

#[test]
fn using_a_moved_binding_fails_the_test() {
    assert_eq!(
        counts(
            r#"
import "intolang://builtin/tests/macros/test";
import "intolang://builtin/tests/assert";
import "intolang://builtin/tests/memory-check";
import "intolang://builtin/tests/ownership-check";

pub Dog() {
    pub into: Int32 age = 1;
}

#[test]
pub fun checked() {
    now into dog = Dog();
    now into moved = dog.move();
    now into again = dog.move();
}
"#
        ),
        (0, 1)
    );
    assert_eq!(
        counts(
            r#"
import "intolang://builtin/tests/macros/test";
import "intolang://builtin/tests/assert";
import "intolang://builtin/tests/memory-check";
import "intolang://builtin/tests/ownership-check";

pub Dog() {
    pub into: Int32 age = 1;
}

#[test]
pub fun checked() {
    now into dog = Dog();
    now into view = dog.borrow();
    now into moved = dog.move();
}
"#
        ),
        (0, 1)
    );
}

#[test]
fn assert_true_and_assert_equals_compare_values() {
    assert_eq!(
        counts(
            r#"
import "intolang://builtin/tests/macros/test";
import "intolang://builtin/tests/assert";
import "intolang://builtin/tests/memory-check";
import "intolang://builtin/tests/ownership-check";

pub Dog() {
    pub into: Int32 age = 1;
}

#[test]
pub fun checked() {
    assertTrue(true);
    assertEquals(3, 3);
    now into count = 4;
    assertEquals(4, count);
}
"#
        ),
        (1, 0)
    );
    assert_eq!(
        counts(
            r#"
import "intolang://builtin/tests/macros/test";
import "intolang://builtin/tests/assert";
import "intolang://builtin/tests/memory-check";
import "intolang://builtin/tests/ownership-check";

pub Dog() {
    pub into: Int32 age = 1;
}

#[test]
pub fun checked() {
    assertTrue(false);
}
"#
        ),
        (0, 1)
    );
    assert_eq!(
        counts(
            r#"
import "intolang://builtin/tests/macros/test";
import "intolang://builtin/tests/assert";
import "intolang://builtin/tests/memory-check";
import "intolang://builtin/tests/ownership-check";

pub Dog() {
    pub into: Int32 age = 1;
}

#[test]
pub fun checked() {
    assertEquals(3, 4);
}
"#
        ),
        (0, 1)
    );
}
