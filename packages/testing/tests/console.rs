#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::{eval_run, eval_vm_bundled};

#[test]
fn the_console_writes_text() {
    let source = r#"
import "intolang://builtin/std";

init {
    Console.write("one");
    Console.write(" two\n");
    println("three");
    print("four\n");
    now into word = "five";
    Console.write(word);
    Console.write("\n");
}
"#;
    let expected = r#"one two
three
four
five
"#;
    let output = eval_vm_bundled(source, VmOptions::default()).unwrap();
    assert_eq!(output.stdout, expected);
    let compiled = eval_run(source).unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(compiled.stdout, expected);
}

#[test]
fn the_console_reads_lines_in_the_vm() {
    let options = VmOptions {
        stdin: Some(b"hello\r\nworld".to_vec()),
        ..VmOptions::default()
    };
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

init {
    into first = Console.read();
    into second = Console.read();
    println(first);
    println(second);
    into third = Console.read();
    println(third.length());
}
"#,
        options,
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"hello
world
0
"#
    );
}

#[test]
fn reading_at_the_end_of_the_input_gives_empty_text() {
    let source = r#"
import "intolang://builtin/std";

init {
    into line = Console.read();
    println(line.length());
}
"#;
    let virtual_run = eval_vm_bundled(source, VmOptions::default()).unwrap();
    let compiled = eval_run(source).unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(
        virtual_run.stdout,
        r#"0
"#
    );
    assert_eq!(compiled.stdout, virtual_run.stdout);
}
