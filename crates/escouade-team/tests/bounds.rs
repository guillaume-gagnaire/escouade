//! What a server that lies (or one in between) can make the client read (TS-18): never more than its bounds. An
//! answer whose announced length is past `MAX_REPLY` is refused before a byte of its body is read; one that comes
//! piece by piece is refused as soon as it goes past; a live message past `MAX_LIVE_MESSAGE` closes the connection.
//! An answer within them goes through. Each against a small server of the test on localhost.

use std::time::Duration;

use escouade_team::client::{
    Call, HttpTransport, LiveMessage, Method, Transport, MAX_LIVE_MESSAGE, MAX_REPLY,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

fn get() -> Call {
    Call {
        method: Method::Get,
        path: "/v1/team/stream?since=0".into(),
        content_type: None,
        body: Vec::new(),
        token: Some("t".into()),
    }
}

/// A server that answers each connection with `answer` (its head, then its body as `pieces` of `piece` bytes).
async fn liar(head: &'static str, piece: usize, pieces: usize) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((mut tcp, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = vec![0u8; 4096];
                let _ = tcp.read(&mut buf).await;
                if tcp.write_all(head.as_bytes()).await.is_err() {
                    return;
                }
                let chunk = vec![b'x'; piece];
                for _ in 0..pieces {
                    let framed = if head.contains("chunked") {
                        let mut f = format!("{piece:x}\r\n").into_bytes();
                        f.extend_from_slice(&chunk);
                        f.extend_from_slice(b"\r\n");
                        f
                    } else {
                        chunk.clone()
                    };
                    if tcp.write_all(&framed).await.is_err() {
                        return;
                    }
                }
                if head.contains("chunked") {
                    let _ = tcp.write_all(b"0\r\n\r\n").await;
                }
            });
        }
    });
    format!("http://{addr}")
}

fn transport(base: &str) -> HttpTransport {
    HttpTransport::new(base, reqwest::Client::new(), None, None).unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_answer_past_its_bound_is_refused_and_one_within_goes_through() {
    // Announced as 10 GB: refused before its body.
    let base = liar(
        "HTTP/1.1 200 OK\r\nContent-Type: application/cbor\r\nContent-Length: 10000000000\r\n\r\n",
        1024,
        4,
    )
    .await;
    let refused = tokio::time::timeout(Duration::from_secs(10), transport(&base).call(get()))
        .await
        .expect("refused at once");
    let e = refused.expect_err("an answer of 10 GB");
    assert!(e.to_string().contains("larger than"), "{e}");

    // Without a length, piece by piece past the bound: refused as soon as it goes past.
    let pieces = MAX_REPLY / (1 << 20) + 4;
    let base = liar(
        "HTTP/1.1 200 OK\r\nContent-Type: application/cbor\r\nTransfer-Encoding: chunked\r\n\r\n",
        1 << 20,
        pieces,
    )
    .await;
    let e = tokio::time::timeout(Duration::from_secs(30), transport(&base).call(get()))
        .await
        .expect("an answer")
        .expect_err("an answer past the bound");
    assert!(e.to_string().contains("larger than"), "{e}");

    // A page of 4 MiB: read whole.
    let base = liar(
        "HTTP/1.1 200 OK\r\nContent-Type: application/cbor\r\nTransfer-Encoding: chunked\r\n\r\n",
        1 << 20,
        4,
    )
    .await;
    let reply = transport(&base)
        .call(get())
        .await
        .expect("within the bound");
    assert_eq!((reply.status, reply.body.len()), (200, 4 << 20));
}

/// A WebSocket server that sends one message of each size given, then closes.
async fn live_liar(sizes: Vec<usize>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        use futures_util::SinkExt as _;
        use tokio_tungstenite::tungstenite::Message;
        let (tcp, _) = listener.accept().await.unwrap();
        let mut ws = tokio_tungstenite::accept_async(tcp).await.unwrap();
        for n in sizes {
            if ws.send(Message::Binary(vec![7u8; n].into())).await.is_err() {
                return;
            }
        }
        let _ = ws.close(None).await;
    });
    format!("http://{addr}")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_live_message_past_its_bound_closes_the_connection() {
    let base = live_liar(vec![1 << 20, MAX_LIVE_MESSAGE + 1]).await;
    let mut live = transport(&base)
        .live("/v1/live", "t")
        .await
        .expect("connected");
    match live.next().await {
        Some(Ok(LiveMessage::Binary(b))) => assert_eq!(b.len(), 1 << 20),
        other => panic!("the first message: {other:?}"),
    }
    match tokio::time::timeout(Duration::from_secs(10), live.next())
        .await
        .expect("an end")
    {
        Some(Err(_)) | None => {}
        Some(Ok(LiveMessage::Closed(_))) => {}
        Some(Ok(LiveMessage::Binary(b))) => panic!("a message of {} bytes read", b.len()),
    }
}
