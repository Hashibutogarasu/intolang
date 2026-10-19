use intovm::TraceEntry;
use testing::eval_vm;

fn destroyed(name: &str) -> TraceEntry {
    TraceEntry::Destroyed(name.to_owned())
}

#[test]
fn compile_time_text_and_instance_fields_are_printed() {
    let outcome = eval_vm(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
}

init {
    println("start");
    now into a = Counter();
    println(a.value);
}
"#,
    )
    .unwrap();
    assert_eq!(
        outcome.stdout,
        r#"start
7
"#
    );
}

#[test]
fn a_scope_end_destroys_in_reverse_declaration_order() {
    let outcome = eval_vm(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
}

init {
    now into a = Counter();
    now into b = Counter();
}
"#,
    )
    .unwrap();
    assert_eq!(outcome.trace, vec![destroyed("b"), destroyed("a")]);
}

#[test]
fn an_explicit_destroy_is_not_repeated_at_the_scope_end() {
    let outcome = eval_vm(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
}

init {
    now into a = Counter();
    now into b = Counter();
    a.destroy();
}
"#,
    )
    .unwrap();
    assert_eq!(outcome.trace, vec![destroyed("a"), destroyed("b")]);
}

#[test]
fn a_moved_instance_is_destroyed_once_by_its_new_owner() {
    let outcome = eval_vm(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
}

init {
    now into a = Counter();
    now into b = a.move();
}
"#,
    )
    .unwrap();
    assert_eq!(outcome.trace, vec![destroyed("b")]);
}

#[test]
fn a_move_in_a_nested_scope_is_decided_by_the_drop_flag() {
    let outcome = eval_vm(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
}

init {
    now into a = Counter();
    now into holder = Counter();
    into flag: Int32 = 1;
    if (flag == 1) {
        now into taken = a.move();
    }
}
"#,
    )
    .unwrap();
    assert_eq!(outcome.trace, vec![destroyed("taken"), destroyed("holder")]);
}

#[test]
fn a_borrow_is_given_back_without_destroying_the_instance() {
    let outcome = eval_vm(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
}

init {
    now into a = Counter();
    now into view = a.borrow();
}
"#,
    )
    .unwrap();
    assert_eq!(outcome.trace, vec![destroyed("view"), destroyed("a")]);
}

#[test]
fn lowering_errors_are_reported_before_anything_runs() {
    let error = eval_vm(
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
    .unwrap_err();
    assert!(error.to_string().contains("`a` was destroyed"), "{error}");
}
