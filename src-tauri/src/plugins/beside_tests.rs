//! The rules of what runs beside a plugin's thread, without a network: the pieces of a stream, its credit, the queue of a
//! socket and the methods served here. The rest is played through a module, in `plugins_net_tests.rs`.

use super::*;

#[test]
fn only_the_methods_of_the_host_are_taken_from_the_module() {
    for line in [
        r#"{"jsonrpc":"2.0","id":1,"method":"net/fetch","params":{}}"#,
        r#"{"jsonrpc":"2.0","method":"net/credit","params":{}}"#,
        r#"{"jsonrpc":"2.0","id":1,"method":"ws/open"}"#,
        r#"{"jsonrpc":"2.0","id":1,"method":"timer/set","params":{"ms":10}}"#,
    ] {
        assert!(request_of(line).is_some(), "{line}");
    }
    for line in [
        // The answer of a plugin, whose text speaks of them.
        r#"{"jsonrpc":"2.0","id":1,"result":"net/fetch"}"#,
        r#"{"jsonrpc":"2.0","method":"log","params":{"message":"ws/open"}}"#,
        // Sent to the plugin, never by it.
        r#"{"jsonrpc":"2.0","method":"net/chunk","params":{}}"#,
        "not JSON net/fetch",
    ] {
        assert!(request_of(line).is_none(), "{line}");
    }
}

#[test]
fn a_piece_is_text_when_it_is_utf8_and_a_cut_character_waits_for_its_rest() {
    let mut tail = Vec::new();
    let e = "é".as_bytes();
    assert_eq!(
        chunk_of(1, &mut tail, b"ab", false),
        Some(json!({ "stream": 1, "data": "ab" }))
    );
    assert_eq!(chunk_of(1, &mut tail, &e[..1], false), None);
    assert_eq!(
        chunk_of(1, &mut tail, &[e[1], b'c'], false),
        Some(json!({ "stream": 1, "data": "éc" }))
    );
    assert!(tail.is_empty());
    // A character cut for good, at the end.
    assert_eq!(chunk_of(1, &mut tail, &e[..1], false), None);
    assert_eq!(
        chunk_of(1, &mut tail, &[], true),
        Some(json!({ "stream": 1, "dataBase64": BASE64.encode(&e[..1]) }))
    );
    assert_eq!(chunk_of(1, &mut tail, &[], true), None);
}

#[test]
fn a_piece_that_is_not_utf8_comes_in_base64_whole() {
    let mut tail = Vec::new();
    assert_eq!(
        chunk_of(2, &mut tail, &[b'a', 0xff, 0], false),
        Some(json!({ "stream": 2, "dataBase64": BASE64.encode([b'a', 0xff, 0]) }))
    );
    assert!(tail.is_empty());
}

#[tokio::test]
async fn the_credit_stops_at_what_was_not_consumed_and_never_goes_past_it() {
    let c = Credit::new();
    assert_eq!(c.take(CREDIT + 10).await, CREDIT);
    // Nothing left: it waits.
    assert!(tokio::time::timeout(Duration::from_millis(50), c.take(1))
        .await
        .is_err());
    c.give(100);
    assert_eq!(c.take(1000).await, 100);
    // A plugin that gives back more than it was handed gets 256 KiB, no more.
    c.give(usize::MAX);
    assert_eq!(c.take(usize::MAX).await, CREDIT);
}

#[test]
fn the_queue_of_a_socket_takes_one_long_message_when_nothing_waits() {
    assert!(fits(0, WS_MESSAGE));
    assert!(fits(1, WS_QUEUE - 1));
    assert!(!fits(1, WS_QUEUE));
    assert!(!fits(WS_QUEUE, 1));
}

/// `ws/send` on a socket nobody drains: refused once its queue is full, over 1 MiB, on a socket that is not there.
#[tokio::test(flavor = "multi_thread")]
async fn ws_send_says_no_past_its_queue_past_a_message_and_on_no_socket() {
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<Frame>();
    let fetcher = Arc::new(Fetcher::new("dev.test", &[], &Default::default()));
    let quiet: Arc<dyn Fn(&str) + Send + Sync> = Arc::new(|_| {});
    let beside = Beside::new(fetcher, tx.downgrade(), quiet.clone(), quiet, None);
    let (out, mut waiting) = unbounded_channel();
    let queued = Arc::new(AtomicUsize::new(0));
    locked(&beside.sockets).insert(
        1,
        OpenSocket {
            out,
            queued: queued.clone(),
        },
    );
    let text = |n: usize| "a".repeat(n);
    let send = |m: String| beside.send(&json!({ "socket": 1, "text": m }));
    // The most a message may hold, alone.
    assert!(send(text(WS_MESSAGE)).is_ok());
    assert_eq!(queued.load(Ordering::Acquire), WS_MESSAGE);
    assert_eq!(send(text(1)).unwrap_err().0, REFUSED);
    // Once it left, there is room again, up to the queue.
    assert!(waiting.try_recv().is_ok());
    queued.store(0, Ordering::Release);
    assert!(send(text(WS_QUEUE - 1)).is_ok());
    assert!(send(text(1)).is_ok());
    let (code, why) = send(text(1)).unwrap_err();
    assert_eq!(code, REFUSED);
    assert!(why.contains("wait to be sent"), "{why}");
    let (code, _) = send(text(WS_MESSAGE + 1)).unwrap_err();
    assert_eq!(code, REFUSED);
    let (code, why) = beside
        .send(&json!({ "socket": 9, "text": "a" }))
        .unwrap_err();
    assert_eq!(code, REFUSED);
    assert!(why.contains("no socket 9"), "{why}");
    let (code, _) = beside
        .send(&json!({ "socket": 1, "dataBase64": "%%" }))
        .unwrap_err();
    assert_eq!(code, crate::plugins::runtime::INVALID_PARAMS);
    let (code, _) = beside.send(&json!({ "socket": 1 })).unwrap_err();
    assert_eq!(code, crate::plugins::runtime::INVALID_PARAMS);
}
