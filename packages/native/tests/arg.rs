use native::NativeArg;

#[test]
fn displays_the_message_encoding() {
    assert_eq!(NativeArg::from("ab").to_string(), "s:6162");
    assert_eq!(NativeArg::from(-4).to_string(), "i:-4");
    assert_eq!(NativeArg::Slot("count".into()).to_string(), "p:count");
    let field = NativeArg::Field {
        slot: "settings".into(),
        class: "Settings".into(),
        index: 2,
    };
    assert_eq!(field.to_string(), "f:settings:Settings:2");
}

#[test]
fn parses_what_it_displays() {
    for arg in [
        NativeArg::from("héllo\n"),
        NativeArg::from(i32::MAX),
        NativeArg::Slot("__member0".into()),
        NativeArg::Field {
            slot: "settings".into(),
            class: "Settings".into(),
            index: 0,
        },
    ] {
        assert_eq!(arg.to_string().parse::<NativeArg>().unwrap(), arg);
    }
}

#[test]
fn rejects_unknown_kinds_and_bad_values() {
    assert!("x:1".parse::<NativeArg>().is_err());
    assert!("i:one".parse::<NativeArg>().is_err());
    assert!("s:abc".parse::<NativeArg>().is_err());
    assert!("plain".parse::<NativeArg>().is_err());
    assert!("f:slot".parse::<NativeArg>().is_err());
    assert!("f:slot:Class:one".parse::<NativeArg>().is_err());
}
