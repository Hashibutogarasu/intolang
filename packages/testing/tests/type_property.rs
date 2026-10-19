#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::{eval_run, eval_vm_bundled};

const PROGRAM: &str = "import \"intolang://builtin/std\";

pub Counter() {
    pub into value = 7;
    pub into label = \"counter\";
}

init {
    into n: Int32 = 5;
    println(n.type);
    into s: String = \"x\";
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
    into joined = \"is \" + described.name;
    println(joined);
}
";

const EXPECTED: &str =
    "Int32\nString\nFloat64\nCounter\nInt32\nString\nInt32\nInt32\nList<Int32>\nis Int32\n";

#[test]
fn a_value_gives_the_type_it_has() {
    let output = eval_vm_bundled(PROGRAM, VmOptions::default()).unwrap();
    assert_eq!(output.stdout, EXPECTED);
}

#[test]
fn compiled_code_describes_types_like_the_vm() {
    let compiled = eval_run(PROGRAM).unwrap();
    let virtual_run = eval_vm_bundled(PROGRAM, VmOptions::default()).unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(compiled.stdout, virtual_run.stdout);
    assert_eq!(compiled.stdout, EXPECTED);
}

#[test]
fn an_instance_prints_through_its_to_string() {
    let program = "import \"intolang://builtin/std\";

pub Named() {
    pub into name = \"somebody\";

    pub override fun toString(): String {
        return \"Named(\" + this.name + \")\";
    }
}

init {
    into named = Named();
    println(named);
    named.name = \"other\";
    println(named);
}
";
    let output = eval_vm_bundled(program, VmOptions::default()).unwrap();
    assert_eq!(output.stdout, "Named(somebody)\nNamed(other)\n");
}

#[test]
fn a_class_with_its_own_type_field_keeps_it() {
    let program = "import \"intolang://builtin/std\";

pub Tagged() {
    pub into type = \"custom\";
}

init {
    into tagged = Tagged();
    println(tagged.type);
}
";
    let output = eval_vm_bundled(program, VmOptions::default()).unwrap();
    assert_eq!(output.stdout, "custom\n");
}
