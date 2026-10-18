use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::hash::PackageHash;

/// Metadata describing an Into package, mirroring the `PackageInfo` class.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageInfo {
    pub alias: String,
    pub name: String,
    pub version: String,
    pub licence: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<PackageHash>,
    #[serde(default)]
    pub location: PathBuf,
    #[serde(skip)]
    pub root: PathBuf,
}

impl PackageInfo {
    pub fn new(alias: &str, root: PathBuf, manifest: &serde_json::Value) -> Self {
        let text = |key: &str| {
            manifest
                .get(key)
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_owned()
        };
        Self {
            alias: alias.to_owned(),
            name: text("name"),
            version: text("version"),
            licence: text("LICENCE"),
            hash: None,
            location: PathBuf::new(),
            root,
        }
    }

    pub fn with_hash(mut self, hash: PackageHash) -> Self {
        self.hash = Some(hash);
        self
    }

    pub fn with_location(mut self, location: PathBuf) -> Self {
        self.location = location;
        self
    }
}
