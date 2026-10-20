use errors::{Error, Errors, Result};
use std::fmt;
use std::str::FromStr;

/// The properties of a binding the assertion helpers read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Property {
    InMemory,
    Owned,
    Reference,
    Borrowed,
    Moved,
    Destroyed,
}

impl Property {
    pub const ALL: [Property; 6] = [
        Property::InMemory,
        Property::Owned,
        Property::Reference,
        Property::Borrowed,
        Property::Moved,
        Property::Destroyed,
    ];
}

impl FromStr for Property {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self> {
        Ok(match text {
            "isInMemory" => Property::InMemory,
            "isOwned" => Property::Owned,
            "isReference" => Property::Reference,
            "isBorrowed" => Property::Borrowed,
            "isMoved" => Property::Moved,
            "isDestroyed" => Property::Destroyed,
            other => {
                return Err(Errors::UnknownProperty {
                    name: other.to_string(),
                }
                .into());
            }
        })
    }
}

impl fmt::Display for Property {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Property::InMemory => "isInMemory",
            Property::Owned => "isOwned",
            Property::Reference => "isReference",
            Property::Borrowed => "isBorrowed",
            Property::Moved => "isMoved",
            Property::Destroyed => "isDestroyed",
        })
    }
}
