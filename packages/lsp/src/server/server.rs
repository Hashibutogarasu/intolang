use ipc::{DocumentQuery, IpcError, Notification, Reply, Request, Transport};
use lsp_server::{ErrorCode, ResponseError};
use lsp_types::{
    CompletionParams, CompletionResponse, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DocumentFormattingParams, DocumentSymbolParams,
    DocumentSymbolResponse, GotoDefinitionParams, GotoDefinitionResponse, HoverParams,
    PublishDiagnosticsParams, SemanticTokensParams, SemanticTokensResult,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::bridge::{Bridge, BridgeError};
use crate::convert;
use crate::documents::Documents;

const PUBLISH: &str = "textDocument/publishDiagnostics";

/// The LSP side: it turns what the editor sends into IPC messages and what
/// the analyzer answers into LSP, and knows nothing else about Into.
pub struct Server<T: Transport> {
    bridge: Bridge<T>,
    documents: Documents,
}

impl<T: Transport> Server<T> {
    pub fn new(bridge: Bridge<T>) -> Self {
        Self {
            bridge,
            documents: Documents::default(),
        }
    }

    pub fn shutdown(&mut self) {
        self.bridge.shutdown();
    }

    pub fn request(&mut self, method: &str, params: Value) -> Result<Value, ResponseError> {
        match method {
            "textDocument/hover" => {
                let params: HoverParams = parse(params)?;
                let position = params.text_document_position_params;
                let uri = position.text_document.uri.as_str().to_owned();
                let query = self.query(&uri, position.position);
                let hover = match self.bridge.request(Request::Hover(query)) {
                    Ok(Reply::Hover(hover)) => hover,
                    Ok(_) | Err(BridgeError::Ipc(_)) => None,
                    Err(error) => return Err(failure(error)),
                };
                respond(hover.map(|found| convert::hover(&self.documents, &uri, found)))
            }
            "textDocument/definition" => {
                let params: GotoDefinitionParams = parse(params)?;
                let position = params.text_document_position_params;
                let uri = position.text_document.uri.as_str().to_owned();
                let query = self.query(&uri, position.position);
                let locations = match self.bridge.request(Request::Definition(query)) {
                    Ok(Reply::Definition(locations)) => locations,
                    Ok(_) | Err(BridgeError::Ipc(_)) => Vec::new(),
                    Err(error) => return Err(failure(error)),
                };
                respond(GotoDefinitionResponse::Array(
                    locations
                        .iter()
                        .filter_map(|found| convert::location(&self.documents, found))
                        .collect(),
                ))
            }
            "textDocument/completion" => {
                let params: CompletionParams = parse(params)?;
                let position = params.text_document_position;
                let uri = position.text_document.uri.as_str().to_owned();
                let query = self.query(&uri, position.position);
                let items = match self.bridge.request(Request::Completion(query)) {
                    Ok(Reply::Completion(items)) => items,
                    Ok(_) | Err(BridgeError::Ipc(_)) => Vec::new(),
                    Err(error) => return Err(failure(error)),
                };
                respond(CompletionResponse::Array(
                    items
                        .into_iter()
                        .map(|found| convert::completion(&self.documents, &uri, found))
                        .collect(),
                ))
            }
            "textDocument/semanticTokens/full" => {
                let params: SemanticTokensParams = parse(params)?;
                let uri = params.text_document.uri.as_str().to_owned();
                let query = self.query(&uri, Default::default());
                let ranges = match self.bridge.request(Request::NonOwning(query)) {
                    Ok(Reply::NonOwning(ranges)) => ranges,
                    Ok(_) | Err(BridgeError::Ipc(_)) => Vec::new(),
                    Err(error) => return Err(failure(error)),
                };
                respond(SemanticTokensResult::Tokens(convert::non_owning(
                    &self.documents,
                    &uri,
                    ranges,
                )))
            }
            "textDocument/documentSymbol" => {
                let params: DocumentSymbolParams = parse(params)?;
                let uri = params.text_document.uri.as_str().to_owned();
                let query = self.query(&uri, Default::default());
                let symbols = match self.bridge.request(Request::DocumentSymbol(query)) {
                    Ok(Reply::DocumentSymbol(symbols)) => symbols,
                    Ok(_) | Err(BridgeError::Ipc(_)) => Vec::new(),
                    Err(error) => return Err(failure(error)),
                };
                respond(DocumentSymbolResponse::Flat(
                    symbols
                        .into_iter()
                        .filter_map(|found| convert::symbol(&self.documents, &uri, found))
                        .collect(),
                ))
            }
            "textDocument/formatting" => {
                let params: DocumentFormattingParams = parse(params)?;
                let uri = params.text_document.uri.as_str().to_owned();
                let query = self.query(&uri, Default::default());
                let edits = match self.bridge.request(Request::Format(query)) {
                    Ok(Reply::Format(edits)) => edits,
                    Ok(_) | Err(BridgeError::Ipc(_)) => Vec::new(),
                    Err(error) => return Err(failure(error)),
                };
                respond(
                    edits
                        .into_iter()
                        .map(|found| convert::edit(&self.documents, &uri, found))
                        .collect::<Vec<_>>(),
                )
            }
            other => Err(ResponseError {
                code: ErrorCode::MethodNotFound as i32,
                message: format!("The method {other} is not supported"),
                data: None,
            }),
        }
    }

    /// The notifications to send to the editor, as a method and its parameters.
    pub fn notification(&mut self, method: &str, params: Value) -> Vec<(String, Value)> {
        let answered = match method {
            "textDocument/didOpen" => {
                let Ok(params) = serde_json::from_value::<DidOpenTextDocumentParams>(params) else {
                    return Vec::new();
                };
                let document = params.text_document;
                let uri = document.uri.as_str().to_owned();
                self.documents.set(&uri, document.text.clone());
                self.bridge
                    .open(uri, i64::from(document.version), document.text)
            }
            "textDocument/didChange" => {
                let Ok(params) = serde_json::from_value::<DidChangeTextDocumentParams>(params)
                else {
                    return Vec::new();
                };
                let Some(change) = params.content_changes.into_iter().last() else {
                    return Vec::new();
                };
                let uri = params.text_document.uri.as_str().to_owned();
                self.documents.set(&uri, change.text.clone());
                self.bridge
                    .change(uri, i64::from(params.text_document.version), change.text)
            }
            "textDocument/didClose" => {
                let Ok(params) = serde_json::from_value::<DidCloseTextDocumentParams>(params)
                else {
                    return Vec::new();
                };
                let uri = params.text_document.uri.as_str().to_owned();
                self.documents.remove(&uri);
                self.bridge.close(uri)
            }
            _ => return Vec::new(),
        };
        match answered {
            Ok(notifications) => notifications
                .into_iter()
                .filter_map(|notification| self.publish(notification))
                .collect(),
            Err(error) => {
                eprintln!("into-lsp: {error}");
                Vec::new()
            }
        }
    }

    fn query(&self, uri: &str, position: lsp_types::Position) -> DocumentQuery {
        DocumentQuery {
            uri: uri.to_owned(),
            position: self.documents.analyzer_position(uri, position),
        }
    }

    fn publish(&self, notification: Notification) -> Option<(String, Value)> {
        let Notification::Diagnostics {
            uri,
            version,
            diagnostics,
        } = notification
        else {
            return None;
        };
        let params = PublishDiagnosticsParams {
            uri: convert::uri(&uri)?,
            diagnostics: diagnostics
                .into_iter()
                .map(|found| convert::diagnostic(&self.documents, &uri, found))
                .collect(),
            version: version.and_then(|version| i32::try_from(version).ok()),
        };
        Some((PUBLISH.to_owned(), serde_json::to_value(params).ok()?))
    }
}

fn parse<P: DeserializeOwned>(params: Value) -> Result<P, ResponseError> {
    serde_json::from_value(params).map_err(|error| ResponseError {
        code: ErrorCode::InvalidParams as i32,
        message: error.to_string(),
        data: None,
    })
}

fn respond<V: Serialize>(value: V) -> Result<Value, ResponseError> {
    serde_json::to_value(value).map_err(|error| ResponseError {
        code: ErrorCode::InternalError as i32,
        message: error.to_string(),
        data: None,
    })
}

fn failure(error: BridgeError) -> ResponseError {
    let code = match &error {
        BridgeError::Ipc(IpcError { code, .. }) if *code == IpcError::INVALID_REQUEST => {
            ErrorCode::InvalidRequest
        }
        _ => ErrorCode::InternalError,
    };
    ResponseError {
        code: code as i32,
        message: error.to_string(),
        data: None,
    }
}
