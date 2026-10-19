/// The first line, which lets the file run directly where `into` is installed.
pub const SHEBANG: &str = "#!/usr/bin/env into";

/// The word that starts the second line, followed by the format version.
pub const MAGIC: &str = "INEX";

/// The format version this tool reads and writes.
pub const VERSION: u32 = 1;

pub const NAME: &str = "name";
pub const ALIAS: &str = "alias";
pub const PACKAGE_VERSION: &str = "version";
pub const GENERATOR: &str = "generator";
pub const SOURCE: &str = "source";

/// Header fields are separated by tabs, so paths may contain spaces.
pub const FIELD: char = '\t';

/// The section that carries the lowered message.
pub const MESSAGE: &str = "message";

/// The section that carries a bundled file.
pub const FILE: &str = "file";
