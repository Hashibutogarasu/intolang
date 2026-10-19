#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::{eval_run, eval_vm_bundled};

#[test]
fn compiled_code_converts_to_hex_like_the_vm() {
    let source = r#"
import "intolang://builtin/std";
import "intolang://builtin/hex";

init {
    into c: Char = 'A';
    into h: Hex = c as Hex;
    println(h.digits);
    into n: Hex = 255 as Hex;
    println(n.digits);
    into zero: Hex = 0 as Hex;
    println(zero.digits);
    into big: Hex = 4096 as Hex;
    println(big.digits);
    into z: Hex = 'Z' as Hex;
    into label: String = "0x" + z.digits;
    println(label);
}
"#;
    let compiled = eval_run(source).unwrap();
    let virtual_run = eval_vm_bundled(source, VmOptions::default()).unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(compiled.stdout, virtual_run.stdout);
    assert_eq!(
        compiled.stdout,
        r#"41
ff
0
1000
0x5a
"#
    );
}
