use std::collections::HashMap;
use std::fmt;
use std::io;

use ipc::{
    Client, DocumentChange, DocumentClose, DocumentOpen, InitializeParams, IpcError, Notification,
    Reply, Request, Transport,
};

#[derive(Debug)]
pub enum BridgeError {
    Io(io::Error),
    Ipc(IpcError),
}

impl fmt::Display for BridgeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "The analyzer cannot be reached: {error}"),
            Self::Ipc(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for BridgeError {}

impl From<io::Error> for BridgeError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

type Connect<T> = Box<dyn FnMut() -> io::Result<T> + Send>;

/// The connection to the analyzer. It starts the analyzer when it is first
/// needed and starts it again, with the open documents replayed, when it died.
pub struct Bridge<T: Transport> {
    connect: Connect<T>,
    initialize: InitializeParams,
    client: Option<Client<T>>,
    open: HashMap<String, (i64, String)>,
}

impl<T: Transport> Bridge<T> {
    pub fn new(
        connect: impl FnMut() -> io::Result<T> + Send + 'static,
        initialize: InitializeParams,
    ) -> Self {
        Self {
            connect: Box::new(connect),
            initialize,
            client: None,
            open: HashMap::new(),
        }
    }

    pub fn request(&mut self, request: Request) -> Result<Reply, BridgeError> {
        self.attempt(|client| client.request(request.clone()))
    }

    /// The diagnostics the analyzer publishes for the document.
    pub fn open(
        &mut self,
        uri: String,
        version: i64,
        text: String,
    ) -> Result<Vec<Notification>, BridgeError> {
        self.open.insert(uri.clone(), (version, text.clone()));
        self.synchronize(Notification::DocumentOpen(DocumentOpen {
            uri,
            version,
            text,
        }))
    }

    pub fn change(
        &mut self,
        uri: String,
        version: i64,
        text: String,
    ) -> Result<Vec<Notification>, BridgeError> {
        self.open.insert(uri.clone(), (version, text.clone()));
        self.synchronize(Notification::DocumentChange(DocumentChange {
            uri,
            version,
            text,
        }))
    }

    pub fn close(&mut self, uri: String) -> Result<Vec<Notification>, BridgeError> {
        self.open.remove(&uri);
        self.synchronize(Notification::DocumentClose(DocumentClose { uri }))
    }

    pub fn shutdown(&mut self) {
        if let Some(mut client) = self.client.take() {
            let _ = client.request(Request::Shutdown);
            let _ = client.notify(Notification::Exit);
        }
    }

    /// Every document message is answered by one diagnostics notification.
    fn synchronize(
        &mut self,
        notification: Notification,
    ) -> Result<Vec<Notification>, BridgeError> {
        self.attempt(|client| {
            client.notify(notification.clone())?;
            Ok(client.next_notification()?.into_iter().collect::<Vec<_>>())
        })
    }

    fn attempt<R>(
        &mut self,
        mut action: impl FnMut(&mut Client<T>) -> io::Result<R>,
    ) -> Result<R::Output, BridgeError>
    where
        R: Outcome,
    {
        let mut failure = None;
        for _ in 0..2 {
            let client = match self.connected() {
                Ok(client) => client,
                Err(error) => {
                    failure = Some(error);
                    continue;
                }
            };
            match action(client) {
                Ok(outcome) => return outcome.settle(),
                Err(error) => {
                    self.client = None;
                    failure = Some(error);
                }
            }
        }
        Err(BridgeError::Io(failure.unwrap_or_else(|| {
            io::Error::from(io::ErrorKind::NotConnected)
        })))
    }

    fn connected(&mut self) -> io::Result<&mut Client<T>> {
        if self.client.is_none() {
            let mut client = Client::new((self.connect)()?);
            let reply = client.request(Request::Initialize(self.initialize.clone()))?;
            if let Err(error) = reply {
                return Err(io::Error::other(error.to_string()));
            }
            for (uri, (version, text)) in &self.open {
                client.notify(Notification::DocumentOpen(DocumentOpen {
                    uri: uri.clone(),
                    version: *version,
                    text: text.clone(),
                }))?;
                client.next_notification()?;
            }
            self.client = Some(client);
        }
        Ok(self.client.as_mut().expect("connected above"))
    }
}

/// What an action on the client yields once the transport worked: a value, or
/// an error the analyzer answered with.
pub trait Outcome {
    type Output;

    fn settle(self) -> Result<Self::Output, BridgeError>;
}

impl Outcome for Result<Reply, IpcError> {
    type Output = Reply;

    fn settle(self) -> Result<Reply, BridgeError> {
        self.map_err(BridgeError::Ipc)
    }
}

impl Outcome for Vec<Notification> {
    type Output = Vec<Notification>;

    fn settle(self) -> Result<Self::Output, BridgeError> {
        Ok(self)
    }
}
