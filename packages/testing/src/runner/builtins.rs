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

pub const ASSERTIONS: [Required; 3] = [
    Required {
        module: "tests/assert",
        function: "assert",
    },
    Required {
        module: "tests/memory-check",
        function: "memoryExists",
    },
    Required {
        module: "tests/memory-check",
        function: "memoryNotExists",
    },
];

/// The macro that declares the attribute marking test functions.
pub const TEST_MACRO: Required = Required {
    module: "tests/macros/test",
    function: "test",
};

/// The assertion function test bodies call.
pub const ASSERT: &str = "assert";

/// The property that reports whether a binding currently occupies memory.
pub const IS_IN_MEMORY: &str = "isInMemory";
