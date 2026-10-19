use intovm::{INEX_VERSION, Inex, InexFile, InexHeader, InexSource};
use types::Language;

fn sample() -> Inex {
    Inex {
        header: InexHeader {
            name: "Hello World".to_owned(),
            alias: "hello-world".to_owned(),
            version: "0.1.0".to_owned(),
            generator: "into 0.1.0".to_owned(),
        },
        message: "Hello World!\n\u{1e}NATIVE_ASSIGN\tn\tInt32\tn:1\n".to_owned(),
        sources: vec![
            InexSource {
                alias: "json".to_owned(),
                language: Language::Cpp,
                path: "src/ffi/json value.cpp".to_owned(),
                dependencies: vec!["jsoncpp".to_owned(), "zlib".to_owned()],
            },
            InexSource {
                alias: "ps".to_owned(),
                language: Language::C,
                path: "src/ffi/ps/process.c".to_owned(),
                dependencies: Vec::new(),
            },
        ],
        files: vec![
            InexFile {
                path: "ps/src/ffi/ps/process.h".to_owned(),
                contents: b"int x;\n\n".to_vec(),
            },
            InexFile {
                path: "ps/src/ffi/empty".to_owned(),
                contents: Vec::new(),
            },
        ],
    }
}

fn bytes(inex: &Inex) -> Vec<u8> {
    Vec::from(inex)
}

#[test]
fn what_is_written_is_read_back() {
    let inex = sample();
    assert_eq!(Inex::try_from(bytes(&inex).as_slice()).unwrap(), inex);
}

#[test]
fn the_file_starts_with_the_shebang_and_the_magic() {
    let text = String::from_utf8(bytes(&sample())).unwrap();
    assert!(
        text.starts_with(&format!("#!/usr/bin/env into\nINEX {INEX_VERSION}\n")),
        "{text}"
    );
}

#[test]
fn a_file_without_a_shebang_is_read_too() {
    let written = bytes(&sample());
    let without = &written[b"#!/usr/bin/env into\n".len()..];
    assert_eq!(Inex::try_from(without).unwrap(), sample());
}

#[test]
fn a_broken_magic_is_an_error() {
    let error = Inex::try_from(b"#!/usr/bin/env into\nELF 1\n".as_slice()).unwrap_err();
    assert!(error.to_string().contains("Not an inex file"), "{error}");
    let error = Inex::try_from(b"hello".as_slice()).unwrap_err();
    assert!(error.to_string().contains("Not an inex file"), "{error}");
}

#[test]
fn an_unsupported_version_is_an_error() {
    let text = String::from_utf8(bytes(&sample())).unwrap().replacen(
        &format!("INEX {INEX_VERSION}"),
        "INEX 99",
        1,
    );
    let error = Inex::try_from(text.as_bytes()).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("inex version 99 is not supported"),
        "{error}"
    );
}

#[test]
fn a_truncated_file_is_an_error() {
    let written = bytes(&sample());
    let error = Inex::try_from(&written[..written.len() - 30]).unwrap_err();
    assert!(error.to_string().contains("truncated"), "{error}");
}
