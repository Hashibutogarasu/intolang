use ast::Program;

#[test]
fn parses_reference_parameters_and_return_types() {
    let program: Program =
        r#"pub fun pick(target: Process&, count: Int32): Process& { return target; }
"#
        .parse()
        .unwrap();
    let function = program.functions().next().unwrap();
    assert_eq!(function.params[0].ty.as_ref().unwrap().as_str(), "Process&");
    assert_eq!(function.params[1].ty.as_ref().unwrap().as_str(), "Int32");
    assert_eq!(function.return_type.as_ref().unwrap().as_str(), "Process&");
}

#[test]
fn parses_reference_annotations() {
    let program: Program = r#"now into view: Counter& = owner;
"#
    .parse()
    .unwrap();
    assert_eq!(program.items.len(), 1);
}

#[test]
fn keeps_the_consumes_attribute_on_methods() {
    let program: Program = r#"pub Process() {
    #[consumes]
    pub fun detach(): Void {}
}
"#
    .parse()
    .unwrap();
    let class = program.classes().next().unwrap();
    assert_eq!(class.methods[0].attributes[0].name, "consumes");
}
