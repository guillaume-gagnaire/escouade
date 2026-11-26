//! What the gate asks of the connectors: their state and their ceiling, and a flow to a host. RP-4 plugs this trait on
//! the connectors' entry (`escouade-connector-proto`, RP-2); until then the server has none ([`NoConnectors`]), and the
//! tests a fake one in memory.

use crate::lists::Pattern;
use std::future::Future;
use std::pin::Pin;
use tokio::io::{AsyncRead, AsyncWrite};

/// A flow of bytes both ways: a TCP connection, a flow of a connector's channel, a pipe in memory.
pub trait Stream: AsyncRead + AsyncWrite + Unpin + Send + 'static {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send + 'static> Stream for T {}

/// A flow, boxed.
pub type Flow = Box<dyn Stream>;

/// Whether a connector's channel is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reach {
    Up,
    /// Cut since that time (milliseconds since 1970).
    Down {
        since_ms: u64,
    },
}

/// A connector the server knows: paired, connected or not.
#[derive(Debug, Clone)]
pub struct Known {
    pub id: String,
    /// What the user called it (« Acme »).
    pub name: String,
    pub reach: Reach,
    /// Its ceiling, as it sent it at its last connection (empty before its first).
    pub ceiling: Vec<Pattern>,
}

/// What a flow is opened for: the host and port, and the agent and project the server says it is for (the connector
/// writes them in its journal).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ask {
    pub host: String,
    pub port: u16,
    pub agent: String,
    pub project: String,
}

/// Why a connector opened no flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failed {
    /// It refused, with its reason (its ceiling, a resolved address refused…).
    Refused(String),
    /// Its channel is cut.
    Down { since_ms: u64 },
    /// It could not reach the host (refused, timed out, no such name).
    Broken(String),
}

/// A flow being opened.
pub type Opening = Pin<Box<dyn Future<Output = Result<Flow, Failed>> + Send>>;

/// The connectors of a server.
pub trait Connectors: Send + Sync + 'static {
    /// Those it knows, their state and ceiling.
    fn known(&self) -> Vec<Known>;
    /// A flow to `ask` through the connector `id`.
    fn open(&self, id: &str, ask: Ask) -> Opening;
}

/// A server with no connector: before RP-4, or none paired.
pub struct NoConnectors;

impl Connectors for NoConnectors {
    fn known(&self) -> Vec<Known> {
        Vec::new()
    }

    fn open(&self, id: &str, _: Ask) -> Opening {
        let why = format!("no connector {id}");
        Box::pin(async move { Err(Failed::Broken(why)) })
    }
}
