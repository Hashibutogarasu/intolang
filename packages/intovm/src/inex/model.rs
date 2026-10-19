use types::Language;

/// The identity of the package an `.inex` file was built from.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InexHeader {
    pub name: String,
    pub alias: String,
    pub version: String,
    /// The tool that wrote the file, such as `into 0.1.0`.
    pub generator: String,
}

/// A native source the message calls, which the VM compiles when it runs the
/// file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InexSource {
    pub alias: String,
    pub language: Language,
    /// The path relative to the package root.
    pub path: String,
    /// The `pkg-config` libraries the source needs.
    pub dependencies: Vec<String>,
}

/// A file that travels with the program, such as a header a native source
/// includes. The path starts with the alias of the package that owns it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InexFile {
    pub path: String,
    pub contents: Vec<u8>,
}

/// An Into executable the VM runs: the lowered message and what the native
/// calls in it need.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inex {
    pub header: InexHeader,
    pub message: String,
    pub sources: Vec<InexSource>,
    pub files: Vec<InexFile>,
}
