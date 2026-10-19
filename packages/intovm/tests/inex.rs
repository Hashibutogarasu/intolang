use bundle::{Bundle, Entry};
use intovm::{INEX_VERSION, Inex};

fn sample() -> Inex {
    let mut bundle = Bundle::new();
    bundle
        .add_file("into-package.json", r#"{"name":"Hello World"}"#, 0o644)
        .unwrap();
    bundle
        .add_file(
            "intoc",
            "Hello World!\n\u{1e}NATIVE_ASSIGN\tn\tInt32\tn:1\n",
            0o644,
        )
        .unwrap();
    bundle
        .add_file("src/main/hello world.into", "init {}", 0o644)
        .unwrap();
    bundle
        .add_file("res/日本語/a.bin", [0u8, 159, 146, 150], 0o600)
        .unwrap();
    bundle.add_file("res/same.bin", [1u8, 2, 3], 0o644).unwrap();
    bundle
        .add_file("res/again.bin", [1u8, 2, 3], 0o755)
        .unwrap();
    bundle
        .add_file(".into/packages/ps/src/ffi/empty", "", 0o644)
        .unwrap();
    bundle.add_directory("res/empty").unwrap();
    Inex {
        generator: "into 0.1.0".to_owned(),
        bundle,
    }
}

fn small() -> Inex {
    let mut bundle = Bundle::new();
    bundle.add_file("a", "contents", 0o644).unwrap();
    Inex {
        generator: "g".to_owned(),
        bundle,
    }
}

#[test]
fn what_is_written_is_read_back() {
    let inex = sample();
    let read = Inex::try_from(Vec::from(&inex).as_slice()).unwrap();
    assert_eq!(read, inex);
    assert_eq!(read.bundle.get("res/empty"), Some(&Entry::Directory));
    assert_eq!(
        read.bundle.read("res/日本語/a.bin"),
        Some(&[0u8, 159, 146, 150][..])
    );
    assert_eq!(
        read.intoc().unwrap(),
        "Hello World!\n\u{1e}NATIVE_ASSIGN\tn\tInt32\tn:1\n"
    );
}

#[test]
fn equal_contents_are_written_once() {
    let mut bundle = Bundle::new();
    for name in ["one", "two", "three"] {
        bundle.add_file(name, "same contents", 0o644).unwrap();
    }
    let written = Vec::from(&Inex {
        generator: "g".to_owned(),
        bundle,
    });
    let occurrences = written
        .windows(b"same contents".len())
        .filter(|window| *window == b"same contents")
        .count();
    assert_eq!(occurrences, 1);
}

#[test]
fn the_file_starts_with_the_shebang_the_magic_and_the_version() {
    let written = Vec::from(&sample());
    let mut start = b"#!/usr/bin/env into\nINEX".to_vec();
    start.extend_from_slice(&INEX_VERSION.to_le_bytes());
    assert!(written.starts_with(&start));
}

#[test]
fn a_file_without_a_shebang_is_read_too() {
    let written = Vec::from(&sample());
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
    let mut bytes = b"#!/usr/bin/env into\nINEX".to_vec();
    bytes.extend_from_slice(&99u32.to_le_bytes());
    let error = Inex::try_from(bytes.as_slice()).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("inex version 99 is not supported"),
        "{error}"
    );
}

#[test]
fn the_text_format_of_version_one_is_not_read() {
    let error =
        Inex::try_from(b"#!/usr/bin/env into\nINEX 1\nname:\thello\nalias:\thello\n".as_slice())
            .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("inex version 1 is not supported"),
        "{error}"
    );
}

#[test]
fn a_truncated_file_is_an_error() {
    let written = Vec::from(&sample());
    let error = Inex::try_from(&written[..written.len() - 5]).unwrap_err();
    assert!(error.to_string().contains("truncated"), "{error}");
    let error = Inex::try_from(&written[..40]).unwrap_err();
    assert!(error.to_string().contains("truncated"), "{error}");
}

#[test]
fn an_entry_outside_the_data_is_an_error() {
    let mut written = Vec::from(&small());
    let offset = b"#!/usr/bin/env into\nINEX".len() + 4 + 5 + 4 + 5 + 1 + 4 + 8;
    written[offset..offset + 8].copy_from_slice(&u64::MAX.to_le_bytes());
    let error = Inex::try_from(written.as_slice()).unwrap_err();
    assert!(error.to_string().contains("outside its data"), "{error}");
}

#[test]
fn a_changed_content_does_not_match_its_hash() {
    let mut written = Vec::from(&small());
    let last = written.len() - 1;
    written[last] ^= 0xff;
    let error = Inex::try_from(written.as_slice()).unwrap_err();
    assert!(error.to_string().contains("does not match"), "{error}");
}

#[test]
fn a_bundle_without_the_program_has_no_intoc() {
    let error = small().intoc().unwrap_err();
    assert!(error.to_string().contains("inex has no intoc"), "{error}");
}
