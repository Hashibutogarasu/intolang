use std::fmt;

use serde::{Deserialize, Serialize};

use crate::protocol::{
    Diagnostic, DocumentChange, DocumentClose, DocumentOpen, DocumentQuery, Hover,
    InitializeParams, InitializeResult, Location, Symbol, TextEdit,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "method", content = "params")]
pub enum Request {
    #[serde(rename = "initialize")]
    Initialize(InitializeParams),
    #[serde(rename = "shutdown")]
    Shutdown,
    #[serde(rename = "analysis/definition")]
    Definition(DocumentQuery),
    #[serde(rename = "analysis/hover")]
    Hover(DocumentQuery),
    #[serde(rename = "analysis/documentSymbol")]
    DocumentSymbol(DocumentQuery),
    #[serde(rename = "analysis/format")]
    Format(DocumentQuery),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "method", content = "params")]
pub enum Notification {
    #[serde(rename = "document/open")]
    DocumentOpen(DocumentOpen),
    #[serde(rename = "document/change")]
    DocumentChange(DocumentChange),
    #[serde(rename = "document/close")]
    DocumentClose(DocumentClose),
    #[serde(rename = "exit")]
    Exit,
    #[serde(rename = "analysis/diagnostics")]
    Diagnostics {
        uri: String,
        version: Option<i64>,
        diagnostics: Vec<Diagnostic>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum Reply {
    Initialized(InitializeResult),
    Empty,
    Definition(Vec<Location>),
    Hover(Option<Hover>),
    DocumentSymbol(Vec<Symbol>),
    Format(Vec<TextEdit>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IpcError {
    pub code: i32,
    pub message: String,
}

impl IpcError {
    pub const INVALID_REQUEST: i32 = -32600;
    pub const UNKNOWN_DOCUMENT: i32 = -32001;
    pub const INTERNAL: i32 = -32603;

    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl fmt::Display for IpcError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} ({})", self.message, self.code)
    }
}

impl std::error::Error for IpcError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Message {
    Request {
        id: u64,
        #[serde(flatten)]
        request: Request,
    },
    Response {
        id: u64,
        result: Option<Reply>,
        error: Option<IpcError>,
    },
    Notification {
        #[serde(flatten)]
        notification: Notification,
    },
}
