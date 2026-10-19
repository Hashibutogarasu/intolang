use testing::eval;

#[test]
fn the_program_end_destroys_in_reverse_order() {
    let text = eval(
        r#"
import "intolang://builtin/std";

pub First() {
    pub into: Int32 id = 1;
    pub override fun destroy(): Void { println("first"); }
}
pub Second() {
    pub into: Int32 id = 2;
    pub override fun destroy(): Void { println("second"); }
}
now into a = First();
now into b = Second();
"#,
    )
    .unwrap()
    .text();
    assert_eq!(
        text,
        r#"second
first
"#
    );
}

#[test]
fn init_destroys_its_bindings_when_it_ends() {
    let text = eval(
        r#"
import "intolang://builtin/std";

pub First() {
    pub into: Int32 id = 1;
    pub override fun destroy(): Void { println("first"); }
}
pub Second() {
    pub into: Int32 id = 2;
    pub override fun destroy(): Void { println("second"); }
}
init {
    now into a = First();
    now into b = Second();
}
"#,
    )
    .unwrap()
    .text();
    assert_eq!(
        text,
        r#"second
first
"#
    );
}

#[test]
fn an_if_block_destroys_its_bindings_at_its_end() {
    let text = eval(
        r#"
import "intolang://builtin/std";

pub First() {
    pub into: Int32 id = 1;
    pub override fun destroy(): Void { println("first"); }
}
pub Second() {
    pub into: Int32 id = 2;
    pub override fun destroy(): Void { println("second"); }
}
now into n = 1;
if (n == 1) {
    now into a = First();
}
now into b = Second();
"#,
    )
    .unwrap()
    .text();
    assert_eq!(
        text,
        r#"first
second
"#
    );
}

#[test]
fn a_binding_cannot_be_used_after_its_scope_ended() {
    let error = eval(
        r#"
import "intolang://builtin/std";

pub First() {
    pub into: Int32 id = 1;
    pub override fun destroy(): Void { println("first"); }
}
now into n = 1;
if (n == 1) {
    now into a = First();
}
println(a.id);
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("`a` was destroyed"), "{error}");
}

#[test]
fn an_explicit_destroy_is_not_repeated_at_the_scope_end() {
    let text = eval(
        r#"
import "intolang://builtin/std";

pub First() {
    pub into: Int32 id = 1;
    pub override fun destroy(): Void { println("first"); }
}
now into a = First();
a.destroy();
"#,
    )
    .unwrap()
    .text();
    assert_eq!(
        text,
        r#"first
"#
    );
}

#[test]
fn a_moved_binding_is_destroyed_once_by_its_new_owner() {
    let text = eval(
        r#"
import "intolang://builtin/std";

pub First() {
    pub into: Int32 id = 1;
    pub override fun destroy(): Void { println("first"); }
}
now into a = First();
now into b = a.move();
"#,
    )
    .unwrap()
    .text();
    assert_eq!(
        text,
        r#"first
"#
    );
}

#[test]
fn a_copy_is_destroyed_separately_from_its_source() {
    let text = eval(
        r#"
import "intolang://builtin/std";

pub First() {
    pub into: Int32 id = 1;
    pub override fun destroy(): Void { println("first"); }
}
now into a = First();
now into b = a.copy();
"#,
    )
    .unwrap()
    .text();
    assert_eq!(
        text,
        r#"first
first
"#
    );
}

#[test]
fn a_borrow_is_given_back_without_destroying_the_instance() {
    let text = eval(
        r#"
import "intolang://builtin/std";

pub First() {
    pub into: Int32 id = 1;
    pub override fun destroy(): Void { println("first"); }
}
now into a = First();
now into n = 1;
if (n == 1) {
    now into view = a.borrow();
}
now into b = a.move();
"#,
    )
    .unwrap()
    .text();
    assert_eq!(
        text,
        r#"first
"#
    );
}

#[test]
fn an_alias_does_not_destroy_the_instance_twice() {
    let text = eval(
        r#"
import "intolang://builtin/std";

pub First() {
    pub into: Int32 id = 1;
    pub override fun destroy(): Void { println("first"); }
}
now into a = First();
now into b = a;
"#,
    )
    .unwrap()
    .text();
    assert_eq!(
        text,
        r#"first
"#
    );
}

#[test]
fn break_destroys_what_the_loop_body_owns() {
    let output = eval(
        r#"
import "intolang://builtin/std";

pub First() {
    pub into: Int32 id = 1;
    pub override fun destroy(): Void { println("first"); }
}
now into n = 0;
while (n < 2) {
    now into a = First();
    break;
}
"#,
    )
    .unwrap();
    assert_eq!(
        output.text(),
        r#"first
first
"#,
        "{output}"
    );
}

#[test]
fn a_move_in_a_nested_scope_is_decided_by_a_drop_flag() {
    let output = eval(
        r#"
import "intolang://builtin/std";

pub First() {
    pub into: Int32 id = 1;
    pub override fun destroy(): Void { println("first"); }
}
now into a = First();
now into n = 1;
if (n == 1) {
    now into b = a.move();
}
"#,
    )
    .unwrap();
    assert!(output.contains("__drop_a"), "{output}");
}
