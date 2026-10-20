use serde::{Deserialize, Serialize};

use crate::catalog::CatalogEntry;

/// Which side raised an error: the Into program or the toolchain itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Domain {
    IntoLang,
    Compiler,
}

macro_rules! declare_errors {
    (
        $(
            $domain:ident {
                $(
                    $(#[$meta:meta])*
                    $name:ident $({ $($field:ident : $fty:ty),+ $(,)? })? = $code:literal,
                )*
            }
        )*
    ) => {
        /// Every error of the toolchain with its code, message and domain.
        #[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
        #[repr(u16)]
        pub enum Errors {
            $($(
                $(#[$meta])*
                $name $({ $($field : $fty),+ })? = $code,
            )*)*
            #[error("Unknown error")]
            Unknown = 0xFFFF,
        }

        impl Errors {
            /// The code of the error; `Unknown` has none.
            pub const fn code(&self) -> Option<u16> {
                match self {
                    $($(Self::$name { .. } => Some($code),)*)*
                    Self::Unknown => None,
                }
            }

            pub const fn domain(&self) -> Domain {
                match self {
                    $($(Self::$name { .. } => Domain::$domain,)*)*
                    Self::Unknown => Domain::Compiler,
                }
            }
        }

        pub(crate) const ENTRIES: &[CatalogEntry] = &[
            $($(
                CatalogEntry {
                    code: $code,
                    domain: Domain::$domain,
                    name: stringify!($name),
                },
            )*)*
        ];
    };
}

declare_errors! {
    IntoLang {
        #[error("{message}")]
        Syntax { message: String } = 0x0001,
        #[error("`{text}` is not {expected}")]
        NotLexeme { text: String, expected: String } = 0x0002,
        #[error("Unsupported {kind} `{text}`")]
        Unsupported { kind: String, text: String } = 0x0003,
        #[error("Expected a type name")]
        ExpectedTypeName = 0x0004,
        #[error("{construct} needs {needs}")]
        TypeNeeds { construct: String, needs: String } = 0x0005,
        #[error("Type arguments need a type name before `<`")]
        TypeArgumentsNeedName = 0x0006,
        #[error("A character literal holds exactly one character: '{raw}'")]
        InvalidCharLiteral { raw: String } = 0x0007,
        #[error("Unterminated string at line {line}")]
        UnterminatedString { line: usize } = 0x0008,
        #[error("Unterminated character literal at line {line}")]
        UnterminatedChar { line: usize } = 0x0009,
        #[error("link attribute expects language, source path, and target function (line {line})")]
        LinkAttributeArguments { line: usize } = 0x000A,
        #[error("Native source path must remain inside the imported package: {path}")]
        NativeSourceOutside { path: String } = 0x000B,
        #[error("Link target function cannot be empty")]
        LinkTargetEmpty = 0x000C,
        #[error("{message}")]
        Semantic { message: String } = 0x1001,
        #[error("{message}")]
        Ownership { message: String } = 0x2001,
        #[error("{message}")]
        Runtime { message: String } = 0x6001,
    }
    Compiler {
        #[error("{message}")]
        Package { message: String } = 0x8001,
        #[error("`{text}` is not a package hash")]
        PackageHashInvalid { text: String } = 0x8002,
        #[error("Invalid package alias `{text}`")]
        PackageAliasInvalid { text: String } = 0x8003,
        #[error("Could not find project manifest: {path}: {reason}")]
        ManifestNotFound { path: String, reason: String } = 0x8004,
        #[error("Invalid project manifest: {path}: {reason}")]
        ManifestInvalid { path: String, reason: String } = 0x8005,
        #[error("A project manifest must be a JSON object")]
        ManifestNotObject = 0x8006,
        #[error("Project manifest `{field}` must be an object")]
        ManifestFieldNotObject { field: String } = 0x8007,
        #[error("Could not find package directory: {path}: {reason}")]
        PackageDirectoryNotFound { path: String, reason: String } = 0x8008,
        #[error("Invalid package manifest: {path}: {reason}")]
        PackageManifestInvalid { path: String, reason: String } = 0x8009,
        #[error("Package manifest must define `{key}`")]
        PackageManifestMissing { key: String } = 0x800A,
        #[error("The compiler does not embed the `{alias}` package")]
        PackageNotEmbedded { alias: String } = 0x800B,
        #[error("Package `{alias}` is not installed")]
        PackageNotInstalled { alias: String } = 0x800C,
        #[error("No {file} found; install a package with `into install <package>` first")]
        LockMissing { file: String } = 0x800D,
        #[error("Lock entry for `{alias}` has no location")]
        LockEntryNoLocation { alias: String } = 0x800E,
        #[error("Lock entry for `{alias}` has no hash")]
        LockEntryNoHash { alias: String } = 0x800F,
        #[error("Package `{alias}` is locked at version {locked} but the compiler provides version {provided}")]
        PackageVersionLocked { alias: String, locked: String, provided: String } = 0x8010,
        #[error("Could not find package `{alias}` at {path}: {reason}")]
        PackageLocationMissing { alias: String, path: String, reason: String } = 0x8011,
        #[error("Package `{alias}` at {path} does not match the lock file (expected {expected}, found {found})")]
        PackageHashMismatch { alias: String, path: String, expected: String, found: String } = 0x8012,
        #[error("Could not find project directory: {path}: {reason}")]
        ProjectDirectoryNotFound { path: String, reason: String } = 0x8013,
        #[error("Invalid export path in into-info.into: {path}")]
        ExportPathInvalid { path: String } = 0x8014,
        #[error("Package `{alias}` has no into-info.into")]
        PackageNoInfo { alias: String } = 0x8015,
        #[error("Package `{alias}` does not export module `{module}`")]
        ModuleNotExported { alias: String, module: String } = 0x8016,
        #[error("Unsupported lock file version {version}")]
        LockVersionUnsupported { version: u32 } = 0x8017,
        #[error("Invalid lock file: {path}: {reason}")]
        LockInvalid { path: String, reason: String } = 0x8018,
        #[error("`{uri}` is not a module URI")]
        NotModuleUri { uri: String } = 0x8019,
        #[error("`{uri}` does not name an export")]
        ExportNameMissing { uri: String } = 0x801A,
        #[error("`{uri}` does not name a module")]
        ImportNameMissing { uri: String } = 0x801B,
        #[error("Library entry has no parent")]
        LibraryNoParent = 0x801C,
        #[error("Library entry exports no public functions")]
        LibraryNoPublicFunctions = 0x801D,
        #[error("Invalid library export URI: {uri}")]
        LibraryExportUriInvalid { uri: String } = 0x801E,
        #[error("Invalid library export module: {uri}")]
        LibraryExportModuleInvalid { uri: String } = 0x801F,
        #[error("Library entry does not export any modules")]
        LibraryNoExports = 0x8020,
        #[error("{message}")]
        Native { message: String } = 0x9001,
        #[error("Malformed {kind} record")]
        MalformedRecord { kind: String } = 0x9002,
        #[error("Malformed function parameter `{field}`")]
        MalformedParameter { field: String } = 0x9003,
        #[error("Unknown binding property `{name}`")]
        UnknownProperty { name: String } = 0x9004,
        #[error("Malformed native field argument `{text}`")]
        MalformedFieldArgument { text: String } = 0x9005,
        #[error("Unsupported native link argument type `{kind}`")]
        UnsupportedLinkArgument { kind: String } = 0x9006,
        #[error("Unknown storage `{name}`")]
        UnknownStorage { name: String } = 0x9007,
        #[error("Unknown ownership change `{name}`")]
        UnknownOwnershipChange { name: String } = 0x9008,
        #[error("Native source {path} is outside its package")]
        NativeSourceOutsidePackage { path: String } = 0x9009,
        #[error("Linked native source does not exist: {path}")]
        NativeSourceMissing { path: String } = 0x900A,
        #[error("pkg-config returned no library directory for `{library}`")]
        PkgConfigNoDirectory { library: String } = 0x900B,
        #[error("Could not run {tool}: {reason}")]
        ToolNotRun { tool: String, reason: String } = 0x900C,
        #[error("pkg-config failed for native dependencies {dependencies}")]
        PkgConfigFailed { dependencies: String } = 0x900D,
        #[error("Native module compilation failed for {path}")]
        NativeModuleCompilationFailed { path: String } = 0x900E,
        #[error("Native source compilation failed for {path}")]
        NativeSourceCompilationFailed { path: String } = 0x900F,
        #[error("Linking the native library failed")]
        NativeLibraryLinkFailed = 0x9010,
        #[error("Embedding the bundle failed")]
        BundleEmbedFailed = 0x9011,
        #[error("{message}")]
        Codegen { message: String } = 0xA001,
        #[error("{message}")]
        Io { message: String } = 0xB001,
        #[error("{message}")]
        Json { message: String } = 0xB005,
        #[error("Could not read {path}: {reason}")]
        Read { path: String, reason: String } = 0xB002,
        #[error("Could not write {path}: {reason}")]
        Write { path: String, reason: String } = 0xB003,
        #[error("Could not {action} {path}: {reason}")]
        FileAction { action: String, path: String, reason: String } = 0xB004,
        #[error("A bundle path cannot be empty")]
        BundlePathEmpty = 0xB101,
        #[error("`{path}` is absolute; a bundle path is relative")]
        BundlePathAbsolute { path: String } = 0xB102,
        #[error("`{path}` holds a character a bundle path cannot hold")]
        BundlePathCharacter { path: String } = 0xB103,
        #[error("`{path}` has an empty component")]
        BundlePathEmptyComponent { path: String } = 0xB104,
        #[error("`{path}` has the component `{component}`")]
        BundlePathDotComponent { path: String, component: String } = 0xB105,
        #[error("`{path}` is already in the bundle")]
        BundleDuplicate { path: String } = 0xB106,
        #[error("`{path}` is a file, not a directory")]
        BundleNotDirectory { path: String } = 0xB107,
        #[error("`{path}` nests too deep")]
        BundleTooDeep { path: String } = 0xB108,
        #[error("{message}")]
        Internal { message: String } = 0xC001,
        #[error("{message}")]
        Panic { message: String } = 0xC002,
        #[error("Unknown message tag `{tag}`")]
        UnknownMessageTag { tag: String } = 0xC003,
        #[error("Not a directive line")]
        NotDirectiveLine = 0xC004,
        #[error("Invalid native string argument encoding")]
        InvalidHexEncoding = 0xC005,
        #[error("Invalid number: {reason}")]
        InvalidNumber { reason: String } = 0xC006,
    }
}
