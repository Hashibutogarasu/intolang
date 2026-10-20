use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ipc::{
    Diagnostic, DocumentQuery, Handler, InitializeParams, InitializeResult, IpcError, Notification,
    PROTOCOL_VERSION, Reply, Request,
};

use super::analysis::Analysis;
use super::document::Document;
use crate::features::{self, Source};
use crate::text::uri_to_path;
use crate::workspace::{Buffers, Environment};

pub struct Analyzer {
    documents: HashMap<String, Document>,
    environment: Option<Environment>,
}

impl Default for Analyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer {
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
            environment: None,
        }
    }

    fn packages_dir(&self) -> Option<&Path> {
        self.environment.as_ref().map(Environment::path)
    }

    fn buffers(&self) -> Buffers {
        Buffers::new(
            self.documents
                .iter()
                .filter_map(|(uri, document)| Some((uri_to_path(uri)?, document.text.clone())))
                .collect(),
        )
    }

    fn initialize(&mut self, params: InitializeParams) -> Result<Reply, IpcError> {
        if params.protocol_version != PROTOCOL_VERSION {
            return Err(IpcError::new(
                IpcError::INVALID_REQUEST,
                format!(
                    "Protocol version {} is not supported; this is version {PROTOCOL_VERSION}",
                    params.protocol_version
                ),
            ));
        }
        let root = params.root_uri.as_deref().and_then(uri_to_path);
        let builtin = params.packages_dir.map(PathBuf::from);
        self.environment = Environment::build(root.as_deref(), builtin.as_deref());
        Ok(Reply::Initialized(InitializeResult {
            protocol_version: PROTOCOL_VERSION,
            name: "intolang-ipc".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
        }))
    }

    fn update(&mut self, uri: String, text: String, version: i64) -> Notification {
        match self.documents.get_mut(&uri) {
            Some(document) => document.replace(text, version),
            None => {
                self.documents
                    .insert(uri.clone(), Document::new(text, version));
            }
        }
        let sources = self.buffers();
        let packages_dir = self.environment.as_ref().map(Environment::path);
        let document = self.documents.get_mut(&uri).expect("inserted above");
        let path = uri_to_path(&uri);
        let parsed = Analysis::build(&document.text, path.as_deref(), packages_dir, &sources).ok();
        let fresh = parsed.is_some();
        if parsed.is_some() {
            document.analysis = parsed;
        }
        let mut diagnostics = features::diagnose(&Source {
            uri: &uri,
            path: path.as_deref(),
            text: &document.text,
            line_index: &document.line_index,
            packages_dir,
        });
        if let (true, Some(analysis)) = (fresh, &document.analysis) {
            diagnostics.extend(features::unused(
                analysis,
                &document.line_index,
                packages_dir,
            ));
        }
        Notification::Diagnostics {
            uri,
            version: Some(version),
            diagnostics,
        }
    }

    fn document(&self, uri: &str) -> Result<&Document, IpcError> {
        self.documents.get(uri).ok_or_else(|| {
            IpcError::new(
                IpcError::UNKNOWN_DOCUMENT,
                format!("The document {uri} is not open"),
            )
        })
    }

    fn query(&self, query: &DocumentQuery) -> Result<(&Document, usize), IpcError> {
        let document = self.document(&query.uri)?;
        let offset = document.line_index.offset(query.position);
        Ok((document, offset))
    }
}

impl Handler for Analyzer {
    fn handle(&mut self, request: Request) -> Result<Reply, IpcError> {
        match request {
            Request::Initialize(params) => self.initialize(params),
            Request::Shutdown => Ok(Reply::Empty),
            Request::Definition(query) => {
                let (document, offset) = self.query(&query)?;
                Ok(Reply::Definition(match &document.analysis {
                    Some(analysis) => features::definition(
                        analysis,
                        &query.uri,
                        &document.line_index,
                        offset,
                        self.packages_dir(),
                    ),
                    None => Vec::new(),
                }))
            }
            Request::Hover(query) => {
                let (document, offset) = self.query(&query)?;
                Ok(Reply::Hover(document.analysis.as_ref().and_then(
                    |analysis| {
                        features::hover(analysis, &document.line_index, offset, self.packages_dir())
                    },
                )))
            }
            Request::DocumentSymbol(query) => {
                let document = self.document(&query.uri)?;
                Ok(Reply::DocumentSymbol(match &document.analysis {
                    Some(analysis) => features::document_symbols(analysis, &document.line_index),
                    None => Vec::new(),
                }))
            }
            Request::NonOwning(query) => {
                let document = self.document(&query.uri)?;
                Ok(Reply::NonOwning(match &document.analysis {
                    Some(analysis) => features::non_owning(analysis, &document.line_index),
                    None => Vec::new(),
                }))
            }
            Request::Completion(query) => {
                let document = self.document(&query.uri)?;
                let path = uri_to_path(&query.uri);
                let source = Source {
                    uri: &query.uri,
                    path: path.as_deref(),
                    text: &document.text,
                    line_index: &document.line_index,
                    packages_dir: self.packages_dir(),
                };
                Ok(Reply::Completion(features::completion(
                    &source,
                    document.analysis.as_ref(),
                    &self.buffers(),
                    query.position,
                )))
            }
            Request::Format(query) => {
                let document = self.document(&query.uri)?;
                Ok(Reply::Format(features::format(
                    &document.text,
                    &document.line_index,
                )))
            }
        }
    }

    fn notify(&mut self, notification: Notification) -> Vec<Notification> {
        match notification {
            Notification::DocumentOpen(open) => {
                vec![self.update(open.uri, open.text, open.version)]
            }
            Notification::DocumentChange(change) => {
                vec![self.update(change.uri, change.text, change.version)]
            }
            Notification::DocumentClose(close) => {
                self.documents.remove(&close.uri);
                vec![Notification::Diagnostics {
                    uri: close.uri,
                    version: None,
                    diagnostics: Vec::<Diagnostic>::new(),
                }]
            }
            Notification::Exit | Notification::Diagnostics { .. } => Vec::new(),
        }
    }
}
