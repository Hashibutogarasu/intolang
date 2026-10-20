use std::collections::HashSet;

use errors::{CATALOG, Domain, Errors};

#[test]
fn codes_are_unique() {
    let mut seen = HashSet::new();
    for entry in CATALOG {
        assert!(
            seen.insert(entry.code),
            "{} repeats the code {:#06x}",
            entry.name,
            entry.code
        );
    }
}

#[test]
fn codes_stay_in_the_range_of_their_domain() {
    for entry in CATALOG {
        let in_range = match entry.domain {
            Domain::IntoLang => matches!(entry.code, 0x0001..=0x7FFF),
            Domain::Compiler => matches!(entry.code, 0x8000..=0xFFFE),
        };
        assert!(
            in_range,
            "{} has the code {:#06x} outside of its domain",
            entry.name, entry.code
        );
    }
}

#[test]
fn the_unknown_error_has_no_code() {
    assert_eq!(Errors::Unknown.code(), None);
    assert_eq!(Errors::Unknown.to_string(), "Unknown error");
}

#[test]
fn a_code_is_the_discriminant_of_its_variant() {
    let syntax = Errors::Syntax {
        message: "Expected a type name".into(),
    };
    assert_eq!(syntax.code(), Some(0x0001));
    assert_eq!(syntax.domain(), Domain::IntoLang);
    assert_eq!(syntax.to_string(), "Expected a type name");

    let package = Errors::Package {
        message: "Package `a` is not installed".into(),
    };
    assert_eq!(package.code(), Some(0x8001));
    assert_eq!(package.domain(), Domain::Compiler);
}
