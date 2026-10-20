#![allow(dead_code)]

use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use ipc::{
    Diagnostic, Documentation, Handler, Hover, InitializeResult, IpcError, Location,
    MemoryTransport, Notification, Position, Range, Reply, Request, Severity, Symbol, SymbolKind,
    TextEdit, memory_pair, serve,
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
            Request::Format(_) => Ok(Reply::Format(vec![TextEdit {
                range: range(0, 3),
                text: "formatted".into(),
            }])),
        }
    }

    fn notify(&mut self, notification: Notification) -> Vec<Notification> {
        let (uri, version) = match notification {
            Notification::DocumentOpen(open) => {
                self.log.opened.lock().unwrap().push(open.uri.clone());
                (open.uri, Some(open.version))
            }
            Notification::DocumentChange(change) => (change.uri, Some(change.version)),
            Notification::DocumentClose(close) => {
                return vec![Notification::Diagnostics {
                    uri: close.uri,
                    version: None,
                    diagnostics: Vec::new(),
                }];
            }
            _ => return Vec::new(),
        };
        vec![Notification::Diagnostics {
            uri,
            version,
            diagnostics: vec![Diagnostic {
                range: range(1, 2),
                severity: Severity::Error,
                code: Some(0x1234),
                message: "bad".into(),
                related: Vec::new(),
                expected: Vec::new(),
                found: None,
                hint: Some("try again".into()),
            }],
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
