#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::{eval_run, eval_vm_bundled};

const STD: &str = "import \"intolang://builtin/std\";\n";

const PROGRAM: &str = "pub Record() {
    pub into: Int32 count = 7;
    pub into: Float64 ratio = 0.5;
    pub into: String label = \"none\";
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
    r.label = \"fixed\";
    println(r.label);
    into word: String = \"abc\";
    into up = word.toUpper();
    r.label = up + \"!\";
    println(r.label);
    r.ready = true;
    println(r.ready);
    r.mark = 'z';
    println(r.mark);
    r.reset();
    println(r.count);
}
";

#[test]
fn compiled_code_assigns_fields_like_the_vm() {
    let source = format!("{STD}{PROGRAM}");
    let compiled = eval_run(&source).unwrap();
    let virtual_run = eval_vm_bundled(&source, VmOptions::default()).unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(compiled.stdout, virtual_run.stdout);
    assert_eq!(
        compiled.stdout,
        "7\n9\n2.000000\n1.500000\nnone\nfixed\nABC!\n1\nz\n0\n"
    );
}
