//! Tauri's `Error` and `Result`, as far as the core meets them: a task that did not finish, a body not written.

use std::fmt;

/// What failed.
#[derive(Debug)]
pub enum Error {
    /// A task of the runtime panicked or was aborted (`async_runtime::JoinHandle`).
    JoinError(tokio::task::JoinError),
    /// A body not written as JSON (`ipc::IpcResponse`).
    Json(serde_json::Error),
    /// Anything else, said in words.
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::JoinError(e) => write!(f, "failed to join a task: {e}"),
            Error::Json(e) => write!(f, "{e}"),
            Error::Other(e) => f.write_str(e),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::JoinError(e) => Some(e),
            Error::Json(e) => Some(e),
            Error::Other(_) => None,
        }
    }
}

impl From<tokio::task::JoinError> for Error {
    fn from(e: tokio::task::JoinError) -> Self {
        Error::JoinError(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Json(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Other(e.to_string())
    }
}
