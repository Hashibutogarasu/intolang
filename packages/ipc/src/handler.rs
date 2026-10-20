use crate::message::{IpcError, Notification, Reply, Request};

/// What the compiler side implements. It never sees a transport or an editor.
pub trait Handler {
    fn handle(&mut self, request: Request) -> Result<Reply, IpcError>;

    /// Notifications to send back, such as the diagnostics a change produced.
    fn notify(&mut self, notification: Notification) -> Vec<Notification>;
}
