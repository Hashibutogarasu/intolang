#![cfg(feature = "run")]

use testing::{eval_run, eval_vm};

const STD: &str = "import \"intolang://builtin/std\";\n";

const PROGRAM: &str = "init {
    into a: Int32 = 7;
    into b: Int32 = a % 4 + 2 * 3;
    println(b);
    into c: Int32 = (a & 3) | 8;
    println(c);
    into d: Int32 = a << 2;
    println(d);
    into e: Int32 = d >> 1 ^ 5;
    println(e);
    into x: Int32 = 2;
    into y: Int32 = 5;
    into z: Int32 = (x + y) * (y - x) - x;
    println(z);
    into zero: Int32 = 0;
    if (a > 3 && b < 10) {
        println(\"both\");
    }
    if (a > 100 && 10 / zero > 1) {
        println(\"never\");
    } else {
        println(\"skipped\");
    }
    into big: Boolean = a > 3;
    if (big && !(a == 8)) {
        println(\"yes\");
    }
    into n: Int32 = 0;
    while (n < 10 && n * n < 30) {
        n = n + 1;
    }
    println(n);
}
";

#[test]
fn compiled_code_prints_what_the_vm_prints() {
    let source = format!("{STD}{PROGRAM}");
    let compiled = eval_run(&source).unwrap();
    let virtual_run = eval_vm(&source).unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(compiled.stdout, virtual_run.stdout);
    assert_eq!(
        compiled.stdout,
        "9\n11\n28\n11\n19\nboth\nskipped\nyes\n6\n"
    );
}
