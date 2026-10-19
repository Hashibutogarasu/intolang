//! The builtin Into definitions the test runner depends on: the assertion
//! helpers and the `test` macro that marks test functions.

/// The alias of the package that ships the builtin definitions.
pub const BUILTIN_ALIAS: &str = "builtin";

/// Where the modules of the builtin package live.
pub const SOURCE_ROOT: &str = "src/main";

/// A function a test module must import, and the module defining it.
pub struct Required {
    pub module: &'static str,
    pub function: &'static str,
}

/// The assertion every test module must import.
pub const ASSERTION: Required = Required {
    module: "tests/assert",
    function: "assert",
};

/// The macro that declares the attribute marking test functions.
pub const TEST_MACRO: Required = Required {
    module: "tests/macros/test",
    function: "test",
};
