#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::{eval_run, eval_vm_bundled};

const PROGRAM: &str = "import \"intolang://builtin/std\";

#[dynamic]
pub Dict() {
    pub into title = \"root\";

    pub fun get(key: String): Dict {
        into child = Dict();
        child.title = key;
        return child;
    }
}

pub Plain() {
    pub into title = \"plain\";
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
";

const EXPECTED: &str = "root\ncolor\nb\nsize\nplain\n";

#[test]
fn a_member_that_is_not_declared_is_read_with_get() {
    let output = eval_vm_bundled(PROGRAM, VmOptions::default()).unwrap();
    assert_eq!(output.stdout, EXPECTED);
}

#[test]
fn compiled_code_reads_dynamic_members_like_the_vm() {
    let compiled = eval_run(PROGRAM).unwrap();
    let virtual_run = eval_vm_bundled(PROGRAM, VmOptions::default()).unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(compiled.stdout, virtual_run.stdout);
    assert_eq!(compiled.stdout, EXPECTED);
}

#[test]
fn a_class_that_is_not_dynamic_rejects_unknown_members() {
    let program = "import \"intolang://builtin/std\";

pub Plain() {
    pub into title = \"plain\";
}

init {
    into plain = Plain();
    println(plain.missing.title);
}
";
    let message = eval_vm_bundled(program, VmOptions::default())
        .unwrap_err()
        .to_string();
    assert!(
        message.contains("Undefined variable `plain.missing.title`"),
        "{message}"
    );
}

#[test]
fn a_subclass_of_a_dynamic_class_is_dynamic_too() {
    let program = "import \"intolang://builtin/std\";

#[dynamic]
pub Dict() {
    pub into title = \"root\";

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
";
    let output = eval_vm_bundled(program, VmOptions::default()).unwrap();
    assert_eq!(output.stdout, "2\nname\n");
}
