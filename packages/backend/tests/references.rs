use testing::eval;

#[test]
fn a_reference_parameter_borrows_the_argument_for_the_call() {
    let text = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
    pub fun peek(): Void { println("peek"); }
    #[consumes]
    pub fun finish(): Void { println("finished"); }
}
pub fun look(target: Counter&) { target.peek(); }
now into a = Counter();
look(a);
a.peek();
"#,
    )
    .unwrap()
    .text();
    assert_eq!(
        text,
        r#"peek
peek
"#
    );
}

#[test]
fn a_reference_cannot_call_a_consuming_method() {
    let error = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
    pub fun peek(): Void { println("peek"); }
    #[consumes]
    pub fun finish(): Void { println("finished"); }
}
pub fun release(target: Counter&) { target.finish(); }
now into a = Counter();
release(a);
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("requires ownership"), "{error}");
}

#[test]
fn a_reference_cannot_move_the_instance() {
    let error = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
    pub fun peek(): Void { println("peek"); }
    #[consumes]
    pub fun finish(): Void { println("finished"); }
}
pub fun release(target: Counter&) { target.move(); }
now into a = Counter();
release(a);
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("requires ownership"), "{error}");
}

#[test]
fn the_owner_is_usable_again_after_the_call() {
    let text = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
    pub fun peek(): Void { println("peek"); }
    #[consumes]
    pub fun finish(): Void { println("finished"); }
}
pub fun look(target: Counter&) { target.peek(); }
now into a = Counter();
look(a);
a.finish();
"#,
    )
    .unwrap()
    .text();
    assert_eq!(
        text,
        r#"peek
finished
"#
    );
}

#[test]
fn a_consuming_method_leaves_the_owner_moved() {
    let error = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
    pub fun peek(): Void { println("peek"); }
    #[consumes]
    pub fun finish(): Void { println("finished"); }
}
now into a = Counter();
a.finish();
a.peek();
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("`a` was moved"), "{error}");
}

#[test]
fn a_detaching_method_leaves_the_owner_detached() {
    let error = eval(
        r#"
import "intolang://builtin/std";

pub Job() {
    pub into: Int32 id = 1;
    pub fun wait(): Void { println("waited"); }
    #[consumes("detached")]
    pub fun detach(): Void { println("detached"); }
}
now into job = Job();
job.detach();
job.wait();
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("`job` was detached"), "{error}");
}

#[test]
fn borrow_produces_a_reference_that_cannot_consume() {
    let error = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
    pub fun peek(): Void { println("peek"); }
    #[consumes]
    pub fun finish(): Void { println("finished"); }
}
now into a = Counter();
now into view = a.borrow();
view.finish();
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("requires ownership"), "{error}");
    let text = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
    pub fun peek(): Void { println("peek"); }
    #[consumes]
    pub fun finish(): Void { println("finished"); }
}
now into a = Counter();
now into view = a.borrow();
view.peek();
"#,
    )
    .unwrap()
    .text();
    assert_eq!(
        text,
        r#"peek
"#
    );
}

#[test]
fn a_reference_cannot_be_bound_to_an_owning_annotation() {
    let error = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
    pub fun peek(): Void { println("peek"); }
    #[consumes]
    pub fun finish(): Void { println("finished"); }
}
now into a = Counter();
now into view: Counter = a.borrow();
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("Counter"), "{error}");
}

#[test]
fn a_reference_cannot_be_passed_where_a_value_is_expected() {
    let error = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
    pub fun peek(): Void { println("peek"); }
    #[consumes]
    pub fun finish(): Void { println("finished"); }
}
pub fun take(target: Counter) { target.peek(); }
now into a = Counter();
now into view = a.borrow();
take(view);
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("is a reference"), "{error}");
}

#[test]
fn a_reference_annotation_borrows_the_value() {
    let text = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
    pub fun peek(): Void { println("peek"); }
    #[consumes]
    pub fun finish(): Void { println("finished"); }
}
now into a = Counter();
now into view: Counter& = a;
view.peek();
"#,
    )
    .unwrap()
    .text();
    assert_eq!(
        text,
        r#"peek
"#
    );
    let error = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
    pub fun peek(): Void { println("peek"); }
    #[consumes]
    pub fun finish(): Void { println("finished"); }
}
now into a = Counter();
now into view: Counter& = a;
a.finish();
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(
        error.contains("`a` was moved") || error.contains("is borrowed"),
        "{error}"
    );
}

#[test]
fn a_function_may_return_a_reference_to_its_argument() {
    let text = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
    pub fun peek(): Void { println("peek"); }
    #[consumes]
    pub fun finish(): Void { println("finished"); }
}
pub fun pick(target: Counter&): Counter& { return target; }
now into a = Counter();
now into found = pick(a);
found.peek();
"#,
    )
    .unwrap()
    .text();
    assert_eq!(
        text,
        r#"peek
"#
    );
}

#[test]
fn the_result_of_a_reference_function_owns_nothing() {
    let error = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
    pub fun peek(): Void { println("peek"); }
    #[consumes]
    pub fun finish(): Void { println("finished"); }
}
pub fun pick(target: Counter&): Counter& { return target; }
now into a = Counter();
now into found = pick(a);
found.finish();
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("requires ownership"), "{error}");
    let error = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
    pub fun peek(): Void { println("peek"); }
    #[consumes]
    pub fun finish(): Void { println("finished"); }
}
pub fun pick(target: Counter&): Counter& { return target; }
now into a = Counter();
now into found: Counter = pick(a);
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("Counter"), "{error}");
    let error = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
    pub fun peek(): Void { println("peek"); }
    #[consumes]
    pub fun finish(): Void { println("finished"); }
}
pub fun pick(target: Counter&): Counter& { return target; }
now into a = Counter();
now into found = pick(a);
a.finish();
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("is borrowed"), "{error}");
}

#[test]
fn a_function_cannot_return_a_reference_to_a_local() {
    let error = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
    pub fun peek(): Void { println("peek"); }
    #[consumes]
    pub fun finish(): Void { println("finished"); }
}
pub fun local(): Counter& { now into inner = Counter(); return inner; }
now into found = local();
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("returns a reference to a local"), "{error}");
}

#[test]
fn moving_an_argument_hands_the_instance_to_the_function() {
    let error = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
    pub fun peek(): Void { println("peek"); }
    #[consumes]
    pub fun finish(): Void { println("finished"); }
}
pub fun consume(owned: Counter) { owned.finish(); }
now into a = Counter();
consume(a.move());
a.peek();
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("`a` was moved"), "{error}");
    let text = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
    pub fun peek(): Void { println("peek"); }
    #[consumes]
    pub fun finish(): Void { println("finished"); }
}
pub fun consume(owned: Counter) { owned.finish(); }
now into a = Counter();
consume(a.move());
"#,
    )
    .unwrap()
    .text();
    assert_eq!(
        text,
        r#"finished
"#
    );
}

#[test]
fn copying_an_argument_keeps_the_original() {
    let text = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
    pub fun peek(): Void { println("peek"); }
    #[consumes]
    pub fun finish(): Void { println("finished"); }
}
pub fun consume(owned: Counter) { owned.finish(); }
now into a = Counter();
consume(a.copy());
a.peek();
"#,
    )
    .unwrap()
    .text();
    assert_eq!(
        text,
        r#"finished
peek
"#
    );
}

#[test]
fn a_borrowed_argument_is_given_back_after_the_call() {
    let text = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
    pub fun peek(): Void { println("peek"); }
    #[consumes]
    pub fun finish(): Void { println("finished"); }
}
pub fun look(target: Counter&) { target.peek(); }
now into a = Counter();
look(a.borrow());
a.finish();
"#,
    )
    .unwrap()
    .text();
    assert_eq!(
        text,
        r#"peek
finished
"#
    );
}
