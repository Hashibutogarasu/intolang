#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::{eval_run, eval_vm_bundled};

const PROGRAM: &str = "import \"intolang://builtin/std\";

pub fun fib(n: Int32): Int32 {
    if (n < 2) {
        return n;
    }
    return fib(n - 1) + fib(n - 2);
}

pub fun sign(n: Int32): Int32 {
    if (n < 0) {
        return 0 - 1;
    }
    if (n == 0) {
        return 0;
    }
    return 1;
}

pub fun report(n: Int32) {
    if (n > 2) {
        println(\"big\");
        return;
    }
    println(\"small\");
}

pub fun repeat(text: String, count: Int32): String {
    if (count < 1) {
        return \"\";
    }
    return text + repeat(text, count - 1);
}

pub fun even(n: Int32): Int32 {
    if (n == 0) {
        return 1;
    }
    return odd(n - 1);
}

pub fun odd(n: Int32): Int32 {
    if (n == 0) {
        return 0;
    }
    return even(n - 1);
}

pub fun half(n: Float64): Float64 {
    if (n < 1.0) {
        return n;
    }
    return half(n / 2.0);
}

pub Counter() {
    pub into: Int32 step = 3;

    pub fun total(n: Int32): Int32 {
        if (n < 1) {
            return 0;
        }
        return this.step + this.total(n - 1);
    }
}

init {
    println(fib(12));
    println(sign(7));
    println(sign(0 - 5));
    report(1);
    report(5);
    println(repeat(\"ab\", 3));
    println(even(10));
    println(even(7));
    into whole: Int32 = 8;
    println(half(whole));
    into c = Counter();
    println(c.total(4));
}
";

#[test]
fn compiled_code_calls_functions_like_the_vm() {
    let compiled = eval_run(PROGRAM).unwrap();
    let virtual_run = eval_vm_bundled(PROGRAM, VmOptions::default()).unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(compiled.stdout, virtual_run.stdout);
    assert_eq!(
        compiled.stdout,
        "144\n1\n-1\nsmall\nbig\nababab\n1\n0\n0.500000\n12\n"
    );
}
