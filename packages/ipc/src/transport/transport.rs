use std::io;

use crate::message::Message;

pub trait Transport {
    fn send(&mut self, message: &Message) -> io::Result<()>;

    /// `None` once the peer closed the connection.
    fn receive(&mut self) -> io::Result<Option<Message>>;
}
