#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::{eval_run, eval_vm_bundled};

const PROGRAM: &str = "import \"intolang://builtin/std\";

init {
    Console.write(\"one\");
    Console.write(\" two\\n\");
    println(\"three\");
    print(\"four\\n\");
    now into word = \"five\";
    Console.write(word);
    Console.write(\"\\n\");
}
";

const EXPECTED: &str = "one two\nthree\nfour\nfive\n";

const READ_PROGRAM: &str = "import \"intolang://builtin/std\";

init {
    into first = Console.read();
    into second = Console.read();
    println(first);
    println(second);
    into third = Console.read();
    println(third.length());
}
";

#[test]
fn the_console_writes_text_in_the_vm() {
    let output = eval_vm_bundled(PROGRAM, VmOptions::default()).unwrap();
    assert_eq!(output.stdout, EXPECTED);
}

#[test]
fn compiled_code_writes_to_the_console_like_the_vm() {
    let compiled = eval_run(PROGRAM).unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(compiled.stdout, EXPECTED);
}

#[test]
fn the_console_reads_lines_in_the_vm() {
    let options = VmOptions {
        stdin: Some(b"hello\r\nworld".to_vec()),
        ..VmOptions::default()
    };
    let output = eval_vm_bundled(READ_PROGRAM, options).unwrap();
    assert_eq!(output.stdout, "hello\nworld\n0\n");
}

#[test]
fn reading_at_the_end_of_the_input_gives_empty_text() {
    let virtual_run = eval_vm_bundled(READ_PROGRAM, VmOptions::default()).unwrap();
    let compiled = eval_run(READ_PROGRAM).unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(virtual_run.stdout, "\n\n0\n");
    assert_eq!(compiled.stdout, virtual_run.stdout);
}
