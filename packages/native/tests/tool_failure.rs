use errors::{Domain, Errors, Layer};
use native::{SharedSource, compile_shared_library};
use std::path::PathBuf;
use std::sync::Mutex;
use types::Language;

/// The tests read and change `CC`, so they run one at a time.
static COMPILER: Mutex<()> = Mutex::new(());

fn scratch(name: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!("into-native-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    directory
}

#[test]
fn a_source_that_does_not_compile_fails_with_the_output_of_the_compiler() {
    let _guard = COMPILER
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let directory = scratch("broken-source");
    let source = directory.join("broken.c");
    std::fs::write(&source, "int broken( {").unwrap();
    let sources = [SharedSource {
        path: source,
        language: Language::C,
        dependencies: Vec::new(),
    }];

    let error = compile_shared_library(&sources, &directory.join("out.so")).unwrap_err();
    std::fs::remove_dir_all(&directory).ok();

    assert!(matches!(
        error.kind(),
        Errors::NativeSourceCompilationFailed { .. }
    ));
    assert_eq!(error.code(), Some(0x900F));
    assert_eq!(error.domain(), Domain::Compiler);
    assert!(error.trace().into.is_none());

    let layers = error.trace().layers();
    let native = layers
        .iter()
        .find(|layer| layer.layer() == Layer::Native)
        .expect("the output of the compiler is the native layer");
    assert!(native.render().contains("broken.c"), "{}", native.render());
}

#[test]
fn a_missing_compiler_is_reported_as_a_tool_that_could_not_run() {
    let _guard = COMPILER
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let directory = scratch("missing-compiler");
    let source = directory.join("empty.c");
    std::fs::write(&source, "").unwrap();
    let sources = [SharedSource {
        path: source,
        language: Language::C,
        dependencies: Vec::new(),
    }];

    let error = temp_env_cc("into-no-such-compiler", || {
        compile_shared_library(&sources, &directory.join("out.so")).unwrap_err()
    });
    std::fs::remove_dir_all(&directory).ok();

    assert!(matches!(error.kind(), Errors::ToolNotRun { .. }));
    assert_eq!(error.code(), Some(0x900C));
}

fn temp_env_cc<T>(compiler: &str, run: impl FnOnce() -> T) -> T {
    let previous = std::env::var("CC").ok();
    unsafe { std::env::set_var("CC", compiler) };
    let result = run();
    match previous {
        Some(value) => unsafe { std::env::set_var("CC", value) },
        None => unsafe { std::env::remove_var("CC") },
    }
    result
}
