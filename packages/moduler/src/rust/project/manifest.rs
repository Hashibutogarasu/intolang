use anyhow::{Context, Error, Result, bail};
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
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("Could not find project manifest: {}", path.display()))?;
        contents
            .parse()
            .with_context(|| format!("Invalid project manifest: {}", path.display()))
    }

    fn dependencies(&mut self) -> Result<&mut Map<String, Value>> {
        let Some(manifest) = self.value.as_object_mut() else {
            bail!("A project manifest must be a JSON object");
        };
        manifest
            .entry(DEPENDENCIES)
            .or_insert_with(|| Value::Object(Map::new()))
            .as_object_mut()
            .with_context(|| format!("Project manifest `{DEPENDENCIES}` must be an object"))
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
