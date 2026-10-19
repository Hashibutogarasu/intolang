#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::{eval_run, eval_vm_bundled};

#[test]
fn fields_with_inferred_types_start_with_their_values() {
    let source = r#"
import "intolang://builtin/std";

pub Base() {
}

pub Cat(): Base {
    pub into neko = "cat";
    pub into lives = 9;
}

pub Dog(): Base {
    pub into foo = "bar";
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
    first.foo = "first";
    second.foo = "second";
    println(first.foo);
    println(second.foo);
    println(first.cat.neko);
}
"#;
    let expected = r#"bar
3
cat
9
4
first
second
cat
"#;
    let virtual_run = eval_vm_bundled(source, VmOptions::default()).unwrap();
    assert_eq!(virtual_run.stdout, expected);
    let compiled = eval_run(source).unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(compiled.stdout, expected);
}

#[test]
fn every_construction_gets_its_own_storage() {
    let compiled = eval_run(
        r#"
import "intolang://builtin/std";

pub Item() {
    pub into name = "";
}

init {
    into items: [Item] = [];
    into i: Int32 = 0;
    while (i < 3) {
        into item = Item();
        item.name = "x" + 'a';
        items.push(item);
        i = i + 1;
    }
    println(items.length());
}
"#,
    )
    .unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(
        compiled.stdout,
        r#"3
"#
    );
}
