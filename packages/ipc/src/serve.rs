use std::io;

use crate::handler::Handler;
use crate::message::{Message, Notification};
use crate::transport::Transport;

/// Answers requests until the peer sends `exit` or closes the connection.
pub fn serve<H: Handler, T: Transport>(handler: &mut H, transport: &mut T) -> io::Result<()> {
    while let Some(message) = transport.receive()? {
        match message {
            Message::Request { id, request } => {
                let response = match handler.handle(request) {
                    Ok(reply) => Message::Response {
                        id,
                        result: Some(reply),
                        error: None,
                    },
                    Err(error) => Message::Response {
                        id,
                        result: None,
                        error: Some(error),
                    },
                };
                transport.send(&response)?;
            }
            Message::Notification { notification } => {
                if notification == Notification::Exit {
                    return Ok(());
                }
                for outgoing in handler.notify(notification) {
                    transport.send(&Message::Notification {
                        notification: outgoing,
                    })?;
                }
            }
            Message::Response { .. } => {}
        }
    }
    Ok(())
}
