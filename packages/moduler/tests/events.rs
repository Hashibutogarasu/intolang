use ast::Program;
use moduler::public_items;

#[test]
fn exposes_only_public_events() {
    let program: Program = r#"pub event started(code: Int32) {}
event hidden() {}
"#
    .parse()
    .unwrap();
    let public = public_items(&program);
    assert!(public.contains("started"), "{public:?}");
    assert!(!public.contains("hidden"), "{public:?}");
}
