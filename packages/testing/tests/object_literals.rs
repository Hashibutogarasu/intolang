#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::{eval_run, eval_vm, eval_vm_bundled};

const PROGRAM: &str = "import \"intolang://builtin/std\";

pub Json() {
    pub into size = 0;

    pub static fun object(): Json {
        into made = Json();
        return made;
    }

    pub fun setString(key: String, value: String) {
        println(\"string \" + key + \" \" + value);
    }

    pub fun setInt(key: String, value: Int64) {
        println(\"int \" + key);
        println(value);
    }

    pub fun setFloat(key: String, value: Float64) {
        println(\"float \" + key);
        println(value);
    }

    pub fun setBool(key: String, value: Boolean) {
        println(\"bool \" + key);
        println(value);
    }

    pub fun setJson(key: String, value: Json) {
        println(\"json \" + key);
    }
}

init {
    now into json: Json = { \"foo\": \"bar\", \"number\": 0 };
    into ratio = { \"pi\": 3.5, \"yes\": true };
    into nested: Json = { \"inner\": { \"a\": \"b\" }, \"last\": 'x' };
    into empty: Json = {};
}
";

const EXPECTED: &str = "string foo bar\nint number\n0\nfloat pi\n3.500000\nbool yes\n1\nstring a b\njson inner\nint last\n120\n";

#[test]
fn an_object_literal_stores_its_entries_in_a_json_object() {
    let output = eval_vm_bundled(PROGRAM, VmOptions::default()).unwrap();
    assert_eq!(output.stdout, EXPECTED);
}

#[test]
fn compiled_code_builds_objects_like_the_vm() {
    let compiled = eval_run(PROGRAM).unwrap();
    let virtual_run = eval_vm_bundled(PROGRAM, VmOptions::default()).unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(compiled.stdout, virtual_run.stdout);
    assert_eq!(compiled.stdout, EXPECTED);
}

#[test]
fn an_object_literal_needs_the_json_class() {
    let program =
        "import \"intolang://builtin/std\";\n\ninit {\n    into value = { \"a\": 1 };\n}\n";
    let message = eval_vm(program).unwrap_err().to_string();
    assert!(message.contains("needs the class `Json`"), "{message}");
}
