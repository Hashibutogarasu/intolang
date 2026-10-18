/// A foreign language a native source file is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    C,
    Cpp,
}
