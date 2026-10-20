use errors::{Error, Errors, IntoLangStackTrace, Layer, NativeStackTrace, PanicStackTrace};

fn ownership() -> Errors {
    Errors::Ownership {
        message: "`a` was moved".into(),
    }
}

#[test]
fn the_into_language_layer_keeps_the_existing_format() {
    let error = Error::into_lang(
        ownership(),
        IntoLangStackTrace::at("module scope", "<source>", 3),
    );
    let text = error.to_string();
    assert!(
        text.starts_with(
            "`a` was moved\n\nInto language backtrace:\n  0: module scope at <source>:3"
        ),
        "{text}"
    );
}

#[test]
fn layers_are_printed_from_the_into_language_to_the_raw_panic() {
    let error = Error::into_lang(ownership(), IntoLangStackTrace::at("init", "main.into", 7))
        .with_native(NativeStackTrace::new("cc").with_output("undefined reference"))
        .with_panic(PanicStackTrace {
            message: "boom".into(),
            location: Some("lib.rs:1:1".into()),
            backtrace: String::new(),
        });
    let layers: Vec<Layer> = error
        .trace()
        .layers()
        .iter()
        .map(|layer| layer.layer())
        .collect();
    assert!(
        layers.windows(2).all(|pair| pair[0] < pair[1]),
        "{layers:?}"
    );
    assert_eq!(layers.first(), Some(&Layer::Into));
    assert_eq!(layers.last(), Some(&Layer::Panic));
    assert!(layers.contains(&Layer::Native));

    let text = error.to_string();
    let into = text.find("Into language backtrace:").unwrap();
    let native = text.find("Native backtrace:").unwrap();
    let panic = text.find("Panic:").unwrap();
    assert!(into < native && native < panic, "{text}");
    if let Some(rust) = text.find("Rust backtrace:") {
        assert!(into < rust && rust < native, "{text}");
    }
    assert!(text.contains("undefined reference"));
    assert!(text.ends_with("error code: 0x2001"), "{text}");
}

#[test]
fn an_error_of_the_compiler_never_has_an_into_language_layer() {
    let error = Error::into_lang(
        Errors::Native {
            message: "Native source compilation failed".into(),
        },
        IntoLangStackTrace::at("init", "main.into", 7),
    )
    .with_native(NativeStackTrace::new("cc").with_output("error: expected ';'"));
    assert!(error.trace().into.is_none());
    let text = error.to_string();
    assert!(!text.contains("Into language backtrace:"), "{text}");
    assert!(text.contains("Native backtrace:"), "{text}");
}

#[test]
fn an_error_can_be_recovered_from_anyhow() {
    let wrapped: Result<(), _> = Err::<(), _>(Error::new(ownership()));
    let anyhow_like: Box<dyn std::error::Error + Send + Sync> = Box::new(wrapped.unwrap_err());
    let recovered = anyhow_like.downcast_ref::<Error>().unwrap();
    assert_eq!(recovered.code(), Some(0x2001));
}
