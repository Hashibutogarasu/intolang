use testing::eval;

#[test]
fn destroy_blocks_later_use() {
    let error = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
}

init {
    now into a = Counter();
    a.destroy();
    println(a.value);
}
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("`a` was destroyed"), "{error}");
}

#[test]
fn destroy_blocks_calling_methods_again() {
    let error = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
}

init {
    now into a = Counter();
    a.destroy();
    a.destroy();
}
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("`a` was destroyed"), "{error}");
}

#[test]
fn using_before_destroy_is_fine() {
    let output = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
}

init {
    now into a = Counter();
    println(a.value);
    a.destroy();
}
"#,
    )
    .unwrap();
    assert!(
        output.contains("NATIVE_FIELD_PRINT\ta\tCounter"),
        "{output}"
    );
}

#[test]
fn move_transfers_the_instance_and_invalidates_the_source() {
    let output = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
}

init {
    now into a = Counter();
    now into b = a.move();
    println(b.value);
}
"#,
    )
    .unwrap();
    assert!(output.contains("NATIVE_ALIAS\tb\ta"), "{output}");
    let error = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
}

init {
    now into a = Counter();
    now into b = a.move();
    println(a.value);
}
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("`a` was moved"), "{error}");
}

#[test]
fn moved_bindings_cannot_be_copied_or_moved_again() {
    let error = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
}

init {
    now into a = Counter();
    now into b = a.move();
    now into c = a.copy();
}
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("`a` was moved"), "{error}");
    let error = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
}

init {
    now into a = Counter();
    now into b = a.move();
    now into c = a;
}
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("`a` was moved"), "{error}");
}

#[test]
fn copy_creates_an_independent_instance() {
    let output = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
}

init {
    now into a = Counter();
    now into b = a.copy();
    println(a.value);
    println(b.value);
    b.destroy();
    println(a.value);
}
"#,
    )
    .unwrap();
    assert!(output.contains("NATIVE_COPY\tb\ta\tCounter"), "{output}");
}

#[test]
fn borrow_keeps_the_owner_usable_but_locked() {
    let output = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
}

init {
    now into a = Counter();
    now into view = a.borrow();
    println(a.value);
    println(view.value);
}
"#,
    )
    .unwrap();
    assert!(output.contains("NATIVE_ALIAS\tview\ta"), "{output}");
    let error = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
}

init {
    now into a = Counter();
    now into view = a.borrow();
    a.destroy();
}
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("`a` is borrowed"), "{error}");
    let error = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
}

init {
    now into a = Counter();
    now into view = a.borrow();
    now into b = a.move();
}
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("`a` is borrowed"), "{error}");
}

#[test]
fn returning_a_borrow_releases_the_owner() {
    let output = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
}

init {
    now into a = Counter();
    now into view = a.borrow();
    view.destroy();
    a.destroy();
}
"#,
    )
    .unwrap();
    assert!(output.contains("NATIVE_ALIAS\tview\ta"), "{output}");
}

#[test]
fn a_returned_borrow_cannot_be_used_again() {
    let error = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
}

init {
    now into a = Counter();
    now into view = a.borrow();
    view.destroy();
    println(view.value);
}
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("`view` was destroyed"), "{error}");
}

#[test]
fn several_borrows_can_coexist() {
    let error = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
}

init {
    now into a = Counter();
    now into one = a.borrow();
    now into two = a.borrow();
    one.destroy();
    a.destroy();
}
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("`a` is borrowed"), "{error}");
    let output = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
}

init {
    now into a = Counter();
    now into one = a.borrow();
    now into two = a.borrow();
    one.destroy();
    two.destroy();
    a.destroy();
}
"#,
    )
    .unwrap();
    assert!(output.contains("NATIVE_ALIAS\ttwo\ta"), "{output}");
}

#[test]
fn an_overriding_destroy_still_marks_the_instance_destroyed() {
    let error = eval(
        r#"
import "intolang://builtin/std";

pub Resource() {
    pub into: Int32 id = 1;
    pub override fun destroy(): Void { println("released"); }
}
init {
    now into r = Resource();
    r.destroy();
    r.destroy();
}
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("`r` was destroyed"), "{error}");
    let text = eval(
        r#"
import "intolang://builtin/std";

pub Resource() {
    pub into: Int32 id = 1;
    pub override fun destroy(): Void { println("released"); }
}
init {
    now into r = Resource();
    r.destroy();
}
"#,
    )
    .unwrap()
    .text();
    assert_eq!(
        text,
        r#"released
"#
    );
}
