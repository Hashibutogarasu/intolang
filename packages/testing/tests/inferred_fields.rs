#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::{eval_run, eval_vm_bundled};

const PROGRAM: &str = "import \"intolang://builtin/std\";

pub Base() {
}

pub Cat(): Base {
    pub into neko = \"cat\";
    pub into lives = 9;
}

pub Dog(): Base {
    pub into foo = \"bar\";
    pub into cat = Cat();
    pub into age = 3;
}

init {
    into dog = Dog();
    println(dog.foo);
    println(dog.age);
    println(dog.cat.neko);
    println(dog.cat.lives);
    dog.age = 4;
    println(dog.age);
    into first = Dog();
    into second = Dog();
    first.foo = \"first\";
    second.foo = \"second\";
    println(first.foo);
    println(second.foo);
    println(first.cat.neko);
}
";

const EXPECTED: &str = "bar\n3\ncat\n9\n4\nfirst\nsecond\ncat\n";

#[test]
fn fields_with_inferred_types_start_with_their_values() {
    let output = eval_vm_bundled(PROGRAM, VmOptions::default()).unwrap();
    assert_eq!(output.stdout, EXPECTED);
}

#[test]
fn compiled_code_builds_the_nested_instances_like_the_vm() {
    let compiled = eval_run(PROGRAM).unwrap();
    let virtual_run = eval_vm_bundled(PROGRAM, VmOptions::default()).unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(compiled.stdout, virtual_run.stdout);
    assert_eq!(compiled.stdout, EXPECTED);
}

#[test]
fn every_construction_gets_its_own_storage() {
    let program = "import \"intolang://builtin/std\";

pub Item() {
    pub into name = \"\";
}

init {
    into items: [Item] = [];
    into i: Int32 = 0;
    while (i < 3) {
        into item = Item();
        item.name = \"x\" + 'a';
        items.push(item);
        i = i + 1;
    }
    println(items.length());
}
";
    let compiled = eval_run(program).unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(compiled.stdout, "3\n");
}
