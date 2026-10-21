use serde::{Deserialize, Serialize};

use super::position::Range;
use super::query::Documentation;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompletionKind {
    Keyword,
    Function,
    Class,
    Field,
    Enum,
    Variant,
    Event,
    Macro,
    Variable,
    Module,
    Folder,
    Snippet,
}

/// One candidate. `text` replaces `range`; with `snippet` it uses the snippet
/// grammar (`${1:name}` tab stops and `$0`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionKind,
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(default)]
    pub documentation: Option<Documentation>,
    pub range: Range,
    pub text: String,
    #[serde(default)]
    pub snippet: bool,
}
