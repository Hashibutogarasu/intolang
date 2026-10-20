mod client;
mod handler;
mod message;
mod protocol;
mod serve;
mod transport;
mod uri;

pub use client::Client;
pub use handler::Handler;
pub use message::{IpcError, Message, Notification, Reply, Request};
pub use protocol::{
    Diagnostic, DocumentChange, DocumentClose, DocumentOpen, DocumentQuery, Documentation, Hover,
    InitializeParams, InitializeResult, Location, Position, Range, RelatedDiagnostic, Severity,
    Symbol, SymbolKind, TextEdit,
};
pub use serve::serve;
pub use transport::{Lines, MemoryTransport, Transport, memory_pair};
pub use uri::{path_to_uri, uri_to_path};

pub const PROTOCOL_VERSION: u32 = 1;
