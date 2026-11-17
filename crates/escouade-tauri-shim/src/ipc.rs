//! `tauri::ipc`, as far as the core uses it without a window: a channel made by the remote API (`Channel::new`, its
//! messages to a connection), the bodies of an answer, an answer in bytes.

use serde::{Serialize, Serializer};
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// The prefix of a channel's id in JSON, as Tauri's: the window's `Channel` reads it back.
pub const IPC_PAYLOAD_PREFIX: &str = "__CHANNEL__:";

static CHANNEL_COUNTER: AtomicU32 = AtomicU32::new(1);

/// The body of an answer or a channel's message: JSON written already, or bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvokeResponseBody {
    Json(String),
    Raw(Vec<u8>),
}

impl From<String> for InvokeResponseBody {
    fn from(value: String) -> Self {
        Self::Json(value)
    }
}

impl From<Vec<u8>> for InvokeResponseBody {
    fn from(value: Vec<u8>) -> Self {
        Self::Raw(value)
    }
}

impl InvokeResponseBody {
    /// The body read back as `T` (JSON, or the bytes as an array).
    pub fn deserialize<T: serde::de::DeserializeOwned>(self) -> serde_json::Result<T> {
        match self {
            Self::Json(s) => serde_json::from_str(&s),
            Self::Raw(b) => serde_json::from_slice(&b),
        }
    }
}

/// What can be an answer: anything serialized, or a body as it is.
pub trait IpcResponse {
    fn body(self) -> crate::Result<InvokeResponseBody>;
}

impl<T: Serialize> IpcResponse for T {
    fn body(self) -> crate::Result<InvokeResponseBody> {
        serde_json::to_string(&self)
            .map(Into::into)
            .map_err(Into::into)
    }
}

impl IpcResponse for InvokeResponseBody {
    fn body(self) -> crate::Result<InvokeResponseBody> {
        Ok(self)
    }
}

/// An answer given as a body (bytes, most often).
pub struct Response {
    body: InvokeResponseBody,
}

impl IpcResponse for Response {
    fn body(self) -> crate::Result<InvokeResponseBody> {
        Ok(self.body)
    }
}

impl Response {
    pub fn new(body: impl Into<InvokeResponseBody>) -> Self {
        Self { body: body.into() }
    }
}

type OnMessage = Box<dyn Fn(InvokeResponseBody) -> crate::Result<()> + Send + Sync>;

/// A channel: each message sent goes to its handler (the remote API's connection).
pub struct Channel<TSend = InvokeResponseBody> {
    inner: Arc<(u32, OnMessage)>,
    phantom: PhantomData<fn(TSend)>,
}

impl<TSend> Clone for Channel<TSend> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            phantom: PhantomData,
        }
    }
}

impl<TSend> Serialize for Channel<TSend> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("{IPC_PAYLOAD_PREFIX}{}", self.inner.0))
    }
}

impl<TSend> Channel<TSend> {
    /// A channel whose messages go to `on_message`.
    pub fn new<F: Fn(InvokeResponseBody) -> crate::Result<()> + Send + Sync + 'static>(
        on_message: F,
    ) -> Self {
        Self {
            inner: Arc::new((
                CHANNEL_COUNTER.fetch_add(1, Ordering::Relaxed),
                Box::new(on_message),
            )),
            phantom: PhantomData,
        }
    }

    pub fn id(&self) -> u32 {
        self.inner.0
    }

    /// Sends `data` through the channel.
    pub fn send(&self, data: TSend) -> crate::Result<()>
    where
        TSend: IpcResponse,
    {
        (self.inner.1)(data.body()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn a_channel_sends_json_and_bytes_to_its_handler() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let into = seen.clone();
        let channel: Channel<serde_json::Value> = Channel::new(move |b| {
            into.lock().unwrap().push(b);
            Ok(())
        });
        channel.send(serde_json::json!({"a": 1})).unwrap();
        let bytes: Channel<InvokeResponseBody> = Channel::new({
            let into = seen.clone();
            move |b| {
                into.lock().unwrap().push(b);
                Ok(())
            }
        });
        bytes.send(InvokeResponseBody::Raw(vec![1, 2])).unwrap();
        assert_eq!(
            *seen.lock().unwrap(),
            vec![
                InvokeResponseBody::Json("{\"a\":1}".into()),
                InvokeResponseBody::Raw(vec![1, 2])
            ]
        );
        let id = serde_json::to_string(&channel).unwrap();
        assert_eq!(id, format!("\"__CHANNEL__:{}\"", channel.id()));
        assert_ne!(channel.id(), bytes.id());
    }
}
