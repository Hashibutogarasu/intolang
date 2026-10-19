use testing::eval_vm;

#[test]
fn arithmetic_follows_the_precedence_of_the_operators() {
    let output = eval_vm(
        r#"
import "intolang://builtin/std";

init {
    into a: Int32 = 7;
    into b: Int32 = a % 4 + 2 * 3;
    println(b);
    into c: Int32 = (a & 3) | 8;
    println(c);
    into d: Int32 = a << 2;
    println(d);
    into e: Int32 = d >> 1 ^ 5;
    println(e);
}
"#,
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"9
11
28
11
"#
    );
}

#[test]
fn a_nested_expression_is_computed_in_temporaries() {
    let output = eval_vm(
        r#"
import "intolang://builtin/std";

init {
    into x: Int32 = 2;
    into y: Int32 = 5;
    into z: Int32 = (x + y) * (y - x) - x;
    println(z);
}
"#,
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"19
"#
    );
}

#[test]
fn logical_operators_combine_comparisons() {
    let output = eval_vm(
        r#"
import "intolang://builtin/std";

init {
    into a: Int32 = 7;
    into b: Int32 = 9;
    if (a > 3 && b < 10) {
        println("both");
    }
    if (a > 100 || b == 9) {
        println("either");
    }
    if (!(a == 7)) {
        println("never");
    } else {
        println("not");
    }
}
"#,
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"both
either
not
"#
    );
}

#[test]
fn the_right_side_of_a_logical_operator_is_skipped_when_the_left_decides() {
    let output = eval_vm(
        r#"
import "intolang://builtin/std";

init {
    into a: Int32 = 7;
    into zero: Int32 = 0;
    if (a > 100 && 10 / zero > 1) {
        println("never");
    } else {
        println("skipped");
    }
    if (a < 100 || 10 / zero > 1) {
        println("short");
    }
}
"#,
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"skipped
short
"#
    );
}

#[test]
fn a_loop_computes_its_compound_condition_again_every_time() {
    let output = eval_vm(
        r#"
import "intolang://builtin/std";

init {
    into n: Int32 = 0;
    while (n < 10 && n * n < 30) {
        n = n + 1;
    }
    println(n);
}
"#,
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"6
"#
    );
}

#[test]
fn a_comparison_can_be_stored_in_a_boolean() {
    let output = eval_vm(
        r#"
import "intolang://builtin/std";

init {
    into a: Int32 = 7;
    into big: Boolean = a > 3;
    into small: Boolean = a < 3 || a == 7;
    if (big && small) {
        println("yes");
    }
    if (!big) {
        println("no");
    }
}
"#,
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"yes
"#
    );
}

#[test]
fn a_boolean_variable_is_a_condition() {
    let output = eval_vm(
        r#"
import "intolang://builtin/std";

init {
    into ready: Boolean = true;
    if (ready) {
        println("ready");
    }
    if (!ready) {
        println("waiting");
    }
}
"#,
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"ready
"#
    );
}

#[test]
fn bit_operators_need_integers() {
    let error = eval_vm(
        r#"
import "intolang://builtin/std";

init {
    into a: Float64 = 2.5;
    into b: Float64 = a % 2.0;
}
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("needs integers"), "{error}");
}

#[test]
fn operands_of_different_types_are_rejected() {
    let error = eval_vm(
        r#"
import "intolang://builtin/std";

init {
    into a: Int32 = 1;
    into b: Int64 = 2;
    into c: Int32 = a + b;
}
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("does not match operand type"), "{error}");
}

#[test]
fn division_by_zero_is_an_error() {
    let error = eval_vm(
        r#"
import "intolang://builtin/std";

init {
    into a: Int32 = 5;
    into zero: Int32 = 0;
    into b: Int32 = a % zero;
}
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("Division by zero"), "{error}");
}
