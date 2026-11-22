//! Errors raised across pubid-core.
//!
//! Match with a catch-all arm: new variants arrive as flavors and
//! corpus contracts grow.

use std::fmt;

/// Result alias for the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// The crate's error type.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// The PARG engine failed to load or verify an artifact.
    Artifact(String),
    /// Parsing failed: the input is not an identifier of the flavor.
    Parse(parsanol::PargError),
    /// A URN input the flavor's URN grammar rejects.
    Urn(String),
    /// A bound capture the flavor requires is absent or not a string.
    MissingAttribute {
        /// The capture name the flavor needed.
        capture: &'static str,
    },
    /// A hash is not a canonical hash of the flavor.
    InvalidHash {
        /// The offending field or reason.
        detail: String,
    },
    /// A corpus file or row violates the testsuite wire contract.
    Corpus {
        /// The file the row came from (path or embedded name).
        path: String,
        /// What is wrong with it.
        detail: String,
    },
    /// Filesystem access failed.
    Io(std::io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Artifact(detail) => write!(f, "artifact load failed: {detail}"),
            Error::Parse(err) => write!(f, "parse failed: {err}"),
            Error::Urn(detail) => write!(f, "urn rejected: {detail}"),
            Error::MissingAttribute { capture } => {
                write!(f, "bound capture {capture:?} missing or not a string")
            }
            Error::InvalidHash { detail } => write!(f, "invalid hash: {detail}"),
            Error::Corpus { path, detail } => write!(f, "corpus {path}: {detail}"),
            Error::Io(err) => write!(f, "io error: {err}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(err) => Some(err),
            Error::Parse(err) => Some(err),
            _ => None,
        }
    }
}

impl From<parsanol::PargError> for Error {
    fn from(err: parsanol::PargError) -> Self {
        Error::Parse(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}
