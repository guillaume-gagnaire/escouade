//! What the core keeps of the protocol of `copilot --server --stdio`, for the servers it starts itself for an account
//! (`terminal`): the agents' own are its plugin's (`dev.escouade.copilot-cli`, INT-10.3). Messages framed by
//! `Content-Length` headers, as `plugins::harness` frames them for its plugin.

pub(super) use crate::plugins::harness::read_frame;
use serde_json::Value;

pub(super) use escouade_plugin_copilot::words::{signed_in, INHERITED_TOKENS};

/// Whether the variables of a process give it a folder of its own (an account other than its Principal).
pub(super) fn own_home(env: &[(String, String)]) -> bool {
    env.iter()
        .any(|(k, v)| k == "COPILOT_HOME" && !v.is_empty())
}

/// `msg` framed for Copilot.
pub(super) fn frame(msg: &Value) -> Vec<u8> {
    crate::plugins::harness::framed(msg.to_string().as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn a_frame_is_read_whole_whatever_its_headers() {
        let a = frame(&json!({ "id": 1, "result": "é" }));
        let mut bytes = a.clone();
        bytes.extend_from_slice(b"Content-Type: x\r\ncontent-length: 2\r\n\r\n{}");
        let mut r = tokio::io::BufReader::new(&bytes[..]);
        let first = read_frame(&mut r).await.unwrap().unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&first).unwrap()["result"],
            "é"
        );
        assert_eq!(read_frame(&mut r).await.unwrap().unwrap(), b"{}");
        assert!(read_frame(&mut r).await.unwrap().is_none());
    }
}
