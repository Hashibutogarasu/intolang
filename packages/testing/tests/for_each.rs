#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::{eval_run, eval_vm_bundled};

const PROGRAM: &str = "import \"intolang://builtin/std\";

pub Item() {
    pub into name = \"\";
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
    into names: [String] = [\"a\", \"b\"];
    names.forEach((name) => {
        println(name + \"!\");
    });
    into first = Item();
    first.name = \"one\";
    into second = Item();
    second.name = \"two\";
    into items: [Item] = [];
    items.push(first);
    items.push(second);
    items.forEach((item: Item) => {
        println(item.name);
    });
    into empty: [Int32] = [];
    empty.forEach((n) => {
        println(\"never\");
    });
}
";

const EXPECTED: &str = "10\n20\n30\n10\n21\n32\n60\n10\n20\n30\na!\nb!\none\ntwo\n";

#[test]
fn for_each_hands_every_item_to_the_lambda() {
    let output = eval_vm_bundled(PROGRAM, VmOptions::default()).unwrap();
    assert_eq!(output.stdout, EXPECTED);
}

#[test]
fn compiled_code_loops_like_the_vm() {
    let compiled = eval_run(PROGRAM).unwrap();
    let virtual_run = eval_vm_bundled(PROGRAM, VmOptions::default()).unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(compiled.stdout, virtual_run.stdout);
    assert_eq!(compiled.stdout, EXPECTED);
}

#[test]
fn the_lambda_must_fit_the_items() {
    let program = "import \"intolang://builtin/std\";

init {
    into numbers: [Int32] = [1];
    numbers.forEach((n: String) => {
        println(n);
    });
}
";
    let message = eval_vm_bundled(program, VmOptions::default())
        .unwrap_err()
        .to_string();
    assert!(
        message.contains("The parameter `n` is a `String`"),
        "{message}"
    );
    let program = "import \"intolang://builtin/std\";

init {
    into numbers: [Int32] = [1];
    numbers.forEach((a, b, c) => {
        println(a);
    });
}
";
    let message = eval_vm_bundled(program, VmOptions::default())
        .unwrap_err()
        .to_string();
    assert!(message.contains("not 3 parameter(s)"), "{message}");
}
