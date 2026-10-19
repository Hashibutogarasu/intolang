#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::eval_vm_bundled;

#[test]
fn an_array_literal_makes_a_list_of_numbers() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

init {
    into xs: [Int32] = [10, 20, 30];
    println(xs.length());
    println(xs.get(0));
    println(xs.get(2));
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"3
10
30
"#
    );
}

#[test]
fn the_element_type_comes_from_the_items() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

init {
    into xs = [1.5, 2.5];
    println(xs.get(1));
    into names = ["a", "b"];
    println(names.get(1));
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"2.500000
b
"#
    );
}

#[test]
fn items_are_pushed_and_replaced() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

init {
    into xs: [Int32] = [1];
    xs.push(2);
    xs.push(3);
    xs.set(0, 9);
    println(xs.length());
    println(xs.get(0));
    println(xs.get(2));
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"3
9
3
"#
    );
}

#[test]
fn an_empty_list_takes_its_type_from_the_annotation() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

init {
    into xs: [String] = [];
    xs.push("first");
    xs.push("second" + "!");
    println(xs.length());
    println(xs.get(1));
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"2
second!
"#
    );
}

#[test]
fn characters_and_other_integers_are_stored_as_they_are() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

init {
    into cs: [Char] = ['a', 'b'];
    println(cs.get(1));
    into small: [Int8] = [1, 2];
    println(small.get(0));
    into big: [Int64] = [5000000000];
    println(big.get(0));
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"b
1
5000000000
"#
    );
}

#[test]
fn a_list_can_hold_instances() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

pub Item() {
    pub into: String name = "";
}

init {
    into first = Item();
    first.name = "one";
    into second = Item();
    second.name = "two";
    into items: [Item] = [];
    items.push(first);
    items.push(second);
    into again = items.get(1);
    println(again.name);
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"two
"#
    );
}

#[test]
fn an_item_of_the_wrong_type_is_an_error() {
    let message = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

init {
    into xs: [Int32] = [1];
    xs.push("text");
}
"#,
        VmOptions::default(),
    )
    .unwrap_err();
    assert!(
        format!("{message:#}").contains("Expected a `Int32`"),
        "{message:#}"
    );
    let message = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

init {
    into xs: [String] = [];
    xs.push(3);
}
"#,
        VmOptions::default(),
    )
    .unwrap_err();
    assert!(
        format!("{message:#}").contains("Expected a `String` item"),
        "{message:#}"
    );
}

#[test]
fn an_empty_array_without_a_type_is_an_error() {
    let message = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

init {
    into xs = [];
}
"#,
        VmOptions::default(),
    )
    .unwrap_err();
    assert!(
        format!("{message:#}").contains("An empty array needs a type"),
        "{message:#}"
    );
}

#[test]
fn a_loop_adds_up_the_items() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

init {
    into xs: [Int32] = [4, 5, 6];
    into total: Int32 = 0;
    into i: Int32 = 0;
    while (i < xs.length()) {
        total = total + xs.get(i);
        i = i + 1;
    }
    println(total);
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"15
"#
    );
}
