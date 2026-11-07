//! The loop in process: on the standard input and output, on tokio's threads.

use super::{arrived, called, Cx, Plugin};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

/// Runs `plugin` on `input` and `output` until `input` ends or `shutdown` is answered.
pub async fn serve_on<P, R, W>(plugin: Arc<P>, input: R, mut output: W) -> std::io::Result<()>
where
    P: Plugin,
    R: tokio::io::AsyncRead + Unpin,
    W: AsyncWrite + Unpin + Send + 'static,
{
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let writer = tokio::spawn(async move {
        while let Some(line) = rx.recv().await {
            if output.write_all(line.as_bytes()).await.is_err()
                || output.write_all(b"\n").await.is_err()
                || output.flush().await.is_err()
            {
                break;
            }
        }
    });
    let out = tx.clone();
    let cx = Arc::new(Cx::new(move |line| out.send(line).is_ok()));
    let (stop_tx, mut stop_rx) = mpsc::unbounded_channel::<()>();
    let mut lines = BufReader::new(input).lines();
    loop {
        let line = tokio::select! {
            line = lines.next_line() => line?,
            _ = stop_rx.recv() => break,
        };
        let Some(line) = line else { break };
        let Some(msg) = arrived(&cx, &line) else {
            continue;
        };
        let (plugin, cx, tx, stop) = (plugin.clone(), cx.clone(), tx.clone(), stop_tx.clone());
        tokio::spawn(async move {
            let (out, shut) = called(&*plugin, &cx, msg).await;
            if let Some(out) = out {
                let _ = tx.send(out.to_string());
            }
            if shut {
                let _ = stop.send(());
            }
        });
    }
    drop(tx);
    drop(cx);
    // What was answered is written before the plugin ends.
    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), writer).await;
    Ok(())
}

/// The whole life of a plugin of `escouade-plugins`: its language, then its loop on the standard input and output, on
/// a runtime of its own. Its exit code.
pub fn run<P: Plugin>(name: &str, plugin: P) -> i32 {
    crate::i18n::from_env();
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("escouade-plugins {name}: {e}");
            return 1;
        }
    };
    let result = runtime.block_on(serve_on(
        Arc::new(plugin),
        tokio::io::stdin(),
        tokio::io::stdout(),
    ));
    // A call still running (a slow host) does not keep the process.
    runtime.shutdown_timeout(std::time::Duration::from_millis(200));
    match result {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("escouade-plugins {name}: {e}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::BoxFuture;
    use escouade_plugin_protocol::{code, Refusal};
    use serde_json::{json, Value};

    struct Echo;

    impl Plugin for Echo {
        fn call<'a>(
            &'a self,
            cx: &'a Cx,
            method: &'a str,
            params: Value,
        ) -> BoxFuture<'a, Result<Value, Refusal>> {
            Box::pin(async move {
                match method {
                    "echo" => Ok(params),
                    "slow" => {
                        crate::time::sleep(std::time::Duration::from_millis(300)).await;
                        Ok(json!("slow"))
                    }
                    "secret" => Ok(json!(cx.secret("account:x").await?)),
                    other => Err(Refusal::not_found(other)),
                }
            })
        }
    }

    /// The plugin's lines, as Escouade reads them, and what Escouade writes to it.
    async fn session() -> (
        tokio::io::DuplexStream,
        tokio::io::Lines<BufReader<tokio::io::DuplexStream>>,
    ) {
        let (to_plugin, plugin_in) = tokio::io::duplex(1 << 16);
        let (plugin_out, from_plugin) = tokio::io::duplex(1 << 16);
        tokio::spawn(serve_on(Arc::new(Echo), plugin_in, plugin_out));
        (to_plugin, BufReader::new(from_plugin).lines())
    }

    async fn next(lines: &mut tokio::io::Lines<BufReader<tokio::io::DuplexStream>>) -> Value {
        serde_json::from_str(&lines.next_line().await.unwrap().unwrap()).unwrap()
    }

    #[tokio::test]
    async fn a_slow_call_does_not_hold_back_the_next() {
        let (mut w, mut r) = session().await;
        w.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"slow\"}\n{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"echo\",\"params\":{\"a\":1}}\n")
            .await
            .unwrap();
        let first = next(&mut r).await;
        assert_eq!(first["id"], 2, "{first}");
        assert_eq!(next(&mut r).await["id"], 1);
    }

    #[tokio::test]
    async fn the_plugin_asks_escouade_while_it_answers() {
        let (mut w, mut r) = session().await;
        w.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"secret\"}\n")
            .await
            .unwrap();
        let ask = next(&mut r).await;
        assert_eq!(ask["method"], "secrets/get");
        assert_eq!(ask["params"]["key"], "account:x");
        let reply = json!({ "jsonrpc": "2.0", "id": ask["id"], "result": "ghp_x" });
        w.write_all(format!("{reply}\n").as_bytes()).await.unwrap();
        let done = next(&mut r).await;
        assert_eq!(
            (done["id"].clone(), done["result"].clone()),
            (json!(7), json!("ghp_x"))
        );
    }

    #[tokio::test]
    async fn the_handshake_garbage_and_shutdown() {
        let (mut w, mut r) = session().await;
        w.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{}}\nnot json\n[{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"echo\",\"params\":1},{\"jsonrpc\":\"2.0\",\"id\":4,\"method\":\"nope\"}]\n")
            .await
            .unwrap();
        // Answered as each ends: in any order.
        let mut got = [next(&mut r).await, next(&mut r).await, next(&mut r).await];
        got.sort_by_key(|v| (v.is_array(), v["id"].as_u64()));
        assert_eq!(got[0]["error"]["code"], code::PARSE);
        assert_eq!(got[1]["result"]["protocolVersion"], 1);
        let batch = got[2].clone();
        assert_eq!(batch[0]["result"], 1);
        assert_eq!(batch[1]["error"]["code"], code::METHOD_NOT_FOUND);
        w.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":9,\"method\":\"shutdown\"}\n")
            .await
            .unwrap();
        assert_eq!(next(&mut r).await["id"], 9);
        assert!(
            r.next_line().await.unwrap().is_none(),
            "it ends after shutdown"
        );
    }
}
