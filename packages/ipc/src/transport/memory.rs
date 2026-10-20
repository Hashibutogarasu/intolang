use std::io;
use std::sync::mpsc::{Receiver, Sender, channel};

use super::transport::Transport;
use crate::message::Message;

pub struct MemoryTransport {
    sender: Sender<Message>,
    receiver: Receiver<Message>,
}

pub fn memory_pair() -> (MemoryTransport, MemoryTransport) {
    let (first_sender, first_receiver) = channel();
    let (second_sender, second_receiver) = channel();
    (
        MemoryTransport {
            sender: first_sender,
            receiver: second_receiver,
        },
        MemoryTransport {
            sender: second_sender,
            receiver: first_receiver,
        },
    )
}

impl Transport for MemoryTransport {
    fn send(&mut self, message: &Message) -> io::Result<()> {
        self.sender
            .send(message.clone())
            .map_err(|_| io::Error::from(io::ErrorKind::BrokenPipe))
    }

    fn receive(&mut self) -> io::Result<Option<Message>> {
        Ok(self.receiver.recv().ok())
    }
}
