use testing::eval_vm;

#[test]
fn the_handler_for_the_source_type_runs() {
    let output = eval_vm(
        r#"
import "intolang://builtin/std";

pub Marker() {
    pub into: Int32 seen = 0;

    pub override event onCast<Char, Marker>(from: Char, to: Marker) {
        println(from);
        println("char to marker");
    }

    pub override event onCast<Int32, Marker>(from: Int32, to: Marker) {
        println(from);
        println("int to marker");
    }
}

init {
    into a: Marker = 'A' as Marker;
    into b: Marker = 255 as Marker;
}
"#,
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"A
char to marker
255
int to marker
"#
    );
}

#[test]
fn a_cast_can_be_used_without_an_annotation() {
    let output = eval_vm(
        r#"
import "intolang://builtin/std";

pub Marker() {
    pub into: Int32 seen = 0;

    pub override event onCast<Char, Marker>(from: Char, to: Marker) {
        println(from);
        println("char to marker");
    }

    pub override event onCast<Int32, Marker>(from: Int32, to: Marker) {
        println(from);
        println("int to marker");
    }
}

init {
    into a = 'Z' as Marker;
}
"#,
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"Z
char to marker
"#
    );
}

#[test]
fn a_generic_handler_catches_what_no_concrete_one_does() {
    let output = eval_vm(
        r#"
import "intolang://builtin/std";

pub Wide() {
    pub into: Int32 seen = 0;

    pub override event onCast<Int32, Wide>(from: Int32, to: Wide) {
        println("exact");
    }

    pub override event onCast<From, To>(from: From, to: To) {
        println("fallback");
    }
}

init {
    into a: Wide = 1 as Wide;
    into b: Wide = 'c' as Wide;
}
"#,
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"exact
fallback
"#
    );
}

#[test]
fn a_handler_of_an_ancestor_is_found() {
    let output = eval_vm(
        r#"
import "intolang://builtin/std";

pub Marker() {
    pub into: Int32 seen = 0;

    pub override event onCast<Char, Marker>(from: Char, to: Marker) {
        println(from);
        println("char to marker");
    }

    pub override event onCast<Int32, Marker>(from: Int32, to: Marker) {
        println(from);
        println("int to marker");
    }
}

pub Child(): Marker {
    pub into: Int32 extra = 0;
}

init {
    into a: Child = 'q' as Child;
}
"#,
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"q
char to marker
"#
    );
}

#[test]
fn a_missing_handler_is_a_compile_error() {
    let message = format!(
        "{:#}",
        eval_vm(
            r#"
import "intolang://builtin/std";

pub Marker() {
    pub into: Int32 seen = 0;

    pub override event onCast<Char, Marker>(from: Char, to: Marker) {
        println(from);
        println("char to marker");
    }

    pub override event onCast<Int32, Marker>(from: Int32, to: Marker) {
        println(from);
        println("int to marker");
    }
}

init {
    into s: String = "x";
    into a: Marker = s as Marker;
}
"#,
        )
        .unwrap_err()
    );
    assert!(
        message.contains("no cast from String to Marker"),
        "{message}"
    );
    let message = format!(
        "{:#}",
        eval_vm(
            r#"
import "intolang://builtin/std";

pub Marker() {
    pub into: Int32 seen = 0;

    pub override event onCast<Char, Marker>(from: Char, to: Marker) {
        println(from);
        println("char to marker");
    }

    pub override event onCast<Int32, Marker>(from: Int32, to: Marker) {
        println(from);
        println("int to marker");
    }
}

init {
    into f: Float64 = 1.5;
    into a: Marker = f as Marker;
}
"#,
        )
        .unwrap_err()
    );
    assert!(
        message.contains("no cast from Float64 to Marker"),
        "{message}"
    );
}

#[test]
fn the_built_instance_is_owned_and_destroyed_with_its_scope() {
    let output = eval_vm(
        r#"
import "intolang://builtin/std";

pub Marker() {
    pub into: Int32 seen = 0;

    pub override event onCast<Char, Marker>(from: Char, to: Marker) {
        println(from);
        println("char to marker");
    }

    pub override event onCast<Int32, Marker>(from: Int32, to: Marker) {
        println(from);
        println("int to marker");
    }
}

init {
    into a: Marker = 'A' as Marker;
    into b: Marker = a.copy();
    println("done");
}
"#,
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"A
char to marker
done
"#
    );
}
