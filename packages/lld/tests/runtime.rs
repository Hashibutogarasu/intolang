use std::path::Path;

#[test]
fn runtime_directories_hold_the_cxx_runtime_libraries() {
    let directories = lld::runtime_directories("c++");
    for directory in &directories {
        assert!(Path::new(directory).is_absolute(), "{directory}");
        assert!(
            ["libstdc++.so.6", "libgcc_s.so.1"]
                .iter()
                .any(|library| Path::new(directory).join(library).exists()),
            "{directory}"
        );
    }
}

#[test]
fn an_unknown_driver_yields_no_directories() {
    assert!(lld::runtime_directories("no-such-driver-for-intolang").is_empty());
}
