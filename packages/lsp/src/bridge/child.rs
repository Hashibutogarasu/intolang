use std::io::{self, BufReader};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use ipc::{Lines, Message, Transport};

/// The other end of the IPC is a child process that talks on its standard streams.
pub struct ChildTransport {
    child: Child,
    lines: Lines<BufReader<ChildStdout>, ChildStdin>,
}

pub fn spawn_analyzer(program: &str) -> io::Result<ChildTransport> {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    let stdin = child.stdin.take().expect("stdin is piped");
    let stdout = child.stdout.take().expect("stdout is piped");
    Ok(ChildTransport {
        child,
        lines: Lines::new(BufReader::new(stdout), stdin),
    })
}

impl Transport for ChildTransport {
    fn send(&mut self, message: &Message) -> io::Result<()> {
        self.lines.send(message)
    }

    fn receive(&mut self) -> io::Result<Option<Message>> {
        self.lines.receive()
    }
}

impl Drop for ChildTransport {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
