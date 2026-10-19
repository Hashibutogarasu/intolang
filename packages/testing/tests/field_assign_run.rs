#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::{eval_run, eval_vm_bundled};

#[test]
fn compiled_code_assigns_fields_like_the_vm() {
    let source = r#"
import "intolang://builtin/std";

pub Record() {
    pub into: Int32 count = 7;
    pub into: Float64 ratio = 0.5;
    pub into: String label = "none";
    pub into: Boolean ready = false;
    pub into: Char mark = 'a';

    pub fun reset() {
        this.count = 0;
    }
}

init {
    into r = Record();
    println(r.count);
    r.count = 9;
    println(r.count);
    into small: Int32 = 2;
    r.ratio = small;
    println(r.ratio);
    r.ratio = 1.5;
    println(r.ratio);
    println(r.label);
    r.label = "fixed";
    println(r.label);
    into word: String = "abc";
    into up = word.toUpper();
    r.label = up + "!";
    println(r.label);
    r.ready = true;
    println(r.ready);
    r.mark = 'z';
    println(r.mark);
    r.reset();
    println(r.count);
}
"#;
    let compiled = eval_run(source).unwrap();
    let virtual_run = eval_vm_bundled(source, VmOptions::default()).unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(compiled.stdout, virtual_run.stdout);
    assert_eq!(
        compiled.stdout,
        r#"7
9
2.000000
1.500000
none
fixed
ABC!
1
z
0
"#
    );
}
