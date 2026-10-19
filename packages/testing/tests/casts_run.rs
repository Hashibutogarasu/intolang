#![cfg(feature = "run")]

use testing::{eval_run, eval_vm};

#[test]
fn compiled_code_converts_like_the_vm() {
    let source = r#"
import "intolang://builtin/std";

init {
    into c: Char = 'A';
    println(c);
    into n: Int32 = c as Int32;
    println(n);
    into digit: Int32 = 7;
    into d: Char = (('0' as Int32) + digit) as Char;
    println(d);
    into big: Int32 = 300;
    into small: Int8 = big as Int8;
    println(small);
    into f: Float64 = big as Float64;
    into back: Int32 = f as Int32;
    println(back);
    into pi: Float64 = 3.75;
    into whole: Int32 = pi as Int32;
    println(whole);
    if (c == 'A') {
        println("same");
    }
}
"#;
    let compiled = eval_run(source).unwrap();
    let virtual_run = eval_vm(source).unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(compiled.stdout, virtual_run.stdout);
    assert_eq!(
        compiled.stdout,
        r#"A
65
7
44
300
3
same
"#
    );
}
