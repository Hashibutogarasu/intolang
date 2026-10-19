use ast::Program;

const MODULE_SCOPE: &str = r#"
import "intolang://builtin/std";

into mem1 = 0;
print(mem1); // compile error

init {
    print(mem1);
}

now into mem2 = 2;
const now into mem3 = 3;
mem3 = 4;
late into mem4 = 4;
now into mem5 = 5;
unload mem5;
"#;

const CLASSES: &str = r##"
enum Method {
    GET,
}

#[entrypoint]
pub MemoryTest() {
    #[test]
    pub fun memoryWillCreateWithNowModifier() {
        now into mem1 = 0;
        assertMemoryExists(mem1);
        init {
            assertMemoryExists(mem1);
        }
    }
}

pub Window() {
    #[link("cpp", "src/ffi/wayland/window.cpp", "window_create")]
    pub fun create(width: Int32, height: Int32, title: String): Void;

    pub fun present() {
        this.attach();
        this.flush();
    }

    pub fun titlebar(maximizable: Boolean = true, closable: Boolean = true): TitleBar {
        if (maximizable) {
            this.allowMaximize();
        } else {
            this.preventMaximize();
        }
        return this;
    }
}

#[link("c", "src/ffi/client.c", "wayland_display_connect")]
pub c fun wayland_display_connect(name: String?): Pointer;
pub cpp fun std_string_create(value: String): Pointer;
"##;

const BUILTINS: &str = r#"
pub fun print(str: String, params: Object...) {
    Console.write(str);
}

pub fun assert(condition: Boolean) {
    if condition == false {
        throw "assertion failed";
    }
}

pub fun assertMemoryNotExists(value: Object) {
    if value.isInMemory == true {
        throw "memory exists";
    }
}

pub fun assertMemoryExists(value: Object) {
    if value.isInMemory == false {
        throw "memory does not exist";
    }
}

pub macro test {
    #[test]
}
"#;

const LOOPS: &str = r#"
init {
    now into* format = audio.format();
    into reading = 0;
    while (reading >= 0) {
        reading = readAudioFrame(format, packet);
        if (reading >= 0) {
            now into perChannel = produced * channels;
            speaker.queue(pcm, byteCount);
        }
    }
    while (wayland.window.isOpen()) {
        if (video.next()) {
            into text = video.text();
        } else {
            break;
        }
    }
}
"#;

#[test]
fn parses_module_scope_statements() {
    let program: Program = MODULE_SCOPE.parse().unwrap();
    assert_eq!(program.imports().count(), 1);
    assert_eq!(program.statements().count(), 9);
}

#[test]
fn parses_classes_methods_and_native_functions() {
    let program: Program = CLASSES.parse().unwrap();
    assert_eq!(program.enums().count(), 1);
    assert_eq!(program.classes().count(), 2);
    assert_eq!(program.functions().count(), 2);
    let window = program.classes().nth(1).unwrap();
    assert_eq!(window.methods.len(), 3);
}

#[test]
fn parses_builtin_style_functions_and_macros() {
    let program: Program = BUILTINS.parse().unwrap();
    assert_eq!(program.functions().count(), 4);
    assert_eq!(program.macros().count(), 1);
}

#[test]
fn parses_nested_loops_and_branches() {
    let program: Program = LOOPS.parse().unwrap();
    assert_eq!(program.statements().count(), 1);
}
