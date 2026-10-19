#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::eval_vm_bundled;

#[test]
fn a_method_is_resolved_through_the_class_of_the_argument_whatever_another_class_names_alike() {
    let outcome = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

pub abstract Method() {
    pub abstract fun verb(): String;
}

pub Get(): Method {
    pub override fun verb(): String {
        return "GET";
    }
}

pub Sink() {
    pub fun send(text: String): Int32 {
        return text.length();
    }
}

pub Client() {
    pub static fun text(method: Method): String {
        into verb = method.verb();
        return verb + "!";
    }

    pub fun call(method: Method): Int32 {
        into done = this.send(method);
        return done;
    }

    pub fun send(method: Method): Int32 {
        now into sink = Sink();
        into count = sink.send(Client.text(method));
        return count;
    }
}

init {
    now into client = Client();
    into result = client(Get());
    println(result);
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        outcome.stdout,
        r#"4
"#
    );
}
