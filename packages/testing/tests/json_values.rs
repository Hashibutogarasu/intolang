#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::{eval_run, eval_vm_bundled};

#[test]
fn json_values_are_built_and_written_in_into() {
    let source = r#"
import "intolang://builtin/std";

init {
    now into json: Json = { "foo": "bar", "number": 0 };
    println(json.toString());
    println(json.foo);
    println(json.number);
    println(json.type);
    println(json.number.type);
    println(json.foo.type);
    into ratio: Json = { "pi": 3.5, "yes": true, "big": 5000000000, "neg": -12 };
    println(ratio.toString());
    println(ratio.big.type);
    into nested: Json = { "inner": { "a": "b" }, "last": 'x' };
    println(nested.toString());
    println(nested.inner.a);
    println(nested.missing.toString());
    into quoted: Json = { "text": "say \"hi\"\n" };
    println(quoted.toString());
    println(json.has("foo"));
    println(json.has("zzz"));
    println(json.length());
    json.forEach((key, value) => {
        println(key + " = " + value.toString());
    });
    json.fields.forEach((field: JsonField) => {
        println(field.name);
    });
    json.setString("foo", "replaced");
    println(json.toString());
    into list = Json.array();
    list.push(Json.ofInt(1));
    list.push(Json.ofString("two"));
    list.push(Json.ofFloat(0.25));
    list.push(Json.null());
    println(list.toString());
    println(list.at(1));
    println(list.length());
}
"#;
    let expected = r#"{"foo":"bar","number":0}
bar
0
Json<String, Object>
Int32
String
{"pi":3.5,"yes":true,"big":5000000000,"neg":-12}
Int64
{"inner":{"a":"b"},"last":120}
b
null
{"text":"say \"hi\"\n"}
1
0
2
foo = bar
number = 0
foo
number
{"foo":"replaced","number":0}
[1,"two",0.25,null]
two
4
"#;
    let virtual_run = eval_vm_bundled(source, VmOptions::default()).unwrap();
    assert_eq!(virtual_run.stdout, expected);
    let compiled = eval_run(source).unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(compiled.stdout, expected);
}

#[test]
fn json_text_is_parsed_in_into() {
    let source = r#"
import "intolang://builtin/std";

init {
    into doc = Json.parse("{ \"name\": \"neko\\n\", \"n\": -12, \"pi\": 3.25, \"e\": 1e2, \"ok\": true, \"none\": null, \"list\": [1, \"a\", [2]], \"o\": { \"k\": \"\\u00e9\\u0041\" } }");
    println(doc.toString());
    println(doc.n);
    println(doc.o.k);
    into inner = doc.list.at(2);
    println(inner.at(0));
    into broken = Json.parse("[1,");
    println(broken.toString());
    into empty = Json.parse("{}");
    println(empty.length());
}
"#;
    let expected = r#"{"name":"neko\n","n":-12,"pi":3.25,"e":100,"ok":true,"none":null,"list":[1,"a",[2]],"o":{"k":"éA"}}
-12
éA
2
null
0
"#;
    let virtual_run = eval_vm_bundled(source, VmOptions::default()).unwrap();
    assert_eq!(virtual_run.stdout, expected);
    let compiled = eval_run(source).unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(compiled.stdout, expected);
}

#[test]
fn small_numbers_are_parsed_alike_in_compiled_code_and_the_vm() {
    let source = r#"
import "intolang://builtin/std";

init {
    into one = Json.parse("[1]");
    println(one.toString());
    into pair = Json.parse("[1, 2]");
    println(pair.toString());
    into mixed = Json.parse("[\"x\", 7, -3]");
    println(mixed.toString());
}
"#;
    let expected = r#"[1]
[1,2]
["x",7,-3]
"#;
    let virtual_run = eval_vm_bundled(source, VmOptions::default()).unwrap();
    assert_eq!(virtual_run.stdout, expected);
    let compiled = eval_run(source).unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(compiled.stdout, expected);
}

#[test]
fn json_is_read_from_a_file_through_binary() {
    let directory = std::env::temp_dir().join(format!("into-json-read-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("sample.json");
    std::fs::write(&path, r#"{ "values": [1, 2.5, "x"], "ok": true }"#).unwrap();
    let source = r#"
import "intolang://builtin/std";

pub fun load(path: Path): Json {
    return Json.read(path);
}

pub fun size(path: Path): Int64 {
    into binary = Binary.read(path);
    return binary.length();
}

init {
    into doc = Json.read("<path>");
    println(doc.toString());
    into missing = Json.read("<path>.missing");
    println(missing.toString());
    into forwarded: Path = "<path>";
    into again = load(forwarded);
    println(again.toString());
    println(size(forwarded));
}
"#
    .replace("<path>", &path.display().to_string());
    let expected = r#"{"values":[1,2.5,"x"],"ok":true}
null
{"values":[1,2.5,"x"],"ok":true}
39
"#;
    let virtual_run = eval_vm_bundled(&source, VmOptions::default());
    let compiled = eval_run(&source);
    std::fs::remove_dir_all(&directory).unwrap();
    assert_eq!(virtual_run.unwrap().stdout, expected);
    let compiled = compiled.unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(compiled.stdout, expected);
}
