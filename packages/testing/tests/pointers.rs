#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::{eval_run, eval_vm_bundled};

const PROGRAM: &str = "import \"intolang://builtin/std\";

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
";

const EXPECTED: &str = "100\n200\n7\n65\n42\n2.500000\n1.500000\nz\n5000000000\n140\n";

#[test]
fn memory_is_read_and_written_by_item() {
    let output = eval_vm_bundled(PROGRAM, VmOptions::default()).unwrap();
    assert_eq!(output.stdout, EXPECTED);
}

#[test]
fn compiled_code_reads_and_writes_memory_like_the_vm() {
    let compiled = eval_run(PROGRAM).unwrap();
    let virtual_run = eval_vm_bundled(PROGRAM, VmOptions::default()).unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(compiled.stdout, virtual_run.stdout);
    assert_eq!(compiled.stdout, EXPECTED);
}

#[test]
fn a_plain_pointer_needs_the_type_in_the_method_name() {
    let program = "import \"intolang://builtin/std\";

init {
    into bytes = Memory.allocate(8);
    bytes.store(0, 1);
}
";
    let message = eval_vm_bundled(program, VmOptions::default())
        .unwrap_err()
        .to_string();
    assert!(message.contains("store"), "{message}");
}

#[test]
fn a_null_pointer_is_reported_in_the_vm() {
    let program = "import \"intolang://builtin/std\";

init {
    into nothing = Memory.allocate(0);
    nothing.storeInt32(0, 1);
}
";
    let message = eval_vm_bundled(program, VmOptions::default())
        .unwrap_err()
        .to_string();
    assert!(message.contains("null pointer"), "{message}");
}
