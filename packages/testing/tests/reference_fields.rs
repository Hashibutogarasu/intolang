#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::{eval_run, eval_vm_bundled};

#[test]
fn fields_hold_lists_and_instances() {
    let source = r#"
import "intolang://builtin/std";

pub Entry() {
    pub into label = "";
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
    table.add("a");
    table.add("b");
    println(table.count());
    println(table.names.get(1));
    table.names.push("c");
    println(table.names.length());
    into entry = Entry();
    entry.label = "x";
    table.entries = [entry];
    println(table.entries.length());
    table.first = entry;
    println(table.first.label);
    table.names.forEach((name) => {
        println(name);
    });
}
"#;
    let expected = r#"2
b
3
1
x
a
b
c
"#;
    let virtual_run = eval_vm_bundled(source, VmOptions::default()).unwrap();
    assert_eq!(virtual_run.stdout, expected);
    let compiled = eval_run(source).unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(compiled.stdout, expected);
}

#[test]
fn a_field_of_another_type_is_rejected() {
    let message = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

pub Table() {
    pub into: [String] names = [];
}

init {
    into table = Table();
    into numbers: [Int32] = [1];
    table.names = numbers;
}
"#,
        VmOptions::default(),
    )
    .unwrap_err()
    .to_string();
    assert!(message.contains("The field `names`"), "{message}");
}
