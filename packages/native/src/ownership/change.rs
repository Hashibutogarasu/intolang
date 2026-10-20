use errors::{Error, Errors, Result};
use parser::Record;
use types::{MessageTag, Storage};

/// How a binding was stored, as `NATIVE_OWNERSHIP` carries it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StorageKind(Storage);

impl StorageKind {
    fn name(self) -> &'static str {
        match self.0 {
            Storage::Local => "local",
            Storage::Now => "now",
            Storage::Late => "late",
        }
    }

    fn parse(text: &str) -> Result<Self> {
        Ok(Self(match text {
            "local" => Storage::Local,
            "now" => Storage::Now,
            "late" => Storage::Late,
            other => {
                return Err(Errors::UnknownStorage {
                    name: other.to_string(),
                }
                .into());
            }
        }))
    }
}

/// What `NATIVE_OWNERSHIP` says happened to a binding or to the instance it
/// holds. The backend tracks the same changes statically; the VM tracks them
/// while it runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnershipChange {
    Declare {
        name: String,
        storage: Storage,
    },
    /// An `init` block starts (`true`) or ends (`false`).
    Init(bool),
    Copy {
        target: String,
        source: String,
    },
    Move {
        target: String,
        source: String,
    },
    Borrow {
        target: String,
        source: String,
    },
    Moved(String),
    Destroyed(String),
    Detached(String),
    Unload(String),
}

impl OwnershipChange {
    pub fn to_record(&self) -> Record {
        let record = Record::new(MessageTag::Ownership);
        match self {
            Self::Declare { name, storage } => record
                .field("declare")
                .field(name)
                .field(StorageKind(*storage).name()),
            Self::Init(started) => record.field("init").field(if *started { "1" } else { "0" }),
            Self::Copy { target, source } => record.field("copy").field(target).field(source),
            Self::Move { target, source } => record.field("move").field(target).field(source),
            Self::Borrow { target, source } => record.field("borrow").field(target).field(source),
            Self::Moved(name) => record.field("moved").field(name),
            Self::Destroyed(name) => record.field("destroyed").field(name),
            Self::Detached(name) => record.field("detached").field(name),
            Self::Unload(name) => record.field("unload").field(name),
        }
    }
}

impl TryFrom<&Record> for OwnershipChange {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        let name = || record.get(1).to_owned();
        let source = || record.get(2).to_owned();
        Ok(match record.get(0) {
            "declare" => Self::Declare {
                name: name(),
                storage: StorageKind::parse(record.get(2))?.0,
            },
            "init" => Self::Init(record.get(1) == "1"),
            "copy" => Self::Copy {
                target: name(),
                source: source(),
            },
            "move" => Self::Move {
                target: name(),
                source: source(),
            },
            "borrow" => Self::Borrow {
                target: name(),
                source: source(),
            },
            "moved" => Self::Moved(name()),
            "destroyed" => Self::Destroyed(name()),
            "detached" => Self::Detached(name()),
            "unload" => Self::Unload(name()),
            "" => return Err(Errors::malformed_record("NATIVE_OWNERSHIP").into()),
            other => {
                return Err(Errors::UnknownOwnershipChange {
                    name: other.to_string(),
                }
                .into());
            }
        })
    }
}
