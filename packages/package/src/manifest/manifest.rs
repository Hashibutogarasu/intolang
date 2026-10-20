use errors::{Errors, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;

use super::built_config::BuiltConfig;
use crate::vfs::Vfs;

pub const MANIFEST_FILE: &str = "into-package.json";

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct Manifest {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub alias: Option<String>,
    #[serde(default)]
    pub version: String,
    #[serde(rename = "LICENCE", default)]
    pub licence: String,
    #[serde(rename = "builtConfig", default)]
    pub built_config: BuiltConfig,
    #[serde(default)]
    pub runs: Option<String>,
    #[serde(default)]
    pub library: Option<String>,
    #[serde(default)]
    pub packages: HashMap<String, String>,
    #[serde(rename = "nativeDependencies", default)]
    pub native_dependencies: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub modules: HashMap<String, String>,
}

impl Manifest {
    pub fn load(path: &Path) -> Result<Self> {
        Self::load_in(&*crate::vfs::current(), path)
    }

    pub fn load_in(files: &dyn Vfs, path: &Path) -> Result<Self> {
        let contents =
            files
                .read_to_string(path)
                .map_err(|error| Errors::PackageManifestNotFound {
                    path: path.display().to_string(),
                    reason: error.to_string(),
                })?;
        contents.parse().map_err(|error: serde_json::Error| {
            Errors::PackageManifestInvalid {
                path: path.display().to_string(),
                reason: error.to_string(),
            }
            .into()
        })
    }

    pub fn required_alias(&self) -> Result<&str> {
        self.alias.as_deref().ok_or_else(|| {
            Errors::PackageManifestMissing {
                key: "alias".to_owned(),
            }
            .into()
        })
    }

    pub fn executable_name(&self) -> String {
        self.alias
            .as_deref()
            .unwrap_or(&self.name)
            .replace(' ', "-")
    }

    pub fn dependencies_of(&self, source: &str) -> &[String] {
        self.native_dependencies
            .get(source)
            .map_or(&[], Vec::as_slice)
    }
}

impl FromStr for Manifest {
    type Err = serde_json::Error;

    fn from_str(text: &str) -> std::result::Result<Self, Self::Err> {
        serde_json::from_str(text)
    }
}
