#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::{eval_run, eval_vm_bundled};

const STD: &str = "import \"intolang://builtin/std\";\n";

const PROGRAM: &str = "init {
    into s: String = \"Hello, World\";
    println(s.length());
    println(s.at(1));
    println(s.subString(7, 12));
    println(s.indexOf(\"World\"));
    println(s.indexOf(\"nope\"));
    into lower = s.toLower();
    into upper = s.toUpper();
    println(lower);
    println(upper);
    into c: Char = '!';
    into joined = lower + \", \" + upper + c;
    println(joined);
    into digit: Int32 = 7;
    into numbered = lower + (('0' as Int32) + digit) as Char;
    println(numbered);
    if (s.startsWith(\"Hello\")) {
        println(\"starts\");
    }
    if (s.endsWith(\"nope\")) {
        println(\"never\");
    } else {
        println(\"does not end\");
    }
    if (lower == \"hello, world\") {
        println(\"same\");
    }
    if (lower != upper) {
        println(\"different\");
    }
    into again = s.toLower();
    if (lower == again) {
        println(\"equal\");
    }
    into n: Int32 = 0;
    into text = s.toUpper();
    while (n < 3) {
        text = text + c;
        n = n + 1;
    }
    println(text);
}
";

#[test]
fn compiled_code_handles_text_like_the_vm() {
    let source = format!("{STD}{PROGRAM}");
    let compiled = eval_run(&source).unwrap();
    let virtual_run = eval_vm_bundled(&source, VmOptions::default()).unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(compiled.stdout, virtual_run.stdout);
    assert_eq!(
        compiled.stdout,
        "12\ne\nWorld\n7\n-1\nhello, world\nHELLO, WORLD\nhello, world, HELLO, WORLD!\nhello, world7\nstarts\ndoes not end\nsame\ndifferent\nequal\nHELLO, WORLD!!!\n"
    );
}
