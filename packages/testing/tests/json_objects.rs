#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::{eval_run, eval_vm_bundled};

#[test]
fn a_json_object_class_writes_itself_as_json() {
    let source = r#"
import "intolang://builtin/std";

pub Cat(): JsonObject {
    pub into neko = "cat";

    pub override fun toJson(): Json {
        into json = Json.object();
        json.set("neko", this.neko);
        return json;
    }
}

pub Dog(): JsonObject {
    pub into foo = "bar";
    pub into cat = Cat();
    pub into: Int32 age = 3;
    pub into ratio = 0.5;
    pub into ok = true;

    pub override fun toJson(): Json {
        into json = Json.object();
        json.set("foo", this.foo);
        json.set("cat", this.cat);
        json.set("age", this.age);
        json.set("ratio", this.ratio);
        json.set("ok", this.ok);
        return json;
    }
}

init {
    now into dog = Dog();
    now into dogJson: Json = dog.toJson();
    println(dogJson.foo);
    println(dogJson.cat.neko);
    println(dogJson.toString());
    dogJson.fields.forEach((field: JsonField) => {
        println(field.name);
    });
}
"#;
    let expected = r#"bar
cat
{"foo":"bar","cat":{"neko":"cat"},"age":3,"ratio":0.5,"ok":true}
foo
cat
age
ratio
ok
"#;
    let output = eval_vm_bundled(source, VmOptions::default()).unwrap();
    assert_eq!(output.stdout, expected);
    let compiled = eval_run(source).unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(compiled.stdout, expected);
}
