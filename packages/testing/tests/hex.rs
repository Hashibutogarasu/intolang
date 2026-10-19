#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::eval_vm_bundled;

const IMPORTS: &str = "import \"intolang://builtin/std\";\nimport \"intolang://builtin/hex\";\n";

fn run(body: &str) -> Result<String, String> {
    eval_vm_bundled(
        &format!("{IMPORTS}init {{\n{body}}}\n"),
        VmOptions::default(),
    )
    .map(|outcome| outcome.stdout)
    .map_err(|error| format!("{error:#}"))
}

#[test]
fn a_character_converts_to_the_hex_digits_of_its_code() {
    let output =
        run("    into c: Char = 'A';\n    into h: Hex = c as Hex;\n    println(h.digits);\n")
            .unwrap();
    assert_eq!(output, "41\n");
}

#[test]
fn a_number_converts_to_hex_digits() {
    let output = run(
        "    into n: Hex = 255 as Hex;\n    println(n.digits);\n    into zero: Hex = 0 as Hex;\n    println(zero.digits);\n    into big: Hex = 4096 as Hex;\n    println(big.digits);\n",
    )
    .unwrap();
    assert_eq!(output, "ff\n0\n1000\n");
}

#[test]
fn a_character_outside_ascii_converts_through_its_code() {
    let output = run("    into h: Hex = 'あ' as Hex;\n    println(h.digits);\n").unwrap();
    assert_eq!(output, "3042\n");
}

#[test]
fn a_text_does_not_convert_to_hex() {
    let message = run("    into s: String = \"A\";\n    into h: Hex = s as Hex;\n").unwrap_err();
    assert!(message.contains("no cast from String to Hex"), "{message}");
}

#[test]
fn the_digits_can_be_joined_to_a_text() {
    let output = run(
        "    into h: Hex = 'Z' as Hex;\n    into label: String = \"0x\" + h.digits;\n    println(label);\n",
    )
    .unwrap();
    assert_eq!(output, "0x5a\n");
}
