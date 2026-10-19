#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::eval_vm_bundled;

#[test]
fn a_generic_class_holds_a_value_of_its_argument() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

pub Box<T>() {
    pub into: T content;
}

pub Named<T>(): Box<T> {
    pub into: String label = "unnamed";
}

init {
    into b: Box<String> = Box();
    b.content = "inside";
    println(b.content);
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"inside
"#
    );
}

#[test]
fn a_subclass_keeps_the_fields_and_the_arguments() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

pub Box<T>() {
    pub into: T content;
}

pub Named<T>(): Box<T> {
    pub into: String label = "unnamed";
}

init {
    into n: Named<String> = Named();
    n.content = "value";
    println(n.label);
    println(n.content);
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"unnamed
value
"#
    );
}

#[test]
fn the_argument_decides_the_type_of_the_field() {
    let message = format!(
        "{:#}",
        eval_vm_bundled(
            r#"
import "intolang://builtin/std";

pub Box<T>() {
    pub into: T content;
}

init {
    into b: Box<String> = Box();
    b.content = 5;
}
"#,
            VmOptions::default(),
        )
        .unwrap_err()
    );
    assert!(
        message.contains("The field `content` is a `String`"),
        "{message}"
    );
}

#[test]
fn the_same_class_with_other_arguments_is_another_type() {
    let message = format!(
        "{:#}",
        eval_vm_bundled(
            r#"
import "intolang://builtin/std";

pub Box<T>() {
    pub into: T content;
}

init {
    into a: Box<String> = Box();
    into b: Box<Int32> = a;
}
"#,
            VmOptions::default(),
        )
        .unwrap_err()
    );
    assert!(message.contains("Type mismatch"), "{message}");
}
