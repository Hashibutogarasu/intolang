use std::collections::VecDeque;
use std::io;

use crate::message::{IpcError, Message, Notification, Reply, Request};
use crate::transport::Transport;

pub struct Client<T: Transport> {
    transport: T,
    next_id: u64,
    pending: VecDeque<Notification>,
}

impl<T: Transport> Client<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            next_id: 1,
            pending: VecDeque::new(),
        }
    }

    /// Sends the request and waits for its response; notifications that arrive meanwhile are queued.
    pub fn request(&mut self, request: Request) -> io::Result<Result<Reply, IpcError>> {
        let id = self.next_id;
        self.next_id += 1;
        self.transport.send(&Message::Request { id, request })?;
        loop {
            match self.transport.receive()? {
                None => return Err(io::Error::from(io::ErrorKind::UnexpectedEof)),
                Some(Message::Response {
                    id: answered,
                    result,
                    error,
                }) if answered == id => {
                    return Ok(match (result, error) {
                        (_, Some(error)) => Err(error),
                        (Some(reply), None) => Ok(reply),
                        (None, None) => Ok(Reply::Empty),
                    });
                }
                Some(Message::Notification { notification }) => {
                    self.pending.push_back(notification)
                }
                Some(_) => {}
            }
        }
    }

    pub fn notify(&mut self, notification: Notification) -> io::Result<()> {
        self.transport.send(&Message::Notification { notification })
    }

    /// The next queued notification, or the next one to arrive; `None` once the peer closed.
    pub fn next_notification(&mut self) -> io::Result<Option<Notification>> {
        if let Some(notification) = self.pending.pop_front() {
            return Ok(Some(notification));
        }
        loop {
            match self.transport.receive()? {
                None => return Ok(None),
                Some(Message::Notification { notification }) => return Ok(Some(notification)),
                Some(_) => {}
            }
        }
    }
}
