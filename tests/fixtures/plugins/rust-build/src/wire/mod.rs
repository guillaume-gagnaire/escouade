//! The lines of the protocol, in a folder of their own: their path has a `\` under Windows.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Deserialize)]
pub struct Call {
    pub id: Option<u64>,
    pub method: Option<String>,
}

#[derive(Serialize)]
pub struct Answer {
    jsonrpc: &'static str,
    id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Value>,
}

impl Answer {
    /// The answer to a call; a notification has none.
    pub fn to(call: &Call) -> Option<Self> {
        let id = call.id?;
        let (result, error) = match call.method.as_deref()? {
            "initialize" => (Some(json!({ "protocolVersion": 1, "capabilities": {} })), None),
            "shutdown" => (Some(Value::Null), None),
            "probe/panic" => panic!("the place of this line is in the module"),
            _ => (None, Some(json!({ "code": -32601, "message": "no such method" }))),
        };
        Some(Self { jsonrpc: "2.0", id, result, error })
    }
}
