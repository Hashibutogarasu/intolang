use analysis::Analyzer;
use ipc::{
    Diagnostic, DocumentOpen, Handler, InitializeParams, IpcError, Notification, PROTOCOL_VERSION,
    Request,
};

use crate::builtin::{PACKAGES_DIR, builtin_files};

/// The name of the one document a program is, for the analyzer.
const PROGRAM_URI: &str = "file:///playground/main.into";

/// A language server for one editor: the requests and notifications of `ipc`
/// go in as JSON and come out as JSON.
pub struct Session {
    analyzer: Analyzer,
}

impl Session {
    pub fn new() -> Self {
        let mut session = Self {
            analyzer: Analyzer::new(),
        };
        session.within(|analyzer| {
            analyzer
                .handle(Request::Initialize(InitializeParams {
                    protocol_version: PROTOCOL_VERSION,
                    root_uri: None,
                    packages_dir: Some(PACKAGES_DIR.to_owned()),
                }))
                .expect("the analyzer starts");
        });
        session
    }

    /// Answer a request: `{"ok": reply}` or `{"error": {"code", "message"}}`.
    pub fn handle(&mut self, request: &str) -> String {
        let answer = match serde_json::from_str::<Request>(request) {
            Ok(request) => self.within(|analyzer| analyzer.handle(request)),
            Err(error) => Err(IpcError::new(IpcError::INVALID_REQUEST, error.to_string())),
        };
        let json = match answer {
            Ok(reply) => serde_json::json!({ "ok": reply }),
            Err(error) => serde_json::json!({ "error": error }),
        };
        json.to_string()
    }

    /// Take a notification and return the ones it causes, as a JSON array.
    pub fn notify(&mut self, notification: &str) -> String {
        let caused = match serde_json::from_str::<Notification>(notification) {
            Ok(notification) => self.within(|analyzer| analyzer.notify(notification)),
            Err(_) => Vec::new(),
        };
        serde_json::to_string(&caused).expect("notifications are JSON")
    }

    /// What the editor would underline in `text`.
    pub(crate) fn diagnose(&mut self, text: &str) -> Vec<Diagnostic> {
        let notifications = self.within(|analyzer| {
            analyzer.notify(Notification::DocumentOpen(DocumentOpen {
                uri: PROGRAM_URI.to_owned(),
                version: 1,
                text: text.to_owned(),
            }))
        });
        notifications
            .into_iter()
            .flat_map(|notification| match notification {
                Notification::Diagnostics { diagnostics, .. } => diagnostics,
                _ => Vec::new(),
            })
            .collect()
    }

    fn within<T>(&mut self, body: impl FnOnce(&mut Analyzer) -> T) -> T {
        package::with_vfs(builtin_files(), || body(&mut self.analyzer))
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}
