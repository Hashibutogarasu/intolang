#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::{eval_run, eval_vm_bundled};

#[test]
fn a_member_that_is_not_declared_is_read_with_get() {
    let source = r#"
import "intolang://builtin/std";

#[dynamic]
pub Dict() {
    pub into title = "root";

    pub fun get(key: String): Dict {
        into child = Dict();
        child.title = key;
        return child;
    }
}

pub Plain() {
    pub into title = "plain";
}

init {
    into d = Dict();
    println(d.title);
    println(d.color.title);
    println(d.a.b.title);
    into inner = d.size;
    println(inner.title);
    into plain = Plain();
    println(plain.title);
}
"#;
    let expected = r#"root
color
b
size
plain
"#;
    let virtual_run = eval_vm_bundled(source, VmOptions::default()).unwrap();
    assert_eq!(virtual_run.stdout, expected);
    let compiled = eval_run(source).unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(compiled.stdout, expected);
}

#[test]
fn a_class_that_is_not_dynamic_rejects_unknown_members() {
    let message = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

pub Plain() {
    pub into title = "plain";
}

init {
    into plain = Plain();
    println(plain.missing.title);
}
"#,
        VmOptions::default(),
    )
    .unwrap_err()
    .to_string();
    assert!(
        message.contains("Undefined variable `plain.missing.title`"),
        "{message}"
    );
}

#[test]
fn a_subclass_of_a_dynamic_class_is_dynamic_too() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

#[dynamic]
pub Dict() {
    pub into title = "root";

    pub fun get(key: String): Dict {
        into child = Dict();
        child.title = key;
        return child;
    }
}

pub Config(): Dict {
    pub into version = 2;
}

init {
    into config = Config();
    println(config.version);
    println(config.name.title);
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"2
name
"#
    );
}
