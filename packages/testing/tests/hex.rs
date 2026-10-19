#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::eval_vm_bundled;

#[test]
fn a_character_converts_to_the_hex_digits_of_its_code() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";
import "intolang://builtin/hex";

init {
    into c: Char = 'A';
    into h: Hex = c as Hex;
    println(h.digits);
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"41
"#
    );
}

#[test]
fn a_number_converts_to_hex_digits() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";
import "intolang://builtin/hex";

init {
    into n: Hex = 255 as Hex;
    println(n.digits);
    into zero: Hex = 0 as Hex;
    println(zero.digits);
    into big: Hex = 4096 as Hex;
    println(big.digits);
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"ff
0
1000
"#
    );
}

#[test]
fn a_character_outside_ascii_converts_through_its_code() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";
import "intolang://builtin/hex";

init {
    into h: Hex = 'あ' as Hex;
    println(h.digits);
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"3042
"#
    );
}

#[test]
fn a_text_does_not_convert_to_hex() {
    let message = format!(
        "{:#}",
        eval_vm_bundled(
            r#"
import "intolang://builtin/std";
import "intolang://builtin/hex";

init {
    into s: String = "A";
    into h: Hex = s as Hex;
}
"#,
            VmOptions::default(),
        )
        .unwrap_err()
    );
    assert!(message.contains("no cast from String to Hex"), "{message}");
}

#[test]
fn the_digits_can_be_joined_to_a_text() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";
import "intolang://builtin/hex";

init {
    into h: Hex = 'Z' as Hex;
    into label: String = "0x" + h.digits;
    println(label);
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"0x5a
"#
    );
}
