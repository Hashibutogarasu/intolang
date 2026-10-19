use testing::eval;

const STD: &str = "import \"intolang://builtin/std\";\n";

const COUNTER: &str = "pub Counter() {\n    pub into: Int32 value = 7;\n}\n";

fn lower(body: &str) -> testing::Evaluation {
    eval(&format!("{STD}{COUNTER}init {{\n{body}}}\n")).unwrap()
}

#[test]
fn declarations_say_how_the_binding_is_stored() {
    let output =
        lower("    now into a = Counter();\n    late into b = Counter();\n    into c = 1;\n");
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
    let output = lower("    now into a = Counter();\n");
    let message = output.message.replace('\u{1e}', "");
    let start = message.find("NATIVE_OWNERSHIP\tinit\t1").unwrap();
    let end = message.find("NATIVE_OWNERSHIP\tinit\t0").unwrap();
    assert!(start < end, "{message}");
}

#[test]
fn copy_move_and_borrow_name_their_target_and_source() {
    let output = lower(
        "    now into a = Counter();\n    now into b = a.copy();\n    now into c = a.borrow();\n    now into d = b.move();\n",
    );
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
    let output = lower(
        "    now into a = Counter();\n    a.destroy();\n    now into n = 1;\n    unload n;\n",
    );
    assert!(
        output.contains("NATIVE_OWNERSHIP\tdestroyed\ta"),
        "{output}"
    );
    assert!(output.contains("NATIVE_OWNERSHIP\tunload\tn"), "{output}");
}

#[test]
fn a_scope_end_records_the_destruction_of_what_it_owns() {
    let output = lower("    now into a = Counter();\n");
    assert!(
        output.contains("NATIVE_OWNERSHIP\tdestroyed\ta"),
        "{output}"
    );
}

#[test]
fn property_conditions_lower_to_state_checks() {
    let output = lower(
        "    now into a = Counter();\n    if (a.isOwned == false) {\n        throw \"x\";\n    }\n    if (!a.isMoved) {\n        throw \"y\";\n    }\n",
    );
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
    let output = lower(
        "    now into a = Counter();\n    if (a.isOwned != true) {\n        throw \"x\";\n    }\n",
    );
    assert!(
        output.contains("NATIVE_IF_STATE\ta\tisOwned\t0"),
        "{output}"
    );
}
