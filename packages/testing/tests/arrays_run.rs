#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::{eval_run, eval_vm_bundled};

#[test]
fn compiled_code_keeps_lists_like_the_vm() {
    let source = r#"
import "intolang://builtin/std";

pub Item() {
    pub into: String name = "";
}

init {
    into xs: [Int32] = [10, 20, 30];
    xs.push(40);
    xs.set(0, 11);
    into total: Int32 = 0;
    into i: Int32 = 0;
    while (i < xs.length()) {
        total = total + xs.get(i);
        i = i + 1;
    }
    println(total);
    into ratios = [1.5, 2.5];
    println(ratios.get(1));
    into names: [String] = [];
    names.push("first");
    names.push("second" + "!");
    println(names.get(1));
    into marks: [Char] = ['a', 'b'];
    println(marks.get(1));
    into big: [Int64] = [5000000000];
    println(big.get(0));
    into first = Item();
    first.name = "one";
    into items: [Item] = [];
    items.push(first);
    into again = items.get(0);
    println(again.name);
}
"#;
    let compiled = eval_run(source).unwrap();
    let virtual_run = eval_vm_bundled(source, VmOptions::default()).unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(compiled.stdout, virtual_run.stdout);
    assert_eq!(
        compiled.stdout,
        r#"101
2.500000
second!
b
5000000000
one
"#
    );
}
