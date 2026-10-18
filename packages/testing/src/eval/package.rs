/// An Into package assembled in memory for a test: a manifest alias and the
/// source files under the package root.
#[derive(Debug, Clone)]
pub struct Package {
    pub alias: String,
    pub files: Vec<(String, String)>,
}

impl Package {
    /// The alias of the package `intolang://builtin/...` imports resolve to.
    pub const BUILTIN_ALIAS: &'static str = "builtin";

    pub fn new(alias: impl Into<String>) -> Self {
        Self {
            alias: alias.into(),
            files: Vec::new(),
        }
    }

    /// Add a file at `path`, relative to the package root.
    pub fn file(mut self, path: impl Into<String>, contents: impl Into<String>) -> Self {
        self.files.push((path.into(), contents.into()));
        self
    }

    /// The `into-package.json` manifest of the package.
    pub fn manifest(&self) -> String {
        format!(
            "{{\"name\":\"{0}\",\"alias\":\"{0}\",\"version\":\"0.1.0\",\"LICENCE\":\"MIT\"}}",
            self.alias
        )
    }

    /// The builtin package exposing the console functions `print` and
    /// `println`, as `import "intolang://builtin/std";` expects.
    pub fn builtin_std() -> Self {
        Self::new(Self::BUILTIN_ALIAS).file(
            "src/main/into-info.into",
            "export \"builtin://std/print\";\nexport \"builtin://std/println\";\n",
        )
    }
}
