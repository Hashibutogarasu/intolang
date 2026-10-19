pub const SHEBANG: &[u8] = b"#!/usr/bin/env into\n";

pub const MAGIC: &[u8] = b"INEX";

/// The format version this tool reads and writes.
pub const VERSION: u32 = 2;

/// The file of the bundle that holds the program the VM runs.
pub const INTOC: &str = "intoc";

pub const DIRECTORY: u8 = 0;

pub const FILE: u8 = 1;

pub const DIGEST_LENGTH: usize = 32;
