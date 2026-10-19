#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::{eval_run, eval_vm_bundled};

#[test]
fn compiled_code_calls_lambdas_like_the_vm() {
    let source = r#"
import "intolang://builtin/std";

pub fun each(count: Int32, action: Callback<Int32>) {
    into i: Int32 = 0;
    while (i < count) {
        action(i);
        i = i + 1;
    }
}

pub fun greet(name: String, action: Callback<String>) {
    action(name);
}

pub fun pairs(action: BiCallback<String, Int32>) {
    action("one", 1);
    action("two", 2);
}

init {
    each(3, (n: Int32) => {
        println(n);
    });
    greet("world", (who) => {
        println("hello " + who);
    });
    into total: Int32 = 0;
    each(4, (n) => {
        total = total + n;
    });
    println(total);
    into show: Callback<Int32> = (n) => {
        println(n * 2);
    };
    each(2, show);
    pairs((name, number) => {
        println(name);
        println(number);
    });
}
"#;
    let compiled = eval_run(source).unwrap();
    let virtual_run = eval_vm_bundled(source, VmOptions::default()).unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(compiled.stdout, virtual_run.stdout);
    assert_eq!(
        compiled.stdout,
        r#"0
1
2
hello world
6
0
2
one
1
two
2
"#
    );
}
