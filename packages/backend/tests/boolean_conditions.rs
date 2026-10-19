use testing::eval;

#[test]
fn a_boolean_variable_is_compared_with_a_literal() {
    let output = eval(
        r#"
import "intolang://builtin/std";

init {
    into flag: Boolean = true;
    if (flag == false) {
        println(1);
    }
}
"#,
    )
    .unwrap();
    assert!(
        output.contains("NATIVE_IF_BEGIN\tflag\t==\tn:0\tBoolean"),
        "{output}"
    );
}

#[test]
fn a_literal_on_the_left_is_mirrored() {
    let output = eval(
        r#"
import "intolang://builtin/std";

init {
    into count: Int32 = 1;
    if (3 > count) {
        println(1);
    }
}
"#,
    )
    .unwrap();
    assert!(
        output.contains("NATIVE_IF_BEGIN\tcount\t<\tn:3\tInt32"),
        "{output}"
    );
}

#[test]
fn comparisons_between_literals_are_decided_while_lowering() {
    let output = eval(
        r#"
import "intolang://builtin/std";

pub fun check(left: Int32, right: Int32) {
    if (left != right) {
        println(1);
    }
}
init {
    check(3, 3);
    check(3, 4);
}
"#,
    )
    .unwrap();
    assert!(output.contains("NATIVE_IF_CONST\t0"), "{output}");
    assert!(output.contains("NATIVE_IF_CONST\t1"), "{output}");
}
