#![allow(dead_code)]

use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use ipc::{
    CompletionItem, CompletionKind, Diagnostic, Documentation, Handler, Hover, InitializeResult,
    IpcError, Location, MemoryTransport, Notification, Position, Range, Reply, Request, Severity,
    Symbol, SymbolKind, TextEdit, memory_pair, serve,
};
use lsp::{Bridge, Server};

pub const URI: &str = "file:///project/src/main/main.into";

#[derive(Default, Clone)]
pub struct Log {
    pub requests: Arc<Mutex<Vec<Request>>>,
    pub opened: Arc<Mutex<Vec<String>>>,
}

pub struct Scripted {
    pub log: Log,
}

fn range(start: u32, end: u32) -> Range {
    Range::new(Position::new(0, start), Position::new(0, end))
}

impl Handler for Scripted {
    fn handle(&mut self, request: Request) -> Result<Reply, IpcError> {
        self.log.requests.lock().unwrap().push(request.clone());
        match request {
            Request::Initialize(_) => Ok(Reply::Initialized(InitializeResult {
                protocol_version: ipc::PROTOCOL_VERSION,
                name: "scripted".into(),
                version: "0".into(),
            })),
            Request::Shutdown => Ok(Reply::Empty),
            Request::Hover(query) if query.position.line == 5 => Ok(Reply::Hover(None)),
            Request::Hover(_) => Ok(Reply::Hover(Some(Hover {
                signature: "pub fun a()".into(),
                documentation: Some(Documentation {
                    markdown: "Does a.".into(),
                }),
                range: range(1, 2),
            }))),
            Request::Definition(query) => Ok(Reply::Definition(vec![Location {
                uri: query.uri,
                range: range(1, 2),
            }])),
            Request::DocumentSymbol(_) => Ok(Reply::DocumentSymbol(vec![
                Symbol {
                    name: "a".into(),
                    kind: SymbolKind::Function,
                    container: Some("Box".into()),
                    range: range(1, 2),
                    name_range: range(1, 2),
                },
                Symbol {
                    name: "Box".into(),
                    kind: SymbolKind::Class,
                    container: None,
                    range: range(0, 3),
                    name_range: range(0, 3),
                },
            ])),
            Request::Completion(_) => Ok(Reply::Completion(vec![
                CompletionItem {
                    label: "alpha".into(),
                    kind: CompletionKind::Function,
                    detail: Some("pub fun alpha()".into()),
                    documentation: None,
                    range: range(0, 1),
                    text: "alpha".into(),
                    snippet: false,
                },
                CompletionItem {
                    label: "deep".into(),
                    kind: CompletionKind::Folder,
                    detail: None,
                    documentation: None,
                    range: range(0, 1),
                    text: "deep/".into(),
                    snippet: false,
                },
                CompletionItem {
                    label: "(a) => { }".into(),
                    kind: CompletionKind::Snippet,
                    detail: None,
                    documentation: None,
                    range: range(0, 0),
                    text: "(${1:a}) => {\n\t$0\n}".into(),
                    snippet: true,
                },
            ])),
            Request::Format(_) => Ok(Reply::Format(vec![TextEdit {
                range: range(0, 3),
                text: "formatted".into(),
            }])),
        }
    }

    fn notify(&mut self, notification: Notification) -> Vec<Notification> {
        let (uri, version, unused) = match notification {
            Notification::DocumentOpen(open) => {
                self.log.opened.lock().unwrap().push(open.uri.clone());
                (open.uri, Some(open.version), open.text.contains("unused"))
            }
            Notification::DocumentChange(change) => (
                change.uri,
                Some(change.version),
                change.text.contains("unused"),
            ),
            Notification::DocumentClose(close) => {
                return vec![Notification::Diagnostics {
                    uri: close.uri,
                    version: None,
                    diagnostics: Vec::new(),
                }];
            }
            _ => return Vec::new(),
        };
        let mut diagnostics = vec![Diagnostic {
            range: range(1, 2),
            severity: Severity::Error,
            code: Some(0x1234),
            message: "bad".into(),
            related: Vec::new(),
            expected: Vec::new(),
            found: None,
            hint: Some("try again".into()),
            unnecessary: false,
        }];
        if unused {
            diagnostics.push(Diagnostic {
                range: range(0, 6),
                severity: Severity::Hint,
                code: None,
                message: "faded".into(),
                related: Vec::new(),
                expected: Vec::new(),
                found: None,
                hint: None,
                unnecessary: true,
            });
            diagnostics.push(Diagnostic {
                range: range(0, 6),
                severity: Severity::Information,
                code: None,
                message: "underlined".into(),
                related: Vec::new(),
                expected: Vec::new(),
                found: None,
                hint: None,
                unnecessary: false,
            });
        }
        vec![Notification::Diagnostics {
            uri,
            version,
            diagnostics,
        }]
    }
}

pub fn initialize() -> ipc::InitializeParams {
    ipc::InitializeParams {
        protocol_version: ipc::PROTOCOL_VERSION,
        root_uri: Some("file:///project".into()),
        packages_dir: None,
    }
}

pub fn start(log: &Log) -> (MemoryTransport, JoinHandle<()>) {
    let (server_side, client_side) = memory_pair();
    let log = log.clone();
    let thread = thread::spawn(move || {
        let mut transport = server_side;
        serve(&mut Scripted { log }, &mut transport).unwrap();
    });
    (client_side, thread)
}

pub fn server(log: &Log) -> Server<MemoryTransport> {
    let log = log.clone();
    let bridge = Bridge::new(move || Ok(start(&log).0), initialize());
    Server::new(bridge)
}
