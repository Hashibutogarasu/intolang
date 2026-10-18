/// The alias of the package the compiler embeds for the language itself.
pub const BUILTIN_ALIAS: &str = "builtin";

/// The import scheme that addresses the builtin package.
pub const BUILTIN_SCHEME: &str = "intolang";

/// Whether `alias` names a package that ships inside the compiler binary.
pub fn is_builtin(alias: &str) -> bool {
    alias == BUILTIN_ALIAS || alias == BUILTIN_SCHEME
}
