use errors::{Domain, Error, install_panic_hook, take_panic};

#[test]
fn the_hook_keeps_the_raw_panic_as_the_last_layer() {
    install_panic_hook();
    let outcome = std::panic::catch_unwind(|| {
        panic!("the process was found above");
    });
    assert!(outcome.is_err());

    let panic = take_panic().expect("the hook keeps the panic");
    assert_eq!(panic.message, "the process was found above");
    assert!(panic.location.as_deref().unwrap().contains("panic.rs"));
    assert!(take_panic().is_none());

    let error = Error::from_panic(panic);
    assert_eq!(error.domain(), Domain::Compiler);
    let text = error.to_string();
    assert!(text.contains("Panic:"), "{text}");
    assert!(error.trace().into.is_none());
}
