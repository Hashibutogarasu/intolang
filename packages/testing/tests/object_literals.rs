#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::{eval_run, eval_vm, eval_vm_bundled};

#[test]
fn an_object_literal_stores_its_entries_in_a_json_object() {
    let source = r#"
import "intolang://builtin/std";

pub Json() {
    pub into size = 0;

    pub static fun object(): Json {
        into made = Json();
        return made;
    }

    pub fun setString(key: String, value: String) {
        println("string " + key + " " + value);
    }

    pub fun setInt(key: String, value: Int64) {
        println("int " + key);
        println(value);
    }

    pub fun setFloat(key: String, value: Float64) {
        println("float " + key);
        println(value);
    }

    pub fun setBool(key: String, value: Boolean) {
        println("bool " + key);
        println(value);
    }

    pub fun setJson(key: String, value: Json) {
        println("json " + key);
    }
}

init {
    now into json: Json = { "foo": "bar", "number": 0 };
    into ratio = { "pi": 3.5, "yes": true };
    into nested: Json = { "inner": { "a": "b" }, "last": 'x' };
    into empty: Json = {};
}
"#;
    let expected = r#"string foo bar
int number
0
float pi
3.500000
bool yes
1
string a b
json inner
int last
120
"#;
    let virtual_run = eval_vm_bundled(source, VmOptions::default()).unwrap();
    assert_eq!(virtual_run.stdout, expected);
    let compiled = eval_run(source).unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(compiled.stdout, expected);
}

#[test]
fn an_object_literal_needs_the_json_class() {
    let message = eval_vm(
        r#"
import "intolang://builtin/std";

init {
    into value = { "a": 1 };
}
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(message.contains("needs the class `Json`"), "{message}");
}
