#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::{eval_run, eval_vm_bundled};

#[test]
fn memory_is_read_and_written_by_item() {
    let source = r#"
import "intolang://builtin/std";

init {
    into bytes = Memory.allocate(64);
    bytes.storeInt32(0, 100);
    bytes.storeInt32(1, 200);
    println(bytes.loadInt32(0));
    println(bytes.loadInt32(1));
    into index: Int32 = 1;
    bytes.storeInt32(index, 7);
    println(bytes.loadInt32(index));
    bytes.storeInt8(8, 65);
    println(bytes.loadInt8(8));
    into words: Int32* = Memory.allocate(16);
    words.store(2, 42);
    println(words.load(2));
    into floats = Memory.allocate(32);
    floats.storeFloat64(1, 2.5);
    println(floats.loadFloat64(1));
    floats.storeFloat32(0, 1.5);
    println(floats.loadFloat32(0));
    into marks = Memory.allocate(8);
    marks.storeChar(1, 'z');
    println(marks.loadChar(1));
    into big = Memory.allocate(16);
    big.storeInt64(1, 5000000000);
    println(big.loadInt64(1));
    into pixels: Int32* = Memory.allocate(4 * 8);
    into i: Int32 = 0;
    while (i < 8) {
        pixels.store(i, i * i);
        i = i + 1;
    }
    into total: Int32 = 0;
    i = 0;
    while (i < 8) {
        total = total + pixels.load(i);
        i = i + 1;
    }
    println(total);
    Memory.free(bytes);
}
"#;
    let expected = r#"100
200
7
65
42
2.500000
1.500000
z
5000000000
140
"#;
    let virtual_run = eval_vm_bundled(source, VmOptions::default()).unwrap();
    assert_eq!(virtual_run.stdout, expected);
    let compiled = eval_run(source).unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(compiled.stdout, expected);
}

#[test]
fn a_plain_pointer_needs_the_type_in_the_method_name() {
    let message = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

init {
    into bytes = Memory.allocate(8);
    bytes.store(0, 1);
}
"#,
        VmOptions::default(),
    )
    .unwrap_err()
    .to_string();
    assert!(message.contains("store"), "{message}");
}

#[test]
fn a_null_pointer_is_reported_in_the_vm() {
    let message = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

init {
    into nothing = Memory.allocate(0);
    nothing.storeInt32(0, 1);
}
"#,
        VmOptions::default(),
    )
    .unwrap_err()
    .to_string();
    assert!(message.contains("null pointer"), "{message}");
}
