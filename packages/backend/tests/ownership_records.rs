use testing::eval;

#[test]
fn declarations_say_how_the_binding_is_stored() {
    let output = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
}

init {
    now into a = Counter();
    late into b = Counter();
    into c = 1;
}
"#,
    )
    .unwrap();
    assert!(
        output.contains("NATIVE_OWNERSHIP\tdeclare\ta\tnow"),
        "{output}"
    );
    assert!(
        output.contains("NATIVE_OWNERSHIP\tdeclare\tb\tlate"),
        "{output}"
    );
    assert!(
        output.contains("NATIVE_OWNERSHIP\tdeclare\tc\tlocal"),
        "{output}"
    );
}

#[test]
fn init_blocks_are_announced() {
    let output = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
}

init {
    now into a = Counter();
}
"#,
    )
    .unwrap();
    let message = output.message.replace('\u{1e}', "");
    let start = message.find("NATIVE_OWNERSHIP\tinit\t1").unwrap();
    let end = message.find("NATIVE_OWNERSHIP\tinit\t0").unwrap();
    assert!(start < end, "{message}");
}

#[test]
fn copy_move_and_borrow_name_their_target_and_source() {
    let output = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
}

init {
    now into a = Counter();
    now into b = a.copy();
    now into c = a.borrow();
    now into d = b.move();
}
"#,
    )
    .unwrap();
    assert!(output.contains("NATIVE_OWNERSHIP\tcopy\tb\ta"), "{output}");
    assert!(
        output.contains("NATIVE_OWNERSHIP\tborrow\tc\ta"),
        "{output}"
    );
    assert!(output.contains("NATIVE_OWNERSHIP\tmove\td\tb"), "{output}");
    assert!(output.contains("NATIVE_OWNERSHIP\tmoved\tb"), "{output}");
}

#[test]
fn destroy_and_unload_are_recorded() {
    let output = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
}

init {
    now into a = Counter();
    a.destroy();
    now into n = 1;
    unload n;
}
"#,
    )
    .unwrap();
    assert!(
        output.contains("NATIVE_OWNERSHIP\tdestroyed\ta"),
        "{output}"
    );
    assert!(output.contains("NATIVE_OWNERSHIP\tunload\tn"), "{output}");
}

#[test]
fn a_scope_end_records_the_destruction_of_what_it_owns() {
    let output = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
}

init {
    now into a = Counter();
}
"#,
    )
    .unwrap();
    assert!(
        output.contains("NATIVE_OWNERSHIP\tdestroyed\ta"),
        "{output}"
    );
}

#[test]
fn property_conditions_lower_to_state_checks() {
    let output = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
}

init {
    now into a = Counter();
    if (a.isOwned == false) {
        throw "x";
    }
    if (!a.isMoved) {
        throw "y";
    }
}
"#,
    )
    .unwrap();
    assert!(
        output.contains("NATIVE_IF_STATE\ta\tisOwned\t0"),
        "{output}"
    );
    assert!(
        output.contains("NATIVE_IF_STATE\ta\tisMoved\t0"),
        "{output}"
    );
    assert!(output.contains("NATIVE_THROW\t78\n"), "{output}");
    assert!(output.contains("NATIVE_THROW\t79\n"), "{output}");
}

#[test]
fn a_property_compared_with_not_equal_flips_the_expectation() {
    let output = eval(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
}

init {
    now into a = Counter();
    if (a.isOwned != true) {
        throw "x";
    }
}
"#,
    )
    .unwrap();
    assert!(
        output.contains("NATIVE_IF_STATE\ta\tisOwned\t0"),
        "{output}"
    );
}
