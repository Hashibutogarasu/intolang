use std::io::{self, BufRead, Write};

use super::transport::Transport;
use crate::message::Message;

/// One JSON message per line, over any reader and writer (stdio, a socket, a child process).
pub struct Lines<R, W> {
    reader: R,
    writer: W,
}

impl<R: BufRead, W: Write> Lines<R, W> {
    pub fn new(reader: R, writer: W) -> Self {
        Self { reader, writer }
    }
}

impl<R: BufRead, W: Write> Transport for Lines<R, W> {
    fn send(&mut self, message: &Message) -> io::Result<()> {
        let line = serde_json::to_string(message).map_err(io::Error::other)?;
        self.writer.write_all(line.as_bytes())?;
        self.writer.write_all(b"\n")?;
        self.writer.flush()
    }

    fn receive(&mut self) -> io::Result<Option<Message>> {
        let mut line = String::new();
        loop {
            line.clear();
            if self.reader.read_line(&mut line)? == 0 {
                return Ok(None);
            }
            if line.trim().is_empty() {
                continue;
            }
            return serde_json::from_str(line.trim())
                .map(Some)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error));
        }
    }
}
