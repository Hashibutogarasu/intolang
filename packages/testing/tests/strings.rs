#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::eval_vm_bundled;

#[test]
fn text_known_while_lowering_is_joined_while_lowering() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

init {
    into a: String = "foo";
    into b: String = a + "bar" + '!';
    println(b);
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"foobar!
"#
    );
}

#[test]
fn text_made_at_run_time_can_be_joined_and_printed() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

init {
    into s: String = "abc";
    into up = s.toUpper();
    into joined = up + "-" + s;
    println(joined);
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"ABC-abc
"#
    );
}

#[test]
fn a_character_joins_a_run_time_text() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

init {
    into s: String = "abc";
    into up = s.toUpper();
    into c: Char = 'x';
    into joined = up + c;
    println(joined);
    into front = c + up;
    println(front);
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"ABCx
xABC
"#
    );
}

#[test]
fn a_computed_character_joins_a_text() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

init {
    into s: String = "n";
    into up = s.toUpper();
    into digit: Int32 = 5;
    into joined = up + (('0' as Int32) + digit) as Char;
    println(joined);
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"N5
"#
    );
}

#[test]
fn the_text_methods_answer_for_the_text() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

init {
    into s: String = "Hello, World";
    println(s.length());
    println(s.at(1));
    println(s.subString(7, 12));
    println(s.indexOf("World"));
    println(s.indexOf("nope"));
    println(s.toLower());
    println(s.toUpper());
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"12
e
World
7
-1
hello, world
HELLO, WORLD
"#
    );
}

#[test]
fn prefixes_and_suffixes_are_conditions() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

init {
    into s: String = "Hello, World";
    if (s.startsWith("Hello")) {
        println("starts");
    }
    if (s.endsWith("nope")) {
        println("never");
    } else {
        println("does not end");
    }
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"starts
does not end
"#
    );
}

#[test]
fn texts_compare_for_equality() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

init {
    into s: String = "abc";
    into up = s.toUpper();
    if (up == "ABC") {
        println("same");
    }
    if (up != s) {
        println("different");
    }
    into again = s.toUpper();
    if (up == again) {
        println("equal");
    }
    if (s == "abc") {
        println("constant");
    }
    if (s != "abc") {
        println("never");
    }
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"same
different
equal
constant
"#
    );
}

#[test]
fn a_comparison_result_can_be_stored() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

init {
    into s: String = "abc";
    into up = s.toUpper();
    into same: Boolean = up == "ABC";
    if (same) {
        println("stored");
    }
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"stored
"#
    );
}

#[test]
fn a_loop_can_build_a_text() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

init {
    into s: String = "a";
    into text = s.toUpper();
    into n: Int32 = 0;
    while (n < 3) {
        text = text + s;
        n = n + 1;
    }
    println(text);
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"Aaaa
"#
    );
}

#[test]
fn a_text_changed_by_a_loop_or_a_branch_keeps_the_value_of_the_run() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

init {
    into text: String = "";
    into n: Int32 = 0;
    while (n < 3) {
        text = 'x' + text;
        n = n + 1;
    }
    println(text);
    into label: String = "a";
    if (n == 3) {
        label = "b";
    }
    println(label);
    if (n == 4) {
        label = "c";
    }
    println(label);
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"xxx
b
b
"#
    );
}

#[test]
fn texts_only_compare_with_equality() {
    let message = format!(
        "{:#}",
        eval_vm_bundled(
            r#"
import "intolang://builtin/std";

init {
    into s: String = "abc";
    into up = s.toUpper();
    if (up < "b") {
        println("never");
    }
}
"#,
            VmOptions::default(),
        )
        .unwrap_err()
    );
    assert!(message.contains("`==` and `!=`"), "{message}");
}

#[test]
fn a_text_and_a_number_do_not_join() {
    let message = format!(
        "{:#}",
        eval_vm_bundled(
            r#"
import "intolang://builtin/std";

init {
    into s: String = "abc";
    into n: Int32 = 1;
    into t = s + n;
}
"#,
            VmOptions::default(),
        )
        .unwrap_err()
    );
    assert!(message.contains("Arithmetic"), "{message}");
}
