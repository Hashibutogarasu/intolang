#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::{eval_run, eval_vm_bundled};

const PROGRAM: &str = "import \"intolang://builtin/std\";

init {
    now into json: Json = { \"foo\": \"bar\", \"number\": 0 };
    println(json.toString());
    println(json.foo);
    println(json.number);
    println(json.type);
    println(json.number.type);
    println(json.foo.type);
    into ratio: Json = { \"pi\": 3.5, \"yes\": true, \"big\": 5000000000, \"neg\": -12 };
    println(ratio.toString());
    println(ratio.big.type);
    into nested: Json = { \"inner\": { \"a\": \"b\" }, \"last\": 'x' };
    println(nested.toString());
    println(nested.inner.a);
    println(nested.missing.toString());
    into quoted: Json = { \"text\": \"say \\\"hi\\\"\\n\" };
    println(quoted.toString());
    println(json.has(\"foo\"));
    println(json.has(\"zzz\"));
    println(json.length());
    json.forEach((key, value) => {
        println(key + \" = \" + value.toString());
    });
    json.fields.forEach((field: JsonField) => {
        println(field.name);
    });
    json.setString(\"foo\", \"replaced\");
    println(json.toString());
    into list = Json.array();
    list.push(Json.ofInt(1));
    list.push(Json.ofString(\"two\"));
    list.push(Json.ofFloat(0.25));
    list.push(Json.null());
    println(list.toString());
    println(list.at(1));
    println(list.length());
}
";

const EXPECTED: &str = "{\"foo\":\"bar\",\"number\":0}\nbar\n0\nJson<String, Object>\nInt32\nString\n{\"pi\":3.5,\"yes\":true,\"big\":5000000000,\"neg\":-12}\nInt64\n{\"inner\":{\"a\":\"b\"},\"last\":120}\nb\nnull\n{\"text\":\"say \\\"hi\\\"\\n\"}\n1\n0\n2\nfoo = bar\nnumber = 0\nfoo\nnumber\n{\"foo\":\"replaced\",\"number\":0}\n[1,\"two\",0.25,null]\ntwo\n4\n";

#[test]
fn json_values_are_built_and_written_in_into() {
    let output = eval_vm_bundled(PROGRAM, VmOptions::default()).unwrap();
    assert_eq!(output.stdout, EXPECTED);
}

#[test]
fn compiled_code_writes_json_like_the_vm() {
    let compiled = eval_run(PROGRAM).unwrap();
    let virtual_run = eval_vm_bundled(PROGRAM, VmOptions::default()).unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(compiled.stdout, virtual_run.stdout);
    assert_eq!(compiled.stdout, EXPECTED);
}

const PARSE_PROGRAM: &str = "import \"intolang://builtin/std\";

init {
    into doc = Json.parse(\"{ \\\"name\\\": \\\"neko\\\\n\\\", \\\"n\\\": -12, \\\"pi\\\": 3.25, \\\"e\\\": 1e2, \\\"ok\\\": true, \\\"none\\\": null, \\\"list\\\": [1, \\\"a\\\", [2]], \\\"o\\\": { \\\"k\\\": \\\"\\\\u00e9\\\\u0041\\\" } }\");
    println(doc.toString());
    println(doc.n);
    println(doc.o.k);
    into inner = doc.list.at(2);
    println(inner.at(0));
    into broken = Json.parse(\"[1,\");
    println(broken.toString());
    into empty = Json.parse(\"{}\");
    println(empty.length());
}
";

const PARSE_EXPECTED: &str = "{\"name\":\"neko\\n\",\"n\":-12,\"pi\":3.25,\"e\":100,\"ok\":true,\"none\":null,\"list\":[1,\"a\",[2]],\"o\":{\"k\":\"\u{e9}A\"}}\n-12\n\u{e9}A\n2\nnull\n0\n";

const SMALL_NUMBERS_PROGRAM: &str = "import \"intolang://builtin/std\";

init {
    into one = Json.parse(\"[1]\");
    println(one.toString());
    into pair = Json.parse(\"[1, 2]\");
    println(pair.toString());
    into mixed = Json.parse(\"[\\\"x\\\", 7, -3]\");
    println(mixed.toString());
}
";

#[test]
fn compiled_code_parses_small_numbers_like_the_vm() {
    let compiled = eval_run(SMALL_NUMBERS_PROGRAM).unwrap();
    let virtual_run = eval_vm_bundled(SMALL_NUMBERS_PROGRAM, VmOptions::default()).unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(virtual_run.stdout, "[1]\n[1,2]\n[\"x\",7,-3]\n");
    assert_eq!(compiled.stdout, virtual_run.stdout);
}

#[test]
fn json_text_is_parsed_in_into() {
    let output = eval_vm_bundled(PARSE_PROGRAM, VmOptions::default()).unwrap();
    assert_eq!(output.stdout, PARSE_EXPECTED);
}

#[test]
fn compiled_code_parses_json_like_the_vm() {
    let compiled = eval_run(PARSE_PROGRAM).unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(compiled.stdout, PARSE_EXPECTED);
}

fn read_program(path: &std::path::Path) -> String {
    format!(
        "import \"intolang://builtin/std\";\n\npub fun load(path: Path): Json {{\n    return Json.read(path);\n}}\n\npub fun size(path: Path): Int64 {{\n    into binary = Binary.read(path);\n    return binary.length();\n}}\n\ninit {{\n    into doc = Json.read(\"{}\");\n    println(doc.toString());\n    into missing = Json.read(\"{}.missing\");\n    println(missing.toString());\n    into forwarded: Path = \"{}\";\n    into again = load(forwarded);\n    println(again.toString());\n    println(size(forwarded));\n}}\n",
        path.display(),
        path.display(),
        path.display()
    )
}

#[test]
fn json_is_read_from_a_file_through_binary() {
    let directory = std::env::temp_dir().join(format!("into-json-read-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("sample.json");
    let text = "{ \"values\": [1, 2.5, \"x\"], \"ok\": true }";
    std::fs::write(&path, text).unwrap();
    let program = read_program(&path);
    let expected = format!(
        "{{\"values\":[1,2.5,\"x\"],\"ok\":true}}\nnull\n{{\"values\":[1,2.5,\"x\"],\"ok\":true}}\n{}\n",
        text.len()
    );
    let output = eval_vm_bundled(&program, VmOptions::default());
    let compiled = eval_run(&program);
    std::fs::remove_dir_all(&directory).unwrap();
    assert_eq!(output.unwrap().stdout, expected);
    let compiled = compiled.unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(compiled.stdout, expected);
}
