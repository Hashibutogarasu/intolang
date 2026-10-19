use testing::eval_vm;

#[test]
fn a_character_prints_as_text() {
    let output = eval_vm(
        r#"
import "intolang://builtin/std";

init {
    into c: Char = 'A';
    println(c);
    println('z');
}
"#,
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"A
z
"#
    );
}

#[test]
fn a_character_converts_to_its_code() {
    let output = eval_vm(
        r#"
import "intolang://builtin/std";

init {
    into c: Char = 'A';
    into n: Int32 = c as Int32;
    println(n);
}
"#,
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"65
"#
    );
}

#[test]
fn a_code_converts_to_a_character() {
    let output = eval_vm(
        r#"
import "intolang://builtin/std";

init {
    into n: Int32 = 97;
    into c: Char = n as Char;
    println(c);
}
"#,
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"a
"#
    );
}

#[test]
fn a_digit_is_built_from_the_code_of_zero() {
    let output = eval_vm(
        r#"
import "intolang://builtin/std";

init {
    into digit: Int32 = 7;
    into c: Char = (('0' as Int32) + digit) as Char;
    println(c);
}
"#,
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"7
"#
    );
}

#[test]
fn numbers_convert_across_widths_and_kinds() {
    let output = eval_vm(
        r#"
import "intolang://builtin/std";

init {
    into big: Int32 = 300;
    into small: Int8 = big as Int8;
    println(small);
    into f: Float64 = big as Float64;
    into back: Int32 = f as Int32;
    println(back);
    into pi: Float64 = 3.75;
    into whole: Int32 = pi as Int32;
    println(whole);
}
"#,
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"44
300
3
"#
    );
}

#[test]
fn characters_compare_with_characters() {
    let output = eval_vm(
        r#"
import "intolang://builtin/std";

init {
    into c: Char = 'b';
    if (c == 'b') {
        println("same");
    }
    if (c < 'a') {
        println("never");
    } else {
        println("later");
    }
}
"#,
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"same
later
"#
    );
}

#[test]
fn a_character_cannot_take_part_in_arithmetic() {
    let message = format!(
        "{:#}",
        eval_vm(
            r#"
import "intolang://builtin/std";

init {
    into c: Char = 'A';
    into d: Char = c + c;
}
"#,
        )
        .unwrap_err()
    );
    assert!(message.contains("numeric"), "{message}");
}

#[test]
fn a_boolean_converts_to_a_number_but_not_back() {
    let output = eval_vm(
        r#"
import "intolang://builtin/std";

init {
    into yes: Boolean = 1 < 2;
    into n: Int32 = yes as Int32;
    println(n);
}
"#,
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"1
"#
    );
    let message = format!(
        "{:#}",
        eval_vm(
            r#"
import "intolang://builtin/std";

init {
    into n: Int32 = 1;
    into b: Boolean = n as Boolean;
}
"#,
        )
        .unwrap_err()
    );
    assert!(
        message.contains("no cast from Int32 to Boolean"),
        "{message}"
    );
}

#[test]
fn a_string_does_not_convert_to_a_character() {
    let message = format!(
        "{:#}",
        eval_vm(
            r#"
import "intolang://builtin/std";

init {
    into s: String = "A";
    into c: Char = s as Char;
}
"#,
        )
        .unwrap_err()
    );
    assert!(message.contains("no cast from"), "{message}");
}
