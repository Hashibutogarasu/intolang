#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::{eval_run, eval_vm_bundled};

const PROGRAM: &str = "import \"intolang://builtin/std\";

pub Response() {
    pub into status = 0;
}

pub HttpEvents() {
    pub event response(ctx: Response) {}
    pub event progress(percent: Int32) {}
}

pub Client() {
    pub into on = HttpEvents();
    pub into name = \"client\";
}

pub Greeter() {
    pub into greeting = \"hello\";

    pub fun call(name: String) {
        println(this.greeting + \", \" + name);
    }
}

pub event ping(n: Int32) {}

init {
    into client = Client();
    client.on.progress((percent) => {
        println(percent);
    });
    client.on.response((ctx) => {
        println(ctx.status);
    });
    into other = Client();
    into r = Response();
    r.status = 200;
    client.on.progress.emit(50);
    client.on.response.emit(r);
    other.on.progress.emit(99);
    into listener = client.on.progress.on((percent) => {
        println(percent + 1);
    });
    client.on.progress.emit(1);
    listener.destroy();
    client.on.progress.emit(2);
    into g = Greeter();
    g(\"world\");
    ping((n) => {
        println(n * 2);
    });
    ping.emit(21);
}
";

const EXPECTED: &str = "50\n200\n1\n2\n2\nhello, world\n42\n";

#[test]
fn the_events_of_a_member_belong_to_the_binding() {
    let output = eval_vm_bundled(PROGRAM, VmOptions::default()).unwrap();
    assert_eq!(output.stdout, EXPECTED);
}

#[test]
fn compiled_code_runs_member_events_like_the_vm() {
    let compiled = eval_run(PROGRAM).unwrap();
    let virtual_run = eval_vm_bundled(PROGRAM, VmOptions::default()).unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(compiled.stdout, virtual_run.stdout);
    assert_eq!(compiled.stdout, EXPECTED);
}

#[test]
fn a_handler_must_match_the_parameters_of_the_event() {
    let program = "import \"intolang://builtin/std\";

pub HttpEvents() {
    pub event progress(percent: Int32) {}
}

pub Client() {
    pub into on = HttpEvents();
}

init {
    into client = Client();
    client.on.progress((a, b) => {
        println(a);
    });
}
";
    let message = eval_vm_bundled(program, VmOptions::default())
        .unwrap_err()
        .to_string();
    assert!(
        message.contains("takes 1 parameter(s) but the lambda declares 2"),
        "{message}"
    );
}

#[test]
fn a_destroyed_member_event_cannot_be_used_again() {
    let program = "import \"intolang://builtin/std\";

pub HttpEvents() {
    pub event progress(percent: Int32) {}
}

pub Client() {
    pub into on = HttpEvents();
}

init {
    into client = Client();
    client.on.progress.destroy();
    client.on.progress.emit(1);
}
";
    let message = eval_vm_bundled(program, VmOptions::default())
        .unwrap_err()
        .to_string();
    assert!(message.contains("was destroyed"), "{message}");
}
