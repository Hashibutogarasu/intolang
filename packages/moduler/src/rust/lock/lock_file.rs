use errors::{Error, Errors, Result};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;
use std::str::FromStr;

use crate::package::PackageInfo;

const SUPPORTED_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockFile {
    #[serde(rename = "lockfileVersion")]
    pub version: u32,
    pub packages: Vec<PackageInfo>,
}

impl Default for LockFile {
    fn default() -> Self {
        Self {
            version: SUPPORTED_VERSION,
            packages: Vec::new(),
        }
    }
}

impl LockFile {
    pub const FILE_NAME: &'static str = "into-package-lock.json";

    pub fn load(path: &Path) -> Result<Option<Self>> {
        let contents = match std::fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(Errors::read(path, error).into()),
        };
        contents.parse().map(Some).map_err(|error: Error| {
            Errors::LockInvalid {
                path: path.display().to_string(),
                reason: error.kind().to_string(),
            }
            .into()
        })
    }

    pub fn get(&self, alias: &str) -> Option<&PackageInfo> {
        self.packages.iter().find(|package| package.alias == alias)
    }

    pub fn upsert(&mut self, package: PackageInfo) {
        self.packages
            .retain(|existing| existing.alias != package.alias);
        self.packages.push(package);
        self.packages
            .sort_by(|left, right| left.alias.cmp(&right.alias));
    }

    pub fn remove(&mut self, alias: &str) -> Option<PackageInfo> {
        let index = self
            .packages
            .iter()
            .position(|package| package.alias == alias)?;
        Some(self.packages.remove(index))
    }
}

impl FromStr for LockFile {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self> {
        let lock: LockFile = serde_json::from_str(text)?;
        if lock.version != SUPPORTED_VERSION {
            return Err(Errors::LockVersionUnsupported {
                version: lock.version,
            }
            .into());
        }
        Ok(lock)
    }
}

impl fmt::Display for LockFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = serde_json::to_string_pretty(self).map_err(|_| fmt::Error)?;
        writeln!(f, "{text}")
    }
}
