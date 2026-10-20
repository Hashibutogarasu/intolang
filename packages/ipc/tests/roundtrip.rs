use std::io::Cursor;
use std::thread;

use ipc::{
    Client, Diagnostic, DocumentChange, DocumentOpen, DocumentQuery, Handler, InitializeParams,
    InitializeResult, IpcError, Lines, Location, Message, Notification, Position, Range, Reply,
    Request, Severity, Transport, memory_pair, serve,
};

struct Echo;

impl Handler for Echo {
    fn handle(&mut self, request: Request) -> Result<Reply, IpcError> {
        match request {
            Request::Initialize(params) => Ok(Reply::Initialized(InitializeResult {
                protocol_version: params.protocol_version,
                name: "echo".into(),
                version: "0".into(),
            })),
            Request::Definition(query) => Ok(Reply::Definition(vec![Location {
                uri: query.uri,
                range: Range::new(query.position, query.position),
            }])),
            Request::Shutdown => Ok(Reply::Empty),
            _ => Err(IpcError::new(IpcError::INVALID_REQUEST, "unsupported")),
        }
    }

    fn notify(&mut self, notification: Notification) -> Vec<Notification> {
        match notification {
            Notification::DocumentOpen(DocumentOpen { uri, version, .. })
            | Notification::DocumentChange(DocumentChange { uri, version, .. }) => {
                vec![Notification::Diagnostics {
                    uri,
                    version: Some(version),
                    diagnostics: vec![Diagnostic {
                        range: Range::default(),
                        severity: Severity::Error,
                        code: Some(1),
                        message: "bad".into(),
                        related: vec![],
                        expected: vec![],
                        found: None,
                        hint: None,
                        unnecessary: false,
                    }],
                }]
            }
            _ => vec![],
        }
    }
}

fn query(uri: &str) -> DocumentQuery {
    DocumentQuery {
        uri: uri.into(),
        position: Position::new(2, 3),
    }
}

#[test]
fn a_request_gets_its_reply_and_notifications_are_queued() {
    let (server_side, client_side) = memory_pair();
    let server = thread::spawn(move || {
        let mut transport = server_side;
        serve(&mut Echo, &mut transport).unwrap();
    });
    let mut client = Client::new(client_side);

    let reply = client
        .request(Request::Initialize(InitializeParams {
            protocol_version: ipc::PROTOCOL_VERSION,
            root_uri: None,
            packages_dir: None,
        }))
        .unwrap()
        .unwrap();
    assert!(matches!(reply, Reply::Initialized(ref result) if result.name == "echo"));

    client
        .notify(Notification::DocumentOpen(DocumentOpen {
            uri: "file:///a.into".into(),
            version: 1,
            text: String::new(),
        }))
        .unwrap();
    let reply = client
        .request(Request::Definition(query("file:///a.into")))
        .unwrap()
        .unwrap();
    assert_eq!(
        reply,
        Reply::Definition(vec![Location {
            uri: "file:///a.into".into(),
            range: Range::new(Position::new(2, 3), Position::new(2, 3)),
        }])
    );
    let notification = client.next_notification().unwrap().unwrap();
    assert!(matches!(
        notification,
        Notification::Diagnostics {
            version: Some(1),
            ..
        }
    ));

    let error = client
        .request(Request::Hover(query("file:///a.into")))
        .unwrap()
        .unwrap_err();
    assert_eq!(error.code, IpcError::INVALID_REQUEST);

    client.notify(Notification::Exit).unwrap();
    server.join().unwrap();
}

#[test]
fn lines_transport_writes_one_json_object_per_line() {
    let mut written = Vec::new();
    let message = Message::Request {
        id: 7,
        request: Request::Definition(query("file:///a.into")),
    };
    Lines::new(Cursor::new(Vec::new()), &mut written)
        .send(&message)
        .unwrap();
    let text = String::from_utf8(written).unwrap();
    assert_eq!(text.matches('\n').count(), 1);

    let mut reader = Lines::new(Cursor::new(text.into_bytes()), Vec::new());
    assert_eq!(reader.receive().unwrap(), Some(message));
    assert_eq!(reader.receive().unwrap(), None);
}

#[test]
fn every_message_kind_survives_json() {
    let messages = vec![
        Message::Response {
            id: 1,
            result: Some(Reply::Hover(None)),
            error: None,
        },
        Message::Response {
            id: 2,
            result: None,
            error: Some(IpcError::new(IpcError::INTERNAL, "boom")),
        },
        Message::Notification {
            notification: Notification::Exit,
        },
        Message::Request {
            id: 3,
            request: Request::Shutdown,
        },
    ];
    for message in messages {
        let text = serde_json::to_string(&message).unwrap();
        assert_eq!(serde_json::from_str::<Message>(&text).unwrap(), message);
    }
}

#[test]
fn malformed_input_is_invalid_data() {
    let mut reader = Lines::new(Cursor::new(b"not json\n".to_vec()), Vec::new());
    let error = reader.receive().unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
}
