#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::{eval_run, eval_vm_bundled};

const PROGRAM: &str = "import \"intolang://builtin/std\";

pub Entry() {
    pub into label = \"\";
}

pub Table() {
    pub into: [String] names = [];
    pub into: [Entry] entries = [];
    pub into first = Entry();

    pub static fun make(): Table {
        into made = Table();
        made.names = [];
        made.entries = [];
        return made;
    }

    pub fun add(name: String) {
        this.names.push(name);
    }

    pub fun count(): Int32 {
        return this.names.length();
    }
}

init {
    into table = Table.make();
    table.add(\"a\");
    table.add(\"b\");
    println(table.count());
    println(table.names.get(1));
    table.names.push(\"c\");
    println(table.names.length());
    into entry = Entry();
    entry.label = \"x\";
    table.entries = [entry];
    println(table.entries.length());
    table.first = entry;
    println(table.first.label);
    table.names.forEach((name) => {
        println(name);
    });
}
";

const EXPECTED: &str = "2\nb\n3\n1\nx\na\nb\nc\n";

#[test]
fn fields_hold_lists_and_instances() {
    let output = eval_vm_bundled(PROGRAM, VmOptions::default()).unwrap();
    assert_eq!(output.stdout, EXPECTED);
}

#[test]
fn compiled_code_keeps_lists_in_fields_like_the_vm() {
    let compiled = eval_run(PROGRAM).unwrap();
    let virtual_run = eval_vm_bundled(PROGRAM, VmOptions::default()).unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(compiled.stdout, virtual_run.stdout);
    assert_eq!(compiled.stdout, EXPECTED);
}

#[test]
fn a_field_of_another_type_is_rejected() {
    let program = "import \"intolang://builtin/std\";

pub Table() {
    pub into: [String] names = [];
}

init {
    into table = Table();
    into numbers: [Int32] = [1];
    table.names = numbers;
}
";
    let message = eval_vm_bundled(program, VmOptions::default())
        .unwrap_err()
        .to_string();
    assert!(message.contains("The field `names`"), "{message}");
}
