#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::{eval_run, eval_vm_bundled};

#[test]
fn a_map_finds_values_by_key() {
    let source = r#"
import "intolang://builtin/std";

pub Item() {
    pub into name = "";
}

init {
    into ages: Map<String, String> = Map.create();
    ages.set("ann", "thirty");
    ages.set("bob", "forty");
    println(ages.size());
    println(ages.get("bob"));
    ages.set("ann", "thirty-one");
    println(ages.size());
    println(ages.get("ann"));
    if (ages.has("bob")) {
        println("has bob");
    }
    if (ages.has("zed")) {
        println("never");
    } else {
        println("no zed");
    }
    println(ages.indexOf("zed"));
    into i: Int32 = 0;
    while (i < ages.size()) {
        println(ages.keyAt(i));
        println(ages.valueAt(i));
        i = i + 1;
    }
    into items: Map<String, Item> = Map.create();
    into pen = Item();
    pen.name = "pen";
    items.set("first", pen);
    into found = items.get("first");
    println(found.name);
}
"#;
    let expected = r#"2
forty
2
thirty-one
has bob
no zed
-1
ann
thirty-one
bob
forty
pen
"#;
    let virtual_run = eval_vm_bundled(source, VmOptions::default()).unwrap();
    assert_eq!(virtual_run.stdout, expected);
    let compiled = eval_run(source).unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(compiled.stdout, expected);
}
