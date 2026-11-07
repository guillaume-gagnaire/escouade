//! Each service's stream against a fake server that speaks as its documentation says (texts as it goes, then the final
//! one), refuses the key, fails, or closes. The service is its plugin.

use super::super::online::Spec;
use super::*;
use base64::Engine as _;
use escouade_voice::live::url_of;
use futures_util::{SinkExt, StreamExt};
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_tungstenite::tungstenite::{self, protocol::Role, Message};

pub(crate) const KEY: &str = "sk-live-test-key-123456";

/// How the fake service behaves.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Mode {
    /// As its documentation says.
    Speaks,
    /// The handshake refused with this status.
    Refuses(u16),
    /// An error message after the first sound.
    Fails,
    /// Closed after the first sound.
    Closes,
    /// Closed at the sentence's end, before any final text: a stream that breaks then.
    ClosesAtEnd,
    /// Hears, never gives the final text.
    Mute,
}

/// A stream as the fake service got it.
#[derive(Debug, Clone, Default)]
pub(crate) struct Conn {
    /// "GET /v1/…?… HTTP/1.1"
    pub line: String,
    pub headers: Vec<(String, String)>,
    /// The text messages, the sound's taken out (`audio` counts it).
    pub texts: Vec<String>,
    /// The bytes of PCM sound it got.
    pub audio: usize,
    /// That sound, in the order it came (16-bit PCM, little-endian).
    pub pcm: Vec<u8>,
    pub ended: bool,
    /// The commits (Mistral's flushes) it got.
    pub commits: usize,
}

impl Conn {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }
}

/// A fake service: its streams, and the requests of the sentences sent whole (by request, its text "par requête").
pub(crate) struct FakeLive {
    pub url: String,
    pub conns: Arc<Mutex<Vec<Conn>>>,
    pub requests: Arc<Mutex<Vec<String>>>,
    _rt: tokio::runtime::Runtime,
}

impl FakeLive {
    pub fn conns(&self) -> Vec<Conn> {
        self.conns.lock().clone()
    }

    pub fn spec(&self, p: Provider) -> Spec {
        Spec {
            base: self.url.clone(),
            live: Some(p.live_model(p.model())),
            ..Spec::new(p, KEY.into())
        }
    }
}

/// The text a service gives as the sentence goes on, then the final one.
pub(crate) const PARTIAL: &str = "bonjour";
pub(crate) const FINAL: &str = "bonjour tout le monde";
pub(crate) const BY_REQUEST: &str = "par requête";

pub(crate) fn fake(p: Provider, mode: Mode) -> FakeLive {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let listener = rt
        .block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))
        .unwrap();
    let addr = listener.local_addr().unwrap();
    let conns = Arc::new(Mutex::new(Vec::new()));
    let requests = Arc::new(Mutex::new(Vec::new()));
    let (c, r) = (conns.clone(), requests.clone());
    rt.spawn(async move {
        loop {
            let Ok((sock, _)) = listener.accept().await else {
                return;
            };
            let (c, r) = (c.clone(), r.clone());
            tokio::spawn(async move { serve_one(p, mode, sock, c, r).await });
        }
    });
    FakeLive {
        url: format!("http://{addr}"),
        conns,
        requests,
        _rt: rt,
    }
}

/// Reads a request's head, a byte at a time (nothing of what follows is taken).
pub(crate) async fn read_head(
    sock: &mut tokio::net::TcpStream,
) -> Option<(String, Vec<(String, String)>)> {
    let mut buf = Vec::new();
    let mut b = [0u8; 1];
    while !buf.ends_with(b"\r\n\r\n") {
        if sock.read(&mut b).await.ok()? == 0 {
            return None;
        }
        buf.push(b[0]);
    }
    let head = String::from_utf8_lossy(&buf).to_string();
    let mut lines = head.split("\r\n");
    let line = lines.next().unwrap_or("").to_string();
    let headers = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(n, v)| (n.trim().to_lowercase(), v.trim().to_string()))
        .collect();
    Some((line, headers))
}

/// The answer to a commit: the text of what was said since the one before (`first`: all of it; after: nothing).
fn final_message(p: Provider, first: bool) -> String {
    let (all, rest) = if first {
        (FINAL, "tout le monde")
    } else {
        ("", "")
    };
    let n = if first { "i1" } else { "i2" };
    match p {
        Provider::ElevenLabs => {
            serde_json::json!({"message_type": "committed_transcript", "text": all}).to_string()
        }
        Provider::Mistral => serde_json::json!({
            "type": "transcription.done", "text": FINAL, "model": "m", "language": null, "usage": {}
        })
        .to_string(),
        Provider::Deepgram => serde_json::json!({
            "type": "Results", "is_final": true, "speech_final": true, "from_finalize": true,
            "channel": {"alternatives": [{"transcript": rest}]}
        })
        .to_string(),
        _ => [
            serde_json::json!({"type": "input_audio_buffer.committed", "item_id": n}).to_string(),
            serde_json::json!({
                "type": "conversation.item.input_audio_transcription.completed",
                "item_id": n, "content_index": 0, "transcript": all
            })
            .to_string(),
        ]
        .join(
            "
",
        ),
    }
}

fn partial_messages(p: Provider) -> Vec<String> {
    match p {
        Provider::ElevenLabs => {
            vec![serde_json::json!({"message_type": "partial_transcript", "text": PARTIAL}).to_string()]
        }
        Provider::Mistral => vec![serde_json::json!({"type": "transcription.text.delta", "text": PARTIAL}).to_string()],
        // Its final piece, then what it still hears.
        Provider::Deepgram => vec![
            serde_json::json!({"type": "Results", "is_final": true, "channel": {"alternatives": [{"transcript": PARTIAL}]}})
                .to_string(),
            serde_json::json!({"type": "Results", "is_final": false, "channel": {"alternatives": [{"transcript": "tout"}]}})
                .to_string(),
        ],
        _ => vec![serde_json::json!({
            "type": "conversation.item.input_audio_transcription.delta", "item_id": "i1", "delta": PARTIAL
        })
        .to_string()],
    }
}

fn error_message(p: Provider) -> String {
    match p {
        Provider::ElevenLabs => {
            serde_json::json!({"message_type": "transcriber_error", "error": "boom"}).to_string()
        }
        Provider::Mistral => serde_json::json!({"type": "error", "error": {"message": "boom", "code": 500}}).to_string(),
        Provider::Deepgram => serde_json::json!({"type": "Error", "description": "boom", "message": ""}).to_string(),
        _ => serde_json::json!({"type": "error", "error": {"message": "boom", "code": "server_error"}}).to_string(),
    }
}

/// The sound a message carries (its bytes), whether it ends the sentence, and the message without its sound.
fn read_client(p: Provider, m: &Message) -> (Vec<u8>, bool, Option<String>) {
    let decoded = |s: &str| {
        base64::engine::general_purpose::STANDARD
            .decode(s)
            .unwrap_or_default()
    };
    match m {
        Message::Binary(b) => (b.to_vec(), false, None),
        Message::Text(t) => {
            let v: serde_json::Value = serde_json::from_str(t.as_str()).unwrap_or_default();
            let mut kept = v.clone();
            let (audio, end) = match p {
                Provider::ElevenLabs => {
                    kept["audio_base_64"] = "…".into();
                    (
                        decoded(v["audio_base_64"].as_str().unwrap_or("")),
                        v["commit"].as_bool() == Some(true),
                    )
                }
                Provider::Mistral => {
                    if v.get("audio").is_some() {
                        kept["audio"] = "…".into();
                    }
                    (
                        decoded(v["audio"].as_str().unwrap_or("")),
                        v["type"] == "input_audio.end" || v["type"] == "input_audio.flush",
                    )
                }
                Provider::Deepgram => (Vec::new(), v["type"] == "Finalize"),
                _ => {
                    if v.get("audio").is_some() {
                        kept["audio"] = "…".into();
                    }
                    (
                        decoded(v["audio"].as_str().unwrap_or("")),
                        v["type"] == "input_audio_buffer.commit",
                    )
                }
            };
            (audio, end, Some(kept.to_string()))
        }
        _ => (Vec::new(), false, None),
    }
}

async fn serve_one(
    p: Provider,
    mode: Mode,
    mut sock: tokio::net::TcpStream,
    conns: Arc<Mutex<Vec<Conn>>>,
    requests: Arc<Mutex<Vec<String>>>,
) {
    let Some((line, headers)) = read_head(&mut sock).await else {
        return;
    };
    let get = |n: &str| headers.iter().find(|(k, _)| k == n).map(|(_, v)| v.clone());
    let Some(key) = get("sec-websocket-key") else {
        // A sentence sent whole: its body read, the text of a request.
        let len: usize = get("content-length")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let mut body = vec![0u8; len];
        let _ = sock.read_exact(&mut body).await;
        requests.lock().push(line);
        let text = match p {
            Provider::Deepgram => serde_json::json!({"results": {"channels": [{"alternatives": [{"transcript": BY_REQUEST}]}]}}),
            _ => serde_json::json!({"text": BY_REQUEST}),
        }
        .to_string();
        let _ = sock
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}",
                    text.len()
                )
                .as_bytes(),
            )
            .await;
        return;
    };
    let index = {
        let mut c = conns.lock();
        c.push(Conn {
            line: line.clone(),
            headers: headers.clone(),
            ..Default::default()
        });
        c.len() - 1
    };
    if let Mode::Refuses(status) = mode {
        let body = r#"{"detail":{"message":"invalid key"}}"#;
        let _ = sock
            .write_all(
                format!("HTTP/1.1 {status} Nope\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
                    .as_bytes(),
            )
            .await;
        return;
    }
    let accept = tungstenite::handshake::derive_accept_key(key.as_bytes());
    let _ = sock
        .write_all(
            format!("HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n")
                .as_bytes(),
        )
        .await;
    let mut ws =
        tokio_tungstenite::WebSocketStream::from_raw_socket(sock, Role::Server, None).await;
    let hello = match p {
        Provider::ElevenLabs => {
            Some(r#"{"message_type":"session_started","session_id":"s1","config":{}}"#)
        }
        Provider::Mistral => Some(r#"{"type":"session.created","session":{}}"#),
        _ => None,
    };
    if let Some(h) = hello {
        let _ = ws.send(Message::text(h)).await;
    }
    let mut sounds = 0;
    let mut commits = 0;
    while let Some(Ok(m)) = ws.next().await {
        if let Message::Close(_) = m {
            return;
        }
        let (audio, end, text) = read_client(p, &m);
        {
            let mut c = conns.lock();
            let c = &mut c[index];
            c.audio += audio.len();
            c.pcm.extend_from_slice(&audio);
            if let Some(t) = text {
                c.texts.push(t);
            }
            c.ended |= end
                && (p != Provider::Mistral
                    || matches!(&m, Message::Text(t) if t.as_str().contains("input_audio.end")));
            c.commits += usize::from(end);
        }
        if end && matches!(mode, Mode::ClosesAtEnd) {
            let _ = ws.close(None).await;
            return;
        }
        if !audio.is_empty() {
            sounds += 1;
            match mode {
                Mode::Fails if sounds == 1 => {
                    let _ = ws.send(Message::text(error_message(p))).await;
                    continue;
                }
                Mode::Closes if sounds == 1 => {
                    let _ = ws.close(None).await;
                    return;
                }
                _ => {}
            }
            if sounds == 1 {
                for m in partial_messages(p) {
                    let _ = ws.send(Message::text(m)).await;
                }
            }
        }
        // Mistral's text comes once it is told the end; the others' at each commit.
        let end = end
            && (p != Provider::Mistral
                || matches!(&m, Message::Text(t) if t.as_str().contains("input_audio.end")));
        if end && mode != Mode::Mute {
            commits += 1;
            for line in final_message(p, commits == 1).lines() {
                let _ = ws.send(Message::text(line.to_string())).await;
            }
            if p == Provider::Deepgram {
                // Its metadata, then it closes once told `CloseStream`.
                let _ = ws.send(Message::text(r#"{"type":"Metadata"}"#)).await;
            }
        }
        if p == Provider::Deepgram
            && matches!(&m, Message::Text(t) if t.as_str().contains("CloseStream"))
        {
            let _ = ws.close(None).await;
            return;
        }
    }
}

/// `secs` of a sound at 48 kHz, as a microphone gives it.
pub(crate) fn speech(secs: f32) -> Vec<f32> {
    (0..(48_000.0 * secs) as usize)
        .map(|i| (i as f32 * 0.05).sin() * 0.3)
        .collect()
}

/// Pushes `samples` 20 ms at a time, then waits for the final text.
/// The stream of `spec`, as its plugin serves it.
pub(crate) fn open(spec: &Spec, language: &str, rate: u32, partial: Partial) -> LiveStream {
    crate::voice::service::of(spec.provider).open_live(spec, language, rate, partial)
}

fn say(stream: LiveStream, samples: &[f32]) -> Result<String, OnlineError> {
    for frame in samples.chunks(960) {
        stream.push(frame);
    }
    stream.finish(FINAL_WAIT).map(|f| f.text)
}

fn partials() -> (Partial, Arc<Mutex<Vec<String>>>) {
    let got = Arc::new(Mutex::new(Vec::new()));
    let g = got.clone();
    (Box::new(move |t| g.lock().push(t)), got)
}

#[test]
fn each_service_writes_the_text_as_it_hears_then_gives_the_final_one() {
    for p in [
        Provider::ElevenLabs,
        Provider::Mistral,
        Provider::Deepgram,
        Provider::OpenAi,
    ] {
        let f = fake(p, Mode::Speaks);
        let (partial, seen) = partials();
        let stream = open(&f.spec(p), "fr", 48_000, partial);
        let text = say(stream, &speech(1.0)).unwrap();
        assert_eq!(text, FINAL, "{p:?}");
        let seen = seen.lock().clone();
        assert!(
            seen.iter().any(|t| t.starts_with(PARTIAL)),
            "{p:?}: {seen:?}"
        );
        let c = f.conns();
        assert_eq!(c.len(), 1, "{p:?}");
        let c = &c[0];
        assert!(c.ended, "{p:?}: {:?}", c.texts);
        // All the sound, at its rate, 16 bits: OpenAI's at 24 kHz, the others' at 16 kHz.
        let rate = if p == Provider::OpenAi {
            24_000
        } else {
            16_000
        };
        let want = rate * 2;
        assert!(
            c.audio.abs_diff(want) <= want / 50 + 3200,
            "{p:?}: {} bytes of sound for {want}",
            c.audio
        );
        assert!(f.requests.lock().is_empty(), "{p:?}");
    }
}

#[test]
fn each_stream_says_its_model_language_and_key_as_its_service_asks() {
    let check = |p: Provider, line_has: &[&str], header: (&str, String), texts_have: &[&str]| {
        let f = fake(p, Mode::Speaks);
        let stream = open(&f.spec(p), "fr", 48_000, Box::new(|_| {}));
        say(stream, &speech(0.3)).unwrap();
        let c = f.conns()[0].clone();
        for want in line_has {
            assert!(c.line.contains(want), "{p:?}: {} lacks {want}", c.line);
        }
        assert_eq!(c.header(header.0), Some(header.1.as_str()), "{p:?}");
        let all = c.texts.join("\n");
        for want in texts_have {
            assert!(all.contains(want), "{p:?}: {all} lacks {want}");
        }
    };
    check(
        Provider::ElevenLabs,
        &[
            "GET /v1/speech-to-text/realtime?",
            "model_id=scribe_v2_realtime",
            "audio_format=pcm_16000",
            "commit_strategy=manual",
            "language_code=fr",
        ],
        ("xi-api-key", KEY.into()),
        &[
            r#""message_type":"input_audio_chunk""#,
            r#""commit":true"#,
            r#""sample_rate":16000"#,
        ],
    );
    check(
        Provider::Mistral,
        &["GET /v1/audio/transcriptions/realtime?model=voxtral-mini-transcribe-realtime-2602"],
        ("authorization", format!("Bearer {KEY}")),
        &[
            r#""type":"session.update""#,
            r#""encoding":"pcm_s16le""#,
            r#""type":"input_audio.append""#,
            "input_audio.flush",
            "input_audio.end",
        ],
    );
    check(
        Provider::Deepgram,
        &[
            "GET /v1/listen?model=nova-3",
            "encoding=linear16",
            "sample_rate=16000",
            "interim_results=true",
            "language=fr",
        ],
        ("authorization", format!("Token {KEY}")),
        &["Finalize", "CloseStream"],
    );
    check(
        Provider::OpenAi,
        &["GET /v1/realtime?intent=transcription"],
        ("authorization", format!("Bearer {KEY}")),
        &[
            r#""type":"transcription""#,
            r#""rate":24000"#,
            r#""model":"gpt-live-transcribe""#,
            r#""language":"fr""#,
            r#""turn_detection":null"#,
            "input_audio_buffer.append",
            "input_audio_buffer.commit",
        ],
    );
}

#[test]
fn a_refused_key_a_failure_or_a_stream_cut_is_said_and_nothing_hangs() {
    for p in [
        Provider::ElevenLabs,
        Provider::Mistral,
        Provider::Deepgram,
        Provider::OpenAi,
    ] {
        let f = fake(p, Mode::Refuses(401));
        let stream = open(&f.spec(p), "fr", 48_000, Box::new(|_| {}));
        assert_eq!(
            say(stream, &speech(0.3)),
            Err(OnlineError::KeyRefused(p)),
            "{p:?}"
        );

        let f = fake(p, Mode::Refuses(429));
        let stream = open(&f.spec(p), "fr", 48_000, Box::new(|_| {}));
        assert!(
            matches!(say(stream, &speech(0.3)), Err(OnlineError::Quota(..))),
            "{p:?}"
        );

        let f = fake(p, Mode::Fails);
        let stream = open(&f.spec(p), "fr", 48_000, Box::new(|_| {}));
        let e = say(stream, &speech(0.5)).unwrap_err();
        assert!(e.to_string().contains("boom"), "{p:?}: {e}");

        let f = fake(p, Mode::Closes);
        let stream = open(&f.spec(p), "fr", 48_000, Box::new(|_| {}));
        let e = say(stream, &speech(0.5)).unwrap_err();
        assert!(matches!(e, OnlineError::Network(..)), "{p:?}: {e:?}");

        // Cut once the end was sent, before any final text: no empty sentence (it would be lost), the cut is said and
        // the sentence goes by request. Deepgram's close only ends a sentence once it answered its `Finalize`.
        let f = fake(p, Mode::ClosesAtEnd);
        let stream = open(&f.spec(p), "fr", 48_000, Box::new(|_| {}));
        let e = say(stream, &speech(0.5)).unwrap_err();
        assert!(matches!(e, OnlineError::Network(..)), "{p:?}: {e:?}");
    }
    // A service that never gives its final text: given up after the wait.
    let f = fake(Provider::ElevenLabs, Mode::Mute);
    let stream = open(
        &f.spec(Provider::ElevenLabs),
        "fr",
        48_000,
        Box::new(|_| {}),
    );
    for frame in speech(0.3).chunks(960) {
        stream.push(frame);
    }
    let started = Instant::now();
    assert!(matches!(
        stream.finish(Duration::from_millis(300)),
        Err(OnlineError::Network(..))
    ));
    assert!(started.elapsed() < Duration::from_secs(2));
    // Nothing listens: given up as fast.
    let spec = Spec {
        base: "http://127.0.0.1:9".into(),
        ..f.spec(Provider::ElevenLabs)
    };
    let started = Instant::now();
    let stream = open(&spec, "fr", 48_000, Box::new(|_| {}));
    assert!(matches!(
        say(stream, &speech(0.3)),
        Err(OnlineError::Network(..))
    ));
    assert!(started.elapsed() < Duration::from_secs(3));
}

#[test]
fn the_key_never_shows_in_what_a_failure_says() {
    let f = fake(Provider::Mistral, Mode::Refuses(500));
    let stream = open(&f.spec(Provider::Mistral), "fr", 48_000, Box::new(|_| {}));
    let e = say(stream, &speech(0.2)).unwrap_err();
    assert!(!format!("{e:?}").contains(KEY));
    assert!(
        matches!(e, OnlineError::Service(Provider::Mistral, 500, _)),
        "{e:?}"
    );
}

#[test]
fn a_stream_dropped_before_its_end_closes_without_its_final_text() {
    let f = fake(Provider::ElevenLabs, Mode::Speaks);
    let stream = open(
        &f.spec(Provider::ElevenLabs),
        "fr",
        48_000,
        Box::new(|_| {}),
    );
    for frame in speech(0.3).chunks(960) {
        stream.push(frame);
    }
    drop(stream);
    let deadline = Instant::now() + Duration::from_secs(3);
    while f.conns().is_empty() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    std::thread::sleep(Duration::from_millis(200));
    assert!(!f.conns()[0].ended);
}

#[test]
fn a_service_found_in_any_language_is_not_told_one() {
    let spec = |p: Provider| Spec::new(p, KEY.into()).with_live(Some(p.live_model(p.model())));
    let el = url_of(&spec(Provider::ElevenLabs), "auto");
    assert!(
        el.starts_with("wss://api.elevenlabs.io/v1/speech-to-text/realtime?"),
        "{el}"
    );
    assert!(!el.contains("language_code"));
    assert!(url_of(&spec(Provider::Deepgram), "auto").contains("language=multi"));
    let nova2 = Spec::new(Provider::Deepgram, KEY.into())
        .with_model("nova-2")
        .with_live(Some("nova-2".into()));
    let u = url_of(&nova2, "auto");
    assert!(
        u.contains("model=nova-2") && !u.contains("language="),
        "{u}"
    );
}

#[test]
fn groq_streams_nothing_and_the_others_take_their_live_model() {
    assert!(!Provider::Groq.offers_live());
    let groq = Spec::new(Provider::Groq, KEY.into()).with_live(Some("x".into()));
    assert_eq!(groq.live, None);
    assert_eq!(
        Provider::ElevenLabs.live_model("scribe_v2"),
        "scribe_v2_realtime"
    );
    assert_eq!(
        Provider::Mistral.live_model("voxtral-mini-latest"),
        "voxtral-mini-transcribe-realtime-2602"
    );
    assert_eq!(
        Provider::OpenAi.live_model("gpt-4o-transcribe"),
        "gpt-live-transcribe"
    );
    assert_eq!(Provider::Deepgram.live_model("nova-2"), "nova-2");
}

/// The real Scribe v2 Realtime (`#[ignore]`): `ELEVENLABS_API_KEY` of the machine, passed to this process only; the
/// French question of the tests, said at its pace. What came while it was said, and how long the final text took.
#[test]
#[ignore]
fn real_elevenlabs_live() {
    let key = std::env::var("ELEVENLABS_API_KEY").expect("ELEVENLABS_API_KEY");
    let wav =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/voice/testdata/question-fr.wav");
    let mut r = hound::WavReader::open(wav).unwrap();
    let rate = r.spec().sample_rate;
    let samples: Vec<f32> = r
        .samples::<i16>()
        .map(|s| s.unwrap() as f32 / i16::MAX as f32)
        .collect();
    let spec = Spec::new(Provider::ElevenLabs, key).with_live(Some("scribe_v2_realtime".into()));
    let started = Instant::now();
    let seen = Arc::new(Mutex::new(Vec::<(u128, String)>::new()));
    let s2 = seen.clone();
    let stream = open(
        &spec,
        "fr",
        rate,
        Box::new(move |t| s2.lock().push((started.elapsed().as_millis(), t))),
    );
    let per = rate as usize / 50;
    for frame in samples.chunks(per) {
        stream.push(frame);
        std::thread::sleep(Duration::from_millis(20));
    }
    let said = started.elapsed().as_millis();
    let end = Instant::now();
    let text = stream.finish(FINAL_WAIT).unwrap().text;
    let ms = end.elapsed().as_millis();
    for (at, t) in seen.lock().iter() {
        eprintln!("partial at {at} ms: {t}");
    }
    eprintln!("said in {said} ms; final in {ms} ms after the end: {text}");
    assert!(!seen.lock().is_empty(), "no text while it was said");
    assert!(seen.lock().iter().any(|(at, _)| *at < said));
    assert!(!text.is_empty());
}
