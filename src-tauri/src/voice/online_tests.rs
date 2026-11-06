//! Each service against a fake server that answers as its documentation says: the text it gives,
//! the key it refuses, and the network cut.

use super::*;
use parking_lot::Mutex;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// A request as the fake server got it.
#[derive(Debug, Clone)]
pub(crate) struct Got {
    /// "POST /v1/listen?model=… HTTP/1.1"
    pub line: String,
    /// The headers, their names in lower case.
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Got {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }

    pub fn body_text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    /// The value of the multipart field `name`.
    pub fn field(&self, name: &str) -> Option<String> {
        let body = self.body_text();
        let mark = format!("name=\"{name}\"\r\n\r\n");
        let at = body.find(&mark)? + mark.len();
        let end = body[at..].find("\r\n--")?;
        Some(body[at..at + end].to_string())
    }
}

/// What a fake service answers a request: its status, its type, and its body in pieces (sent chunked, `gap` apart,
/// when there are several: a stream).
#[derive(Debug, Clone)]
pub(crate) struct Reply {
    pub status: u16,
    pub content_type: String,
    pub chunks: Vec<Vec<u8>>,
    pub gap: Duration,
}

impl Reply {
    pub fn json(status: u16, body: &str) -> Self {
        Self {
            status,
            content_type: "application/json".into(),
            chunks: vec![body.as_bytes().to_vec()],
            gap: Duration::ZERO,
        }
    }
}

/// A fake service: every request answered `status` and `body`, what it got kept.
pub(crate) async fn serve(status: u16, body: &str) -> (String, Arc<Mutex<Vec<Got>>>) {
    let reply = Reply::json(status, body);
    serve_with(Arc::new(move |_: &Got| reply.clone())).await
}

/// A fake service that answers each request what `answer` says, what it got kept.
pub(crate) async fn serve_with(
    answer: Arc<dyn Fn(&Got) -> Reply + Send + Sync>,
) -> (String, Arc<Mutex<Vec<Got>>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let got = Arc::new(Mutex::new(Vec::new()));
    let g = got.clone();
    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                return;
            };
            let g = g.clone();
            let answer = answer.clone();
            tokio::spawn(async move {
                let mut buf = Vec::new();
                let mut chunk = [0u8; 8192];
                let head_end = loop {
                    let n = sock.read(&mut chunk).await.unwrap_or(0);
                    if n == 0 {
                        return;
                    }
                    buf.extend_from_slice(&chunk[..n]);
                    if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                        break i + 4;
                    }
                };
                let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
                let mut lines = head.split("\r\n");
                let line = lines.next().unwrap_or("").to_string();
                let headers: Vec<(String, String)> = lines
                    .filter_map(|l| l.split_once(':'))
                    .map(|(n, v)| (n.trim().to_lowercase(), v.trim().to_string()))
                    .collect();
                let len: usize = headers
                    .iter()
                    .find(|(n, _)| n == "content-length")
                    .and_then(|(_, v)| v.parse().ok())
                    .unwrap_or(0);
                while buf.len() < head_end + len {
                    let n = sock.read(&mut chunk).await.unwrap_or(0);
                    if n == 0 {
                        break;
                    }
                    buf.extend_from_slice(&chunk[..n]);
                }
                let got = Got {
                    line,
                    headers,
                    body: buf[head_end..].to_vec(),
                };
                let r = answer(&got);
                g.lock().push(got);
                if r.chunks.len() == 1 {
                    let body = &r.chunks[0];
                    let head = format!(
                        "HTTP/1.1 {} X\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        r.status,
                        r.content_type,
                        body.len()
                    );
                    let _ = sock.write_all(head.as_bytes()).await;
                    let _ = sock.write_all(body).await;
                    return;
                }
                let head = format!(
                    "HTTP/1.1 {} X\r\nContent-Type: {}\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
                    r.status, r.content_type
                );
                if sock.write_all(head.as_bytes()).await.is_err() {
                    return;
                }
                for c in &r.chunks {
                    let mut piece = format!("{:x}\r\n", c.len()).into_bytes();
                    piece.extend_from_slice(c);
                    piece.extend_from_slice(b"\r\n");
                    if sock.write_all(&piece).await.is_err() || sock.flush().await.is_err() {
                        return;
                    }
                    tokio::time::sleep(r.gap).await;
                }
                let _ = sock.write_all(b"0\r\n\r\n").await;
            });
        }
    });
    (format!("http://{addr}"), got)
}

/// An address nothing listens on: the network cut.
pub(crate) fn nowhere() -> String {
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = l.local_addr().unwrap();
    drop(l);
    format!("http://{addr}")
}

/// A catalog of nothing listed: the documentation's models.
fn no_catalog() -> crate::voice::online_catalog::CatalogStore {
    crate::voice::online_catalog::CatalogStore {
        path: std::env::temp_dir().join("escouade-no-catalog/none.json"),
    }
}

pub(crate) const KEY: &str = "sk-test-0123456789abcdef";

/// The dictation, as its plugin serves it.
async fn transcribe(
    s: &Spec,
    samples: &[f32],
    language: &str,
    cancel: &Arc<AtomicBool>,
) -> std::result::Result<String, OnlineError> {
    let (s, samples, language, cancel) = (
        s.clone(),
        samples.to_vec(),
        language.to_string(),
        cancel.clone(),
    );
    tokio::task::spawn_blocking(move || {
        crate::voice::service::of(s.provider).transcribe(&s, &samples, &language, &cancel)
    })
    .await
    .unwrap()
}

fn spec(p: Provider, base: &str) -> Spec {
    Spec {
        base: base.into(),
        ..Spec::new(p, KEY.into())
    }
}

/// A second of sound.
fn second() -> Vec<f32> {
    (0..16_000)
        .map(|i| ((i as f32) / 20.0).sin() * 0.3)
        .collect()
}

async fn run(p: Provider, base: &str, language: &str) -> std::result::Result<String, OnlineError> {
    transcribe(
        &spec(p, base),
        &second(),
        language,
        &Arc::new(AtomicBool::new(false)),
    )
    .await
}

fn assert_wav(body: &[u8]) {
    let at = body.windows(4).position(|w| w == b"RIFF").expect("a WAV");
    let r = hound::WavReader::new(std::io::Cursor::new(&body[at..])).unwrap();
    let s = r.spec();
    assert_eq!(
        (s.channels, s.sample_rate, s.bits_per_sample),
        (1, 16_000, 16)
    );
    assert_eq!(r.duration(), 16_000);
}

/// The three of OpenAI's format: the same request, at their address.
async fn openai_like(p: Provider, path: &str) {
    let (url, got) = serve(200, r#"{"text":"  Ajoute un test\n pour la dictée. "}"#).await;
    let base = format!("{url}{path}");
    assert_eq!(
        run(p, &base, "fr").await.unwrap(),
        "Ajoute un test pour la dictée."
    );
    let g = got.lock()[0].clone();
    assert_eq!(
        g.line,
        format!("POST {path}/v1/audio/transcriptions HTTP/1.1")
    );
    assert_eq!(g.header("authorization"), Some(&*format!("Bearer {KEY}")));
    assert!(g
        .header("content-type")
        .unwrap()
        .starts_with("multipart/form-data; boundary="));
    assert_eq!(g.field("model").as_deref(), Some(p.model()));
    assert_eq!(g.field("language").as_deref(), Some("fr"));
    assert!(g
        .body_text()
        .contains("name=\"file\"; filename=\"dictation.wav\""));
    assert_wav(&g.body);
    // "auto": the service finds the language.
    run(p, &base, "auto").await.unwrap();
    assert_eq!(got.lock()[1].field("language"), None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn openai_transcribes_the_sound() {
    openai_like(Provider::OpenAi, "").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn groq_transcribes_the_sound_as_openai_does() {
    openai_like(Provider::Groq, "/openai").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mistral_transcribes_the_sound_with_voxtral() {
    openai_like(Provider::Mistral, "").await;
    let (url, got) = serve(200, r#"{"model":"voxtral-mini-2507","text":"Bonjour","language":"fr","segments":[],"usage":{}}"#).await;
    assert_eq!(run(Provider::Mistral, &url, "fr").await.unwrap(), "Bonjour");
    assert_eq!(got.lock()[0].field("response_format"), None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn elevenlabs_transcribes_the_sound_with_scribe() {
    let (url, got) = serve(
        200,
        r#"{"language_code":"fra","language_probability":0.98,"text":"Lance les tests.","words":[]}"#,
    )
    .await;
    assert_eq!(
        run(Provider::ElevenLabs, &url, "fr").await.unwrap(),
        "Lance les tests."
    );
    let g = got.lock()[0].clone();
    assert_eq!(g.line, "POST /v1/speech-to-text HTTP/1.1");
    assert_eq!(g.header("xi-api-key"), Some(KEY));
    assert_eq!(g.header("authorization"), None);
    assert_eq!(g.field("model_id").as_deref(), Some("scribe_v2"));
    assert_eq!(g.field("language_code").as_deref(), Some("fr"));
    assert_wav(&g.body);
    run(Provider::ElevenLabs, &url, "auto").await.unwrap();
    assert_eq!(got.lock()[1].field("language_code"), None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn deepgram_transcribes_the_raw_sound() {
    let (url, got) = serve(
        200,
        r#"{"metadata":{},"results":{"channels":[{"alternatives":[{"transcript":"Ouvre le ticket.","confidence":0.99}]}]}}"#,
    )
    .await;
    assert_eq!(
        run(Provider::Deepgram, &url, "fr").await.unwrap(),
        "Ouvre le ticket."
    );
    let g = got.lock()[0].clone();
    assert_eq!(
        g.line,
        "POST /v1/listen?model=nova-3&smart_format=true&language=fr HTTP/1.1"
    );
    assert_eq!(g.header("authorization"), Some(&*format!("Token {KEY}")));
    assert_eq!(g.header("content-type"), Some("audio/wav"));
    assert_wav(&g.body);
    run(Provider::Deepgram, &url, "auto").await.unwrap();
    assert_eq!(
        got.lock()[1].line,
        "POST /v1/listen?model=nova-3&smart_format=true&detect_language=true HTTP/1.1"
    );
}

/// What each service answers a wrong key, as its documentation shows it.
fn refusal(p: Provider) -> (u16, String) {
    match p {
        Provider::OpenAi => (401, format!(r#"{{"error":{{"message":"Incorrect API key provided: {KEY}.","type":"invalid_request_error","code":"invalid_api_key"}}}}"#)),
        Provider::Groq => (401, r#"{"error":{"message":"Invalid API Key","type":"invalid_request_error","code":"invalid_api_key"}}"#.into()),
        Provider::Mistral => (401, r#"{"message":"Unauthorized","request_id":"x"}"#.into()),
        Provider::ElevenLabs => (401, r#"{"detail":{"status":"invalid_api_key","message":"Invalid API key"}}"#.into()),
        _ => (401, r#"{"err_code":"INVALID_AUTH","err_msg":"Invalid credentials.","request_id":"x"}"#.into()),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_key_refused_says_so_for_every_service() {
    for p in PROVIDERS {
        let (status, body) = refusal(p);
        let (url, _) = serve(status, &body).await;
        let e = run(p, &url, "fr").await.unwrap_err();
        assert_eq!(e, OnlineError::KeyRefused(p), "{p:?}");
        let m = e.message(crate::i18n::kit(crate::i18n::Lang::Fr));
        assert!(m.contains(p.label()) && m.contains("refuse la clé"), "{m}");
        assert!(!m.contains(KEY));
    }
    let (url, _) = serve(403, "{}").await;
    assert_eq!(
        run(Provider::Deepgram, &url, "fr").await.unwrap_err(),
        OnlineError::KeyRefused(Provider::Deepgram)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_network_cut_says_so_for_every_service() {
    for p in PROVIDERS {
        let e = run(p, &nowhere(), "fr").await.unwrap_err();
        let OnlineError::Network(q, _) = &e else {
            panic!("{p:?}: {e:?}");
        };
        assert_eq!(*q, p);
        let m = e.message(crate::i18n::kit(crate::i18n::Lang::En));
        assert!(
            m.starts_with(&format!("{} does not answer", p.label())),
            "{m}"
        );
        assert!(m.contains("proxy"), "{m}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn another_error_tells_what_the_service_said_without_the_key() {
    let body = format!(r#"{{"error":{{"message":"Audio too short for {KEY}"}}}}"#);
    let (url, _) = serve(400, &body).await;
    let e = run(Provider::OpenAi, &url, "fr").await.unwrap_err();
    assert_eq!(
        e,
        OnlineError::Service(Provider::OpenAi, 400, "Audio too short for ***".into())
    );
    let (url, _) = serve(200, r#"{"nothing":1}"#).await;
    assert!(matches!(
        run(Provider::Groq, &url, "fr").await.unwrap_err(),
        OnlineError::Service(Provider::Groq, 200, _)
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_cancel_gives_up_the_request() {
    // A server that never answers.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        let mut held = Vec::new();
        while let Ok((s, _)) = listener.accept().await {
            held.push(s);
        }
    });
    let cancel = Arc::new(AtomicBool::new(false));
    let c = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        c.store(true, Ordering::Release);
    });
    let started = std::time::Instant::now();
    let text = transcribe(&spec(Provider::OpenAi, &url), &second(), "fr", &cancel)
        .await
        .unwrap();
    assert_eq!(text, "");
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_transcriber_blocks_on_a_thread_of_its_own() {
    let (url, _) = serve(200, r#"{"text":"Bonjour"}"#).await;
    let t = Online(spec(Provider::Groq, &url));
    // Called from a blocking thread, as `Voice::stop` is.
    let text = tokio::task::spawn_blocking(move || {
        t.transcribe(
            Path::new(""),
            &second(),
            "fr",
            &Arc::new(AtomicBool::new(false)),
        )
    })
    .await
    .unwrap()
    .unwrap();
    assert_eq!(text, "Bonjour");
    // And its error is the one of the service, for the window to tell.
    let t = Online(spec(Provider::Groq, &nowhere()));
    let e = tokio::task::spawn_blocking(move || {
        t.transcribe(
            Path::new(""),
            &second(),
            "fr",
            &Arc::new(AtomicBool::new(false)),
        )
    })
    .await
    .unwrap()
    .unwrap_err();
    assert!(matches!(
        e.downcast_ref::<OnlineError>(),
        Some(OnlineError::Network(Provider::Groq, _))
    ));
}

#[test]
fn the_key_never_shows_in_the_spec_debug_nor_a_redacted_text() {
    let s = spec(Provider::OpenAi, "http://x");
    assert!(!format!("{s:?}").contains(KEY));
    assert_eq!(redact(&format!("bad {KEY}!"), KEY), "bad ***!");
    assert_eq!(
        crate::voice::online::find("deepgram"),
        Some(Provider::Deepgram)
    );
    assert_eq!(Provider::ElevenLabs.entry(), "dictation-elevenlabs");
}

#[test]
fn the_key_is_kept_in_the_keychain_and_only_there() {
    use crate::integrations::secrets::MemoryStore;
    let store = MemoryStore::default();
    assert!(keys(&store).is_empty());
    save_key(&store, "mistral", &format!("  {KEY}\n")).unwrap();
    assert_eq!(store.entry("dictation-mistral").as_deref(), Some(KEY));
    assert_eq!(keys(&store), vec!["mistral".to_string()]);
    assert!(save_key(&store, "mistral", "  ").is_err());
    assert!(save_key(&store, "nope", KEY).is_err());

    // The settings hold the service, never the key: what goes to settings.json.
    let mut settings = crate::model::Settings::default();
    settings.voice.transcription = "online".into();
    settings.voice.online_service = "mistral".into();
    assert!(!serde_json::to_string(&settings).unwrap().contains(KEY));
    let spec = spec_of(&settings, &store, &no_catalog()).unwrap().unwrap();
    assert_eq!((spec.provider, spec.key.as_str()), (Provider::Mistral, KEY));
    assert_eq!(spec.base, "https://api.mistral.ai");
    // « Transcription en temps réel », on by default (ESC-136): its model of its own.
    assert_eq!(
        spec.live.as_deref(),
        Some("voxtral-mini-transcribe-realtime-2602")
    );
    settings.voice.online_live.insert("mistral".into(), false);
    let spec = spec_of(&settings, &store, &no_catalog()).unwrap().unwrap();
    assert_eq!(spec.live, None, "unticked: each sentence whole");
    // Groq offers none: never streamed.
    save_key(&store, "groq", KEY).unwrap();
    settings.voice.online_service = "groq".into();
    assert_eq!(
        spec_of(&settings, &store, &no_catalog())
            .unwrap()
            .unwrap()
            .live,
        None
    );
    settings.voice.online_service = "mistral".into();

    // Local (the default): no key read.
    assert!(
        spec_of(&crate::model::Settings::default(), &store, &no_catalog())
            .unwrap()
            .is_none()
    );

    // No key for the service chosen: said before anything is listened to.
    settings.voice.online_service = "deepgram".into();
    assert_eq!(
        spec_of(&settings, &store, &no_catalog()).unwrap_err(),
        super::super::VoiceError::Online(OnlineError::NoKey(Provider::Deepgram))
    );

    delete_key(&store, "mistral").unwrap();
    assert_eq!(store.entry("dictation-mistral"), None);
}

#[test]
fn a_keychain_that_refuses_the_key_leaves_it_unsaved_and_says_so_without_it() {
    use crate::integrations::secrets::MemoryStore;
    let store = MemoryStore::default();
    store.refuse.store(true, Ordering::SeqCst);
    let e = save_key(&store, "openai", KEY).unwrap_err();
    assert!(!e.contains(KEY), "{e}");
    store.refuse.store(false, Ordering::SeqCst);
    // One that says it kept it and kept nothing: caught by the reading back.
    store.lose.store(true, Ordering::SeqCst);
    assert!(save_key(&store, "openai", KEY).is_err());
    assert_eq!(store.entry("dictation-openai"), None);
}

/// The real ElevenLabs (Scribe), with the user's key given to this process only
/// (`ELEVENLABS_API_KEY`), on a few seconds of French: run by hand, never in CI, never with the
/// key written anywhere. `ESC110_WAV`: the sound to send.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore]
async fn real_elevenlabs() {
    let key = std::env::var("ELEVENLABS_API_KEY").expect("ELEVENLABS_API_KEY");
    let wav = std::env::var("ESC110_WAV").expect("ESC110_WAV");
    let (samples, rate) = crate::voice::source::read_wav(Path::new(&wav)).unwrap();
    let samples = crate::voice::audio::resample(&samples, rate, 16_000);
    let s = Spec::new(Provider::ElevenLabs, key);
    let started = std::time::Instant::now();
    let text = transcribe(&s, &samples, "fr", &Arc::new(AtomicBool::new(false))).await;
    println!(
        "{:.1} s of sound, answered in {} ms: {:?}",
        samples.len() as f32 / 16_000.0,
        started.elapsed().as_millis(),
        text
    );
    assert!(!text.unwrap().is_empty());
    // A wrong key: refused.
    let s = Spec::new(Provider::ElevenLabs, "not-a-key-0000".into());
    let e = transcribe(&s, &samples, "fr", &Arc::new(AtomicBool::new(false))).await;
    println!("wrong key: {e:?}");
    assert_eq!(
        e.unwrap_err(),
        OnlineError::KeyRefused(Provider::ElevenLabs)
    );
}
