use errors::{Error, Errors, Result};
use serde_json::{Map, Value};
use std::fmt;
use std::path::Path;
use std::str::FromStr;

use crate::package::PackageAlias;

const DEPENDENCIES: &str = "packages";

pub struct ProjectManifest {
    value: Value,
}

impl ProjectManifest {
    pub fn load(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path).map_err(|error| Errors::ManifestNotFound {
            path: path.display().to_string(),
            reason: error.to_string(),
        })?;
        contents.parse().map_err(|error: Error| {
            Errors::ManifestInvalid {
                path: path.display().to_string(),
                reason: error.kind().to_string(),
            }
            .into()
        })
    }

    fn dependencies(&mut self) -> Result<&mut Map<String, Value>> {
        let Some(manifest) = self.value.as_object_mut() else {
            return Err(Errors::ManifestNotObject.into());
        };
        manifest
            .entry(DEPENDENCIES)
            .or_insert_with(|| Value::Object(Map::new()))
            .as_object_mut()
            .ok_or_else(|| {
                Errors::ManifestFieldNotObject {
                    field: DEPENDENCIES.to_string(),
                }
                .into()
            })
    }

    pub fn set_dependency(&mut self, alias: &PackageAlias, version: &str) -> Result<bool> {
        let dependencies = self.dependencies()?;
        let version = Value::String(version.to_owned());
        let changed = dependencies.get(alias.as_str()) != Some(&version);
        dependencies.insert(alias.to_string(), version);
        Ok(changed)
    }

    pub fn remove_dependency(&mut self, alias: &PackageAlias) -> Result<Option<String>> {
        let removed = self.dependencies()?.remove(alias.as_str());
        Ok(removed.map(|version| version.as_str().unwrap_or_default().to_owned()))
    }
}

impl FromStr for ProjectManifest {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self> {
        Ok(Self {
            value: serde_json::from_str(text)?,
        })
    }
}

impl fmt::Display for ProjectManifest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = serde_json::to_string_pretty(&self.value).map_err(|_| fmt::Error)?;
        writeln!(f, "{text}")
    }
}
