/// What a path of a bundle is: a file with its size, content hash and mode, or
/// a directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    File { size: u64, hash: String, mode: u32 },
    Directory,
}

impl Entry {
    pub fn is_file(&self) -> bool {
        matches!(self, Entry::File { .. })
    }

    pub fn is_directory(&self) -> bool {
        matches!(self, Entry::Directory)
    }
}
