#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::{eval_run, eval_vm_bundled};

const PROGRAM: &str = "import \"intolang://builtin/std\";

pub Item() {
    pub into name = \"\";
}

init {
    into ages: Map<String, String> = Map.create();
    ages.set(\"ann\", \"thirty\");
    ages.set(\"bob\", \"forty\");
    println(ages.size());
    println(ages.get(\"bob\"));
    ages.set(\"ann\", \"thirty-one\");
    println(ages.size());
    println(ages.get(\"ann\"));
    if (ages.has(\"bob\")) {
        println(\"has bob\");
    }
    if (ages.has(\"zed\")) {
        println(\"never\");
    } else {
        println(\"no zed\");
    }
    println(ages.indexOf(\"zed\"));
    into i: Int32 = 0;
    while (i < ages.size()) {
        println(ages.keyAt(i));
        println(ages.valueAt(i));
        i = i + 1;
    }
    into items: Map<String, Item> = Map.create();
    into pen = Item();
    pen.name = \"pen\";
    items.set(\"first\", pen);
    into found = items.get(\"first\");
    println(found.name);
}
";

const EXPECTED: &str =
    "2\nforty\n2\nthirty-one\nhas bob\nno zed\n-1\nann\nthirty-one\nbob\nforty\npen\n";

#[test]
fn a_map_finds_values_by_key() {
    let output = eval_vm_bundled(PROGRAM, VmOptions::default()).unwrap();
    assert_eq!(output.stdout, EXPECTED);
}

#[test]
fn compiled_code_keeps_maps_like_the_vm() {
    let compiled = eval_run(PROGRAM).unwrap();
    let virtual_run = eval_vm_bundled(PROGRAM, VmOptions::default()).unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(compiled.stdout, virtual_run.stdout);
    assert_eq!(compiled.stdout, EXPECTED);
}
