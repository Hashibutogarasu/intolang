use errors::{Error, Errors, Result};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::fmt;
use std::path::Path;
use std::str::FromStr;

const PREFIX: &str = "sha256-";
const DIGEST_LENGTH: usize = 64;
const EXCLUDED: [&str; 2] = [".git", "dist"];

fn is_excluded(name: &std::ffi::OsStr) -> bool {
    EXCLUDED.iter().any(|excluded| name == *excluded)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageHash(String);

impl PackageHash {
    pub fn of_directory(root: &Path) -> Result<Self> {
        let mut files = Vec::new();
        collect_files(root, "", &mut files)?;
        files.sort();
        let mut digest = Sha256::new();
        for (relative, path) in &files {
            let contents = std::fs::read(path).map_err(|error| Errors::read(path, error))?;
            digest.update(relative.as_bytes());
            digest.update([0]);
            digest.update((contents.len() as u64).to_le_bytes());
            digest.update(&contents);
        }
        let hex = digest
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        Ok(Self(hex))
    }
}

fn collect_files(
    directory: &Path,
    prefix: &str,
    files: &mut Vec<(String, std::path::PathBuf)>,
) -> Result<()> {
    for entry in std::fs::read_dir(directory).map_err(|error| Errors::read(directory, error))? {
        let entry = entry?;
        let name = entry.file_name();
        if is_excluded(&name) {
            continue;
        }
        let relative = format!("{prefix}{}", name.to_string_lossy());
        if entry.file_type()?.is_dir() {
            collect_files(&entry.path(), &format!("{relative}/"), files)?;
        } else {
            files.push((relative, entry.path()));
        }
    }
    Ok(())
}

impl FromStr for PackageHash {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self> {
        let invalid = || Errors::PackageHashInvalid {
            text: text.to_string(),
        };
        let Some(hex) = text.strip_prefix(PREFIX) else {
            return Err(invalid().into());
        };
        if hex.len() != DIGEST_LENGTH || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(invalid().into());
        }
        Ok(Self(hex.to_ascii_lowercase()))
    }
}

impl fmt::Display for PackageHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{PREFIX}{}", self.0)
    }
}

impl Serialize for PackageHash {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for PackageHash {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}
