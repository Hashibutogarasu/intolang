#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::{eval_run, eval_vm_bundled};

#[test]
fn a_value_gives_the_type_it_has() {
    let source = r#"
import "intolang://builtin/std";

pub Counter() {
    pub into value = 7;
    pub into label = "counter";
}

init {
    into n: Int32 = 5;
    println(n.type);
    into s: String = "x";
    println(s.type);
    into f: Float64 = 1.5;
    println(f.type);
    into c = Counter();
    println(c.type);
    println(c.value.type);
    println(c.label.type);
    into described = n.type;
    println(described);
    println(described.name);
    into numbers: [Int32] = [1];
    println(numbers.type);
    into joined = "is " + described.name;
    println(joined);
}
"#;
    let expected = r#"Int32
String
Float64
Counter
Int32
String
Int32
Int32
List<Int32>
is Int32
"#;
    let virtual_run = eval_vm_bundled(source, VmOptions::default()).unwrap();
    assert_eq!(virtual_run.stdout, expected);
    let compiled = eval_run(source).unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(compiled.stdout, expected);
}

#[test]
fn an_instance_prints_through_its_to_string() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

pub Named() {
    pub into name = "somebody";

    pub override fun toString(): String {
        return "Named(" + this.name + ")";
    }
}

init {
    into named = Named();
    println(named);
    named.name = "other";
    println(named);
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"Named(somebody)
Named(other)
"#
    );
}

#[test]
fn a_class_with_its_own_type_field_keeps_it() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

pub Tagged() {
    pub into type = "custom";
}

init {
    into tagged = Tagged();
    println(tagged.type);
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"custom
"#
    );
}
