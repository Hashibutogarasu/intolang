use anyhow::{Result, anyhow, bail};
use ast::{FunctionDecl, Param};
use parser::LanguageLexeme;
use std::path::{Component, Path};
use types::Language;

/// The foreign implementation a `#[link("c|cpp", "source", "symbol")]`
/// attribute binds a declared function to.
#[derive(Debug, Clone)]
pub struct NativeLink {
    pub language: Language,
    pub package_alias: String,
    pub source_path: String,
    pub target: String,
    pub return_type: String,
    pub params: Vec<Param>,
}

impl NativeLink {
    pub const ATTRIBUTE: &'static str = "link";

    /// Read the link metadata attached to `function`, or `None` when it
    /// carries no `#[link]` attribute.
    pub fn from_function(function: &FunctionDecl, package_alias: &str) -> Result<Option<Self>> {
        let Some(attribute) = function
            .attributes
            .iter()
            .find(|attribute| attribute.name == Self::ATTRIBUTE)
        else {
            return Ok(None);
        };
        let arguments = attribute.string_args().filter(|args| args.len() == 3);
        let Some([language, source, target]) = arguments.as_deref() else {
            bail!(
                "link attribute expects language, source path, and target function (line {})",
                attribute.line
            );
        };
        let language = language
            .parse::<LanguageLexeme>()
            .map_err(|_| anyhow!("Unsupported link language `{language}`"))?;
        let source_path = Path::new(source);
        if source_path.is_absolute()
            || source_path
                .components()
                .any(|component| matches!(component, Component::ParentDir))
        {
            bail!(
                "Native source path must remain inside the imported package: {}",
                source_path.display()
            );
        }
        if target.is_empty() {
            bail!("Link target function cannot be empty");
        }
        Ok(Some(Self {
            language: language.into(),
            package_alias: package_alias.to_owned(),
            source_path: source_path.to_string_lossy().into_owned(),
            target: (*target).to_owned(),
            return_type: function
                .return_type
                .as_ref()
                .map_or_else(|| "Void".to_owned(), ToString::to_string),
            params: function.params.clone(),
        }))
    }
}
