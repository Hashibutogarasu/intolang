/// A function exposed from a native library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeFunction {
    pub name: String,
    pub output: String,
}
