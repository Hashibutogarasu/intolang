//! The builtin Into definitions the test runner executes: the assertion
//! helpers and the `test` macro that marks test functions.

/// The alias of the package that ships the builtin definitions.
pub const BUILTIN_ALIAS: &str = "builtin";

/// Where the builtin modules live inside the builtin package.
pub const SOURCE_ROOT: &str = "src/main/builtin";

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

/// The modules defining the assertion helpers. A helper is available to a
/// test once the test module imports it.
pub const HELPER_MODULES: [&str; 3] = [
    "tests/assert",
    "tests/memory-check",
    "tests/ownership-check",
];

/// The macro that declares the attribute marking test functions.
pub const TEST_MACRO: Required = Required {
    module: "tests/macros/test",
    function: "test",
};

/// The properties of a binding the assertion helpers read.
pub const IS_IN_MEMORY: &str = "isInMemory";
pub const IS_OWNED: &str = "isOwned";
pub const IS_REFERENCE: &str = "isReference";
pub const IS_BORROWED: &str = "isBorrowed";
pub const IS_MOVED: &str = "isMoved";
pub const IS_DESTROYED: &str = "isDestroyed";

pub const PROPERTIES: [&str; 6] = [
    IS_IN_MEMORY,
    IS_OWNED,
    IS_REFERENCE,
    IS_BORROWED,
    IS_MOVED,
    IS_DESTROYED,
];

/// The `Object` methods that change what a binding owns.
pub const COPY: &str = "copy";
pub const MOVE: &str = "move";
pub const BORROW: &str = "borrow";
pub const DESTROY: &str = "destroy";
