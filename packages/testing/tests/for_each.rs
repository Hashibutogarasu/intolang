#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::{eval_run, eval_vm_bundled};

#[test]
fn for_each_hands_every_item_to_the_lambda() {
    let source = r#"
import "intolang://builtin/std";

pub Item() {
    pub into name = "";
}

pub fun show(value: Int32) {
    println(value);
}

init {
    into numbers: [Int32] = [10, 20, 30];
    numbers.forEach((n) => {
        println(n);
    });
    numbers.forEach((n, i) => {
        println(n + i);
    });
    into total: Int32 = 0;
    numbers.forEach((n) => {
        total = total + n;
    });
    println(total);
    numbers.forEach(show);
    into names: [String] = ["a", "b"];
    names.forEach((name) => {
        println(name + "!");
    });
    into first = Item();
    first.name = "one";
    into second = Item();
    second.name = "two";
    into items: [Item] = [];
    items.push(first);
    items.push(second);
    items.forEach((item: Item) => {
        println(item.name);
    });
    into empty: [Int32] = [];
    empty.forEach((n) => {
        println("never");
    });
}
"#;
    let expected = r#"10
20
30
10
21
32
60
10
20
30
a!
b!
one
two
"#;
    let virtual_run = eval_vm_bundled(source, VmOptions::default()).unwrap();
    assert_eq!(virtual_run.stdout, expected);
    let compiled = eval_run(source).unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(compiled.stdout, expected);
}

#[test]
fn the_lambda_must_fit_the_items() {
    let message = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

init {
    into numbers: [Int32] = [1];
    numbers.forEach((n: String) => {
        println(n);
    });
}
"#,
        VmOptions::default(),
    )
    .unwrap_err()
    .to_string();
    assert!(
        message.contains("The parameter `n` is a `String`"),
        "{message}"
    );
    let message = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

init {
    into numbers: [Int32] = [1];
    numbers.forEach((a, b, c) => {
        println(a);
    });
}
"#,
        VmOptions::default(),
    )
    .unwrap_err()
    .to_string();
    assert!(message.contains("not 3 parameter(s)"), "{message}");
}
