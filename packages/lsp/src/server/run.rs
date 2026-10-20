use std::error::Error;

use ipc::PROTOCOL_VERSION;
use lsp_server::{Connection, Message, Notification, Response};
use lsp_types::InitializeParams;
use serde_json::json;

use super::{Options, Server, capabilities};
use crate::bridge::{Bridge, spawn_analyzer};

/// Serve one editor on the standard streams until it asks to exit.
pub fn run_stdio() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (connection, threads) = Connection::stdio();
    let (id, params) = connection.initialize_start()?;
    let options = Options::from_initialize(&serde_json::from_value::<InitializeParams>(params)?);
    connection.initialize_finish(
        id,
        json!({
            "capabilities": capabilities(),
            "serverInfo": { "name": "into-lsp", "version": env!("CARGO_PKG_VERSION") },
        }),
    )?;

    let analyzer = options.analyzer.clone();
    let bridge = Bridge::new(
        move || spawn_analyzer(&analyzer),
        ipc::InitializeParams {
            protocol_version: PROTOCOL_VERSION,
            root_uri: options.root_uri,
            packages_dir: options.packages_dir,
        },
    );
    let mut server = Server::new(bridge);

    for message in &connection.receiver {
        match message {
            Message::Request(request) => {
                if connection.handle_shutdown(&request)? {
                    break;
                }
                let response = match server.request(&request.method, request.params) {
                    Ok(result) => Response {
                        id: request.id,
                        result: Some(result),
                        error: None,
                    },
                    Err(error) => Response {
                        id: request.id,
                        result: None,
                        error: Some(error),
                    },
                };
                connection.sender.send(Message::Response(response))?;
            }
            Message::Notification(notification) => {
                if notification.method == "exit" {
                    break;
                }
                for (method, params) in
                    server.notification(&notification.method, notification.params)
                {
                    connection
                        .sender
                        .send(Message::Notification(Notification { method, params }))?;
                }
            }
            Message::Response(_) => {}
        }
    }
    server.shutdown();
    drop(connection);
    threads.join()?;
    Ok(())
}
