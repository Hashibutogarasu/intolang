mod common;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use common::{Log, initialize, start};
use ipc::{Message, Position, Reply, Request, Transport};
use lsp::{Bridge, BridgeError};

fn query(uri: &str) -> ipc::DocumentQuery {
    ipc::DocumentQuery {
        uri: uri.to_owned(),
        position: Position::default(),
    }
}

#[test]
fn an_analyzer_that_died_is_started_again_with_the_open_documents() {
    let log = Log::default();
    let starts = Arc::new(AtomicUsize::new(0));
    let counted = starts.clone();
    let factory_log = log.clone();
    let mut bridge = Bridge::new(
        move || {
            if counted.fetch_add(1, Ordering::SeqCst) == 0 {
                let (server_side, client_side) = ipc::memory_pair();
                std::thread::spawn(move || {
                    let mut transport = server_side;
                    if let Ok(Some(Message::Request { id, .. })) = transport.receive() {
                        let _ = transport.send(&Message::Response {
                            id,
                            result: Some(Reply::Empty),
                            error: None,
                        });
                    }
                });
                Ok(client_side)
            } else {
                Ok(start(&factory_log).0)
            }
        },
        initialize(),
    );

    bridge.open("file:///a.into".into(), 1, "a".into()).unwrap();
    assert_eq!(starts.load(Ordering::SeqCst), 2);
    assert_eq!(
        *log.opened.lock().unwrap(),
        ["file:///a.into", "file:///a.into"]
    );

    let reply = bridge
        .request(Request::Hover(query("file:///a.into")))
        .unwrap();
    assert!(matches!(reply, Reply::Hover(Some(_))));
}

#[test]
fn an_analyzer_that_cannot_be_started_is_an_error() {
    let attempts = Arc::new(Mutex::new(0));
    let counted = attempts.clone();
    let mut bridge = Bridge::<ipc::MemoryTransport>::new(
        move || {
            *counted.lock().unwrap() += 1;
            Err(std::io::Error::from(std::io::ErrorKind::NotFound))
        },
        initialize(),
    );
    let error = bridge
        .request(Request::Hover(query("file:///a.into")))
        .unwrap_err();
    assert!(matches!(error, BridgeError::Io(_)));
    assert!(*attempts.lock().unwrap() >= 1);
}

#[test]
fn the_analyzer_is_started_by_the_first_use_and_not_by_a_shutdown() {
    let log = Log::default();
    let starts = Arc::new(AtomicUsize::new(0));
    let counted = starts.clone();
    let factory_log = log.clone();
    let mut bridge = Bridge::new(
        move || {
            counted.fetch_add(1, Ordering::SeqCst);
            Ok(start(&factory_log).0)
        },
        initialize(),
    );
    bridge.shutdown();
    assert_eq!(starts.load(Ordering::SeqCst), 0);
    let reply = bridge.request(Request::Shutdown).unwrap();
    assert_eq!(reply, Reply::Empty);
    assert_eq!(starts.load(Ordering::SeqCst), 1);
}
