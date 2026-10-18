use std::path::PathBuf;

/// Metadata describing an Into package, mirroring the `PackageInfo` class.
#[derive(Debug, Clone, Default)]
pub struct PackageInfo {
    pub alias: String,
    pub name: String,
    pub version: String,
    pub licence: String,
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
            root,
        }
    }
}
