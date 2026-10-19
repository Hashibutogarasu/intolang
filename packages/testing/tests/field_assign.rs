#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::eval_vm_bundled;

#[test]
fn a_number_field_takes_a_new_value() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

pub Record() {
    pub into: Int32 count = 7;
    pub into: Float64 ratio = 0.5;
    pub into: String label = "none";
    pub into: Boolean ready = false;
    pub into: Char mark = 'a';

    pub fun reset() {
        this.count = 0;
    }
}

init {
    into r = Record();
    println(r.count);
    r.count = 9;
    println(r.count);
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"7
9
"#
    );
}

#[test]
fn a_field_takes_the_value_of_a_variable() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

pub Record() {
    pub into: Int32 count = 7;
}

init {
    into r = Record();
    into next: Int32 = 41;
    r.count = next;
    println(r.count);
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"41
"#
    );
}

#[test]
fn a_field_converts_a_narrower_number() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

pub Record() {
    pub into: Float64 ratio = 0.5;
}

init {
    into r = Record();
    into small: Int32 = 2;
    r.ratio = small;
    println(r.ratio);
    r.ratio = 1.5;
    println(r.ratio);
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"2.000000
1.500000
"#
    );
}

#[test]
fn a_text_field_takes_known_and_run_time_text() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

pub Record() {
    pub into: String label = "none";
}

init {
    into r = Record();
    println(r.label);
    r.label = "fixed";
    println(r.label);
    into word: String = "abc";
    into up = word.toUpper();
    r.label = up + "!";
    println(r.label);
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"none
fixed
ABC!
"#
    );
}

#[test]
fn boolean_and_char_fields_take_values() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

pub Record() {
    pub into: Boolean ready = false;
    pub into: Char mark = 'a';
}

init {
    into r = Record();
    println(r.ready);
    r.ready = true;
    println(r.ready);
    println(r.mark);
    r.mark = 'z';
    println(r.mark);
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"0
1
a
z
"#
    );
}

#[test]
fn a_method_assigns_the_field_of_its_receiver() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

pub Record() {
    pub into: Int32 count = 7;

    pub fun reset() {
        this.count = 0;
    }
}

init {
    into r = Record();
    r.reset();
    println(r.count);
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"0
"#
    );
}

#[test]
fn a_field_can_be_read_in_arithmetic_and_conditions() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

pub Record() {
    pub into: Int32 count = 7;
    pub into: Boolean ready = false;
}

init {
    into r = Record();
    into total: Int32 = r.count + 3;
    println(total);
    if (r.count == 7) {
        println("seven");
    }
    r.ready = true;
    if (r.ready == true) {
        println("ready");
    }
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"10
seven
ready
"#
    );
}

#[test]
fn an_unknown_field_or_name_is_an_error() {
    let message = format!(
        "{:#}",
        eval_vm_bundled(
            r#"
import "intolang://builtin/std";

pub Record() {
    pub into: Int32 count = 7;
}

init {
    into r = Record();
    r.missing = 1;
}
"#,
            VmOptions::default(),
        )
        .unwrap_err()
    );
    assert!(
        message.contains("`Record` has no field `missing`"),
        "{message}"
    );
    let message = format!(
        "{:#}",
        eval_vm_bundled(
            r#"
import "intolang://builtin/std";

init {
    nothing = 1;
}
"#,
            VmOptions::default(),
        )
        .unwrap_err()
    );
    assert!(message.contains("Undefined name `nothing`"), "{message}");
}

#[test]
fn a_value_of_the_wrong_type_is_an_error() {
    let message = format!(
        "{:#}",
        eval_vm_bundled(
            r#"
import "intolang://builtin/std";

pub Record() {
    pub into: Int32 count = 7;
}

init {
    into r = Record();
    r.count = "text";
}
"#,
            VmOptions::default(),
        )
        .unwrap_err()
    );
    assert!(
        message.contains("The field `count` is a `Int32`"),
        "{message}"
    );
    let message = format!(
        "{:#}",
        eval_vm_bundled(
            r#"
import "intolang://builtin/std";

pub Record() {
    pub into: String label = "none";
}

init {
    into r = Record();
    r.label = 1;
}
"#,
            VmOptions::default(),
        )
        .unwrap_err()
    );
    assert!(
        message.contains("The field `label` is a `String`"),
        "{message}"
    );
    let message = format!(
        "{:#}",
        eval_vm_bundled(
            r#"
import "intolang://builtin/std";

pub Record() {
    pub into: Float64 ratio = 0.5;
}

init {
    into r = Record();
    r.ratio = 'a';
}
"#,
            VmOptions::default(),
        )
        .unwrap_err()
    );
    assert!(
        message.contains("The field `ratio` is a `Float64`"),
        "{message}"
    );
}
