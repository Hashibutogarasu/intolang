use serde::Deserialize;

const DEFAULT_DIST: &str = "./dist";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct BuiltConfig {
    #[serde(default = "default_dist")]
    pub dist: String,
}

fn default_dist() -> String {
    DEFAULT_DIST.to_owned()
}

impl Default for BuiltConfig {
    fn default() -> Self {
        Self {
            dist: default_dist(),
        }
    }
}
