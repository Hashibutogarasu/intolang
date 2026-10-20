use lsp_types::InitializeParams;
use serde_json::Value;

const ANALYZER_VARIABLE: &str = "INTOLANG_IPC";
const DEFAULT_ANALYZER: &str = "intolang-ipc";

/// What the editor tells the server when it starts it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    pub root_uri: Option<String>,
    /// The program that answers the analyzer's side of the IPC.
    pub analyzer: String,
    /// The directory that holds the builtin packages.
    pub packages_dir: Option<String>,
}

impl Options {
    /// `initializationOptions` takes `analyzerPath` and `packagesDir`; the
    /// analyzer is otherwise `INTOLANG_IPC` or `intolang-ipc` on the path.
    #[allow(deprecated)]
    pub fn from_initialize(params: &InitializeParams) -> Self {
        let text = |key: &str| {
            params
                .initialization_options
                .as_ref()
                .and_then(|options| options.get(key))
                .and_then(Value::as_str)
                .map(str::to_owned)
        };
        let root_uri = params
            .workspace_folders
            .as_ref()
            .and_then(|folders| folders.first())
            .map(|folder| folder.uri.as_str().to_owned())
            .or_else(|| params.root_uri.as_ref().map(|uri| uri.as_str().to_owned()));
        Self {
            root_uri,
            analyzer: text("analyzerPath")
                .or_else(|| std::env::var(ANALYZER_VARIABLE).ok())
                .unwrap_or_else(|| DEFAULT_ANALYZER.to_owned()),
            packages_dir: text("packagesDir"),
        }
    }
}
