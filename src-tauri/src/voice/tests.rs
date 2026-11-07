//! The dictation without a microphone nor a model: a source the test feeds, a transcriber that
//! says what it was given, a fake server for the models.

use super::models::{ModelInfo, Store};
use super::source::{Capture, Device, Sink, Source};
use super::transcribe::Fake;
use super::*;
use std::sync::atomic::AtomicBool;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// A source the test feeds through `push` and `fail`; `dropped` once its capture is let go.
#[derive(Clone, Default)]
struct Script {
    sink: Arc<Mutex<Option<Sink>>>,
    dropped: Arc<AtomicBool>,
    opened: Arc<Mutex<Vec<String>>>,
    refuse: Arc<Mutex<Option<VoiceError>>>,
    rate: u32,
    /// How long opening takes (macOS asking the user).
    delay: Arc<Mutex<Duration>>,
}

impl Script {
    fn new(rate: u32) -> Self {
        Self {
            rate,
            ..Default::default()
        }
    }

    fn push(&self, samples: &[f32]) {
        if let Some(s) = self.sink.lock().as_mut() {
            (s.samples)(samples);
        }
    }

    fn fail(&self, e: VoiceError) {
        if let Some(s) = self.sink.lock().as_mut() {
            (s.error)(e);
        }
    }
}

struct ScriptCapture(Script);

impl Capture for ScriptCapture {
    fn rate(&self) -> u32 {
        self.0.rate
    }
}

impl Drop for ScriptCapture {
    fn drop(&mut self) {
        self.0.sink.lock().take();
        self.0.dropped.store(true, Ordering::Release);
    }
}

impl Source for Script {
    fn open(&self, device: &str, sink: Sink) -> Result<Box<dyn Capture>, VoiceError> {
        if let Some(e) = self.refuse.lock().clone() {
            return Err(e);
        }
        std::thread::sleep(*self.delay.lock());
        self.opened.lock().push(device.to_string());
        *self.sink.lock() = Some(sink);
        self.dropped.store(false, Ordering::Release);
        Ok(Box::new(ScriptCapture(self.clone())))
    }

    fn devices(&self) -> Vec<Device> {
        vec![Device {
            id: "mic-1".into(),
            name: "Micro test".into(),
            default: true,
        }]
    }
}

struct Harness {
    voice: Voice,
    script: Script,
    fake: Arc<Fake>,
    events: Arc<Mutex<Vec<VoiceEvent>>>,
    dir: PathBuf,
}

fn harness(name: &str, text: &str, rate: u32) -> Harness {
    let dir = crate::paths::test_dir(&format!("voice-{name}"));
    let script = Script::new(rate);
    let fake = Arc::new(Fake::new(text));
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = events.clone();
    let voice = Voice::new(
        Box::new(script.clone()),
        fake.clone(),
        Store {
            dir: dir.join("models"),
        },
        "http://127.0.0.1:9".into(),
        Arc::new(move |e| sink.lock().push(e)),
    );
    Harness {
        voice,
        script,
        fake,
        events,
        dir,
    }
}

/// A model of the catalog on the disk: a file of its size (sparse: nothing written).
fn put_model(store: &Store, id: &str) {
    std::fs::create_dir_all(&store.dir).unwrap();
    let f = std::fs::File::create(store.path(id)).unwrap();
    f.set_len(models::find(id).unwrap().size).unwrap();
}

fn choice(model: &str, language: &str) -> Choice {
    Choice {
        device: String::new(),
        model: model.into(),
        language: language.into(),
        online: None,
    }
}

fn session(s: Started) -> u64 {
    match s {
        Started::Listening { session } => session,
        other => panic!("not listening: {other:?}"),
    }
}

fn voice_tone(rate: u32, secs: f32) -> Vec<f32> {
    (0..(rate as f32 * secs) as usize)
        .map(|i| (i as f32 * 0.05).sin() * 0.3)
        .collect()
}

#[test]
fn without_its_model_a_dictation_asks_for_it_with_its_size_and_listens_to_nothing() {
    let h = harness("needs-model", "", 48_000);
    let s = h.voice.start(&choice("base", "fr")).unwrap();
    assert_eq!(
        s,
        Started::NeedsModel {
            model: "base".into(),
            size: 147_951_465
        }
    );
    assert!(h.script.opened.lock().is_empty());
    assert_eq!(h.voice.listening(), None);
    // A model the catalog does not have: the default one.
    assert!(matches!(
        h.voice.start(&choice("whatever", "fr")).unwrap(),
        Started::NeedsModel { model, .. } if model == "base"
    ));
}

#[test]
fn what_was_said_at_48k_reaches_the_transcriber_at_16k_mono_in_the_language_chosen() {
    let h = harness("transcribe", "Ajoute un test", 48_000);
    put_model(&h.voice.store, "base");
    let id = session(h.voice.start(&choice("base", "fr")).unwrap());
    assert_eq!(h.voice.listening(), Some(id));
    h.script.push(&voice_tone(48_000, 1.5));
    let text = h.voice.stop(id).unwrap();
    assert_eq!(text, "Ajoute un test");
    assert_eq!(*h.fake.calls.lock(), vec![(24_000, "fr".to_string())]);
    // The microphone was let go, and a second listening may start.
    assert!(h.script.dropped.load(Ordering::Acquire));
    assert_eq!(h.voice.listening(), None);
    let id2 = session(h.voice.start(&choice("base", "auto")).unwrap());
    assert!(id2 > id);
    h.voice.cancel();
}

#[test]
fn the_microphone_chosen_in_the_settings_is_the_one_opened() {
    let h = harness("device", "x", 16_000);
    put_model(&h.voice.store, "base");
    let c = Choice {
        device: "mic-1".into(),
        ..choice("base", "en")
    };
    let id = session(h.voice.start(&c).unwrap());
    assert_eq!(*h.script.opened.lock(), vec!["mic-1".to_string()]);
    h.voice.cancel();
    assert!(h.voice.stop(id).is_err());
    assert_eq!(h.voice.devices()[0].name, "Micro test");
}

#[test]
fn one_dictation_at_a_time() {
    let h = harness("one", "x", 16_000);
    put_model(&h.voice.store, "base");
    session(h.voice.start(&choice("base", "fr")).unwrap());
    assert_eq!(
        h.voice.start(&choice("base", "fr")).unwrap_err(),
        VoiceError::AlreadyListening
    );
    h.voice.cancel();
}

#[test]
fn the_level_is_told_while_it_listens_and_no_more_after() {
    let h = harness("level", "", 16_000);
    put_model(&h.voice.store, "base");
    let id = session(h.voice.start(&choice("base", "fr")).unwrap());
    h.script.push(&voice_tone(16_000, 0.1));
    // Waited for, not slept for: on a busy machine (the CI's macOS, 2026-11-02) the first level came after 160 ms.
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let levels: Vec<(f32, u64)> = loop {
        std::thread::sleep(Duration::from_millis(40));
        let levels: Vec<(f32, u64)> = h
            .events
            .lock()
            .iter()
            .filter_map(|e| match e {
                VoiceEvent::Level { session, level, ms } if *session == id => Some((*level, *ms)),
                _ => None,
            })
            .collect();
        if levels.len() >= 2 || std::time::Instant::now() > deadline {
            break levels;
        }
    };
    assert!(!levels.is_empty());
    assert!(levels[0].0 > 0.3, "{levels:?}");
    // The time grows; the level falls back once nothing comes.
    assert!(levels.last().unwrap().1 >= levels[0].1);
    h.voice.cancel();
    std::thread::sleep(Duration::from_millis(120));
    let n = h.events.lock().len();
    std::thread::sleep(Duration::from_millis(150));
    assert_eq!(h.events.lock().len(), n, "level told after the end");
    assert!(h.script.dropped.load(Ordering::Acquire));
}

#[test]
fn a_microphone_kept_from_the_app_says_how_to_allow_it() {
    let h = harness("silent", "jamais", 16_000);
    put_model(&h.voice.store, "base");
    let id = session(h.voice.start(&choice("base", "fr")).unwrap());
    h.script.push(&vec![0.0; 16_000]);
    assert_eq!(h.voice.stop(id).unwrap_err(), VoiceError::Silent);
    assert!(h.fake.calls.lock().is_empty());
    let mac = VoiceError::Silent.message(Lang::Fr, "macos");
    assert!(
        mac.contains("Confidentialité et sécurité › Microphone"),
        "{mac}"
    );
    let win = VoiceError::Denied.message(Lang::En, "windows");
    assert!(
        win.contains("Let desktop apps access your microphone"),
        "{win}"
    );
    let linux = VoiceError::Denied.message(Lang::Fr, "linux");
    assert!(linux.contains("PipeWire"), "{linux}");
    assert!(VoiceError::NoDevice
        .message(Lang::Fr, "linux")
        .contains("Réglages › Voix"));
}

#[test]
fn a_refusal_or_no_microphone_at_the_start_is_said_and_nothing_listens() {
    let h = harness("refused", "", 16_000);
    put_model(&h.voice.store, "base");
    *h.script.refuse.lock() = Some(VoiceError::Denied);
    assert_eq!(
        h.voice.start(&choice("base", "fr")).unwrap_err(),
        VoiceError::Denied
    );
    *h.script.refuse.lock() = Some(VoiceError::NoDevice);
    assert_eq!(
        h.voice.start(&choice("base", "fr")).unwrap_err(),
        VoiceError::NoDevice
    );
    assert_eq!(h.voice.listening(), None);
}

#[test]
fn a_click_without_a_word_gives_no_text() {
    let h = harness("click", "jamais", 16_000);
    put_model(&h.voice.store, "base");
    let id = session(h.voice.start(&choice("base", "fr")).unwrap());
    assert_eq!(h.voice.stop(id).unwrap(), "");
    let id = session(h.voice.start(&choice("base", "fr")).unwrap());
    h.script.push(&voice_tone(16_000, 0.05));
    assert_eq!(h.voice.stop(id).unwrap(), "");
    assert!(h.fake.calls.lock().is_empty());
}

#[test]
fn a_microphone_unplugged_ends_the_listening_with_why_and_lets_it_go() {
    let h = harness("unplugged", "", 16_000);
    put_model(&h.voice.store, "base");
    let id = session(h.voice.start(&choice("base", "fr")).unwrap());
    h.script.fail(VoiceError::NoDevice);
    std::thread::sleep(Duration::from_millis(150));
    let ended = h.events.lock().iter().find_map(|e| match e {
        VoiceEvent::Ended { session, error } if *session == id => Some(error.clone()),
        _ => None,
    });
    assert!(ended.flatten().is_some_and(|e| !e.is_empty()));
    assert!(h.script.dropped.load(Ordering::Acquire));
    h.voice.cancel();
}

#[test]
fn a_listening_left_on_stops_by_itself() {
    let mut h = harness("max", "fin", 16_000);
    let events = h.events.clone();
    h.voice = Voice::new(
        Box::new(h.script.clone()),
        h.fake.clone(),
        Store {
            dir: h.dir.join("models"),
        },
        String::new(),
        Arc::new(move |e| events.lock().push(e)),
    )
    .with_max_listen(Duration::from_millis(120));
    put_model(&h.voice.store, "base");
    let id = session(h.voice.start(&choice("base", "fr")).unwrap());
    h.script.push(&voice_tone(16_000, 1.0));
    std::thread::sleep(Duration::from_millis(300));
    assert!(h
        .events
        .lock()
        .iter()
        .any(|e| matches!(e, VoiceEvent::Ended { session, error: None } if *session == id)));
    assert!(h.script.dropped.load(Ordering::Acquire));
    // What was heard until then is still transcribed.
    assert_eq!(h.voice.stop(id).unwrap(), "fin");
}

#[test]
fn cancelling_stops_the_transcription_under_way() {
    let mut h = harness("cancel", "trop tard", 16_000);
    let fake = Arc::new(Fake {
        delay: Duration::from_secs(5),
        ..Fake::new("trop tard")
    });
    h.voice = Voice::new(
        Box::new(h.script.clone()),
        fake.clone(),
        Store {
            dir: h.dir.join("models"),
        },
        String::new(),
        Arc::new(|_| {}),
    );
    let voice = Arc::new(h.voice);
    put_model(&voice.store, "base");
    let id = session(voice.start(&choice("base", "fr")).unwrap());
    h.script.push(&voice_tone(16_000, 1.0));
    let v = voice.clone();
    let started = Instant::now();
    let t = std::thread::spawn(move || v.stop(id));
    std::thread::sleep(Duration::from_millis(100));
    voice.shutdown();
    assert_eq!(t.join().unwrap().unwrap(), "");
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn the_language_of_the_interface_unless_another_is_chosen() {
    assert_eq!(whisper_language("ui", Lang::Fr), "fr");
    assert_eq!(whisper_language("", Lang::En), "en");
    assert_eq!(whisper_language("auto", Lang::Fr), "auto");
    assert_eq!(whisper_language("de", Lang::Fr), "de");
}

#[test]
fn a_wav_file_plays_as_a_microphone_would_hear_it() {
    let dir = crate::paths::test_dir("voice-wav");
    let path = dir.join("dicte.wav");
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: 44_100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(&path, spec).unwrap();
    for i in 0..44_100 {
        let s = ((i as f32 * 0.05).sin() * 10_000.0) as i16;
        w.write_sample(s).unwrap();
        w.write_sample(s).unwrap();
    }
    w.finalize().unwrap();
    let (mono, rate) = source::read_wav(&path).unwrap();
    assert_eq!((mono.len(), rate), (44_100, 44_100));
    let wav = source::WavFile(path);
    let got = Arc::new(Mutex::new(Vec::<f32>::new()));
    let g = got.clone();
    let cap = wav
        .open(
            "",
            Sink {
                samples: Box::new(move |s| g.lock().extend_from_slice(s)),
                error: Box::new(|_| {}),
            },
        )
        .unwrap();
    assert_eq!(cap.rate(), 44_100);
    std::thread::sleep(Duration::from_millis(300));
    drop(cap);
    let n = got.lock().len();
    // In real time: about 0.3 s of it, not the whole second at once.
    assert!(n > 4_000 && n < 30_000, "{n}");
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(got.lock().len(), n, "played after the capture was dropped");
}

// ---------------------------------------------------------------------------------------------
// The download of a model, from a fake server.

/// Serves `body` at every path (with `status`), as many times as asked, on a free port.
async fn serve(body: Vec<u8>, status: u16) -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let asked = Arc::new(Mutex::new(Vec::new()));
    let a = asked.clone();
    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                return;
            };
            let body = body.clone();
            let a = a.clone();
            tokio::spawn(async move {
                let mut buf = vec![0u8; 4096];
                let n = sock.read(&mut buf).await.unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                a.lock().push(req.lines().next().unwrap_or("").to_string());
                let head = format!(
                    "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = sock.write_all(head.as_bytes()).await;
                for c in body.chunks(64 * 1024) {
                    let _ = sock.write_all(c).await;
                }
            });
        }
    });
    (format!("http://{addr}"), asked)
}

fn info_for(body: &[u8]) -> ModelInfo {
    use sha2::{Digest, Sha256};
    ModelInfo {
        id: "tiny",
        size: body.len() as u64,
        sha256: Box::leak(format!("{:x}", Sha256::digest(body)).into_boxed_str()),
        speed: 5,
        quality: 1,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_model_downloads_with_its_progress_and_is_kept_once_its_checksum_is_right() {
    let body: Vec<u8> = (0..700_000u32).map(|i| (i % 251) as u8).collect();
    let (url, asked) = serve(body.clone(), 200).await;
    let dir = crate::paths::test_dir("voice-download");
    let store = Store {
        dir: dir.join("models"),
    };
    let m = info_for(&body);
    let mut told = Vec::new();
    let path = store
        .download(
            &reqwest::Client::new(),
            &url,
            &m,
            &AtomicBool::new(false),
            |d| told.push(d),
        )
        .await
        .unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), body);
    assert_eq!(path, store.path("tiny"));
    assert_eq!(told.first(), Some(&0));
    assert_eq!(told.last(), Some(&(body.len() as u64)));
    assert_eq!(asked.lock()[0], "GET /ggml-tiny.bin HTTP/1.1");
    assert!(!dir.join("models").join("ggml-tiny.bin.part").exists());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_download_whose_checksum_is_wrong_is_not_kept() {
    let body = vec![7u8; 10_000];
    let (url, _) = serve(body.clone(), 200).await;
    let dir = crate::paths::test_dir("voice-checksum");
    let store = Store {
        dir: dir.join("models"),
    };
    let m = ModelInfo {
        sha256: "00",
        ..info_for(&body)
    };
    let e = store
        .download(
            &reqwest::Client::new(),
            &url,
            &m,
            &AtomicBool::new(false),
            |_| {},
        )
        .await
        .unwrap_err();
    assert!(e.is::<models::ChecksumMismatch>(), "{e:#}");
    assert!(!store.path("tiny").exists());
    assert!(std::fs::read_dir(&store.dir).unwrap().next().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_download_cut_short_or_refused_is_not_kept() {
    let body = vec![1u8; 5_000];
    let (url, _) = serve(body.clone(), 200).await;
    let dir = crate::paths::test_dir("voice-short");
    let store = Store {
        dir: dir.join("models"),
    };
    // The catalog says more than what comes.
    let m = ModelInfo {
        size: 9_000,
        ..info_for(&body)
    };
    assert!(store
        .download(
            &reqwest::Client::new(),
            &url,
            &m,
            &AtomicBool::new(false),
            |_| {}
        )
        .await
        .is_err());
    assert!(!store.path("tiny").exists());
    let (url, _) = serve(b"nope".to_vec(), 404).await;
    assert!(store
        .download(
            &reqwest::Client::new(),
            &url,
            &info_for(&body),
            &AtomicBool::new(false),
            |_| {}
        )
        .await
        .is_err());
    assert!(!store.path("tiny").exists());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_download_called_off_leaves_nothing() {
    let body = vec![3u8; 3_000_000];
    let (url, _) = serve(body.clone(), 200).await;
    let dir = crate::paths::test_dir("voice-calloff");
    let store = Store {
        dir: dir.join("models"),
    };
    let e = store
        .download(
            &reqwest::Client::new(),
            &url,
            &info_for(&body),
            &AtomicBool::new(true),
            |_| {},
        )
        .await
        .unwrap_err();
    assert!(e.is::<models::Cancelled>(), "{e:#}");
    assert!(std::fs::read_dir(&store.dir).unwrap().next().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_download_goes_through_the_proxy_of_the_settings() {
    // The fake server stands for the proxy: asked for the full address, it answers the model.
    let body = vec![9u8; 20_000];
    let (proxy, asked) = serve(body.clone(), 200).await;
    let settings = crate::model::Settings {
        proxy_url: proxy,
        ..Default::default()
    };
    let client = models::client(&settings).unwrap();
    let dir = crate::paths::test_dir("voice-proxy");
    let store = Store {
        dir: dir.join("models"),
    };
    store
        .download(
            &client,
            "http://models.example.invalid/whisper",
            &info_for(&body),
            &AtomicBool::new(false),
            |_| {},
        )
        .await
        .unwrap();
    assert_eq!(
        asked.lock()[0],
        "GET http://models.example.invalid/whisper/ggml-tiny.bin HTTP/1.1"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_service_tells_the_progress_and_deletes_a_model() {
    let h = harness("service-dl", "", 16_000);
    // "tiny" of the catalog would be 77 MB: the service is checked on what it refuses and
    // deletes, the download itself above.
    assert!(h
        .voice
        .download(&reqwest::Client::new(), "nope")
        .await
        .is_err());
    put_model(&h.voice.store, "tiny");
    assert!(h
        .voice
        .models()
        .iter()
        .any(|m| m.id == "tiny" && m.downloaded));
    // Already there: nothing downloaded.
    h.voice
        .download(&reqwest::Client::new(), "tiny")
        .await
        .unwrap();
    h.voice.delete_model("tiny").unwrap();
    assert!(!h.voice.models().iter().any(|m| m.downloaded));
    assert!(h.voice.delete_model("nope").is_err());
    let base = h
        .voice
        .models()
        .into_iter()
        .find(|m| m.id == "base")
        .unwrap();
    assert_eq!(base.size, 147_951_465);
}

/// The real thing, when a model and a WAV of speech are given (`ESCOUADE_VOICE_TEST_MODEL`,
/// `ESCOUADE_VOICE_TEST_WAV`, `ESCOUADE_VOICE_TEST_EXPECT`): run by hand, the CI has neither.
#[test]
fn whisper_transcribes_a_wav_of_speech_when_a_model_is_given() {
    let (Ok(model), Ok(wav)) = (
        std::env::var("ESCOUADE_VOICE_TEST_MODEL"),
        std::env::var("ESCOUADE_VOICE_TEST_WAV"),
    ) else {
        return;
    };
    let lang = std::env::var("ESCOUADE_VOICE_TEST_LANG").unwrap_or_else(|_| "fr".into());
    let (mono, rate) = source::read_wav(std::path::Path::new(&wav)).unwrap();
    let pcm = audio::resample(&mono, rate, audio::WHISPER_RATE);
    let w = transcribe::Whisper::default();
    let started = Instant::now();
    let text = w
        .transcribe(
            std::path::Path::new(&model),
            &pcm,
            &lang,
            &Arc::new(AtomicBool::new(false)),
        )
        .unwrap();
    eprintln!(
        "transcribed in {} ms: {text}",
        started.elapsed().as_millis()
    );
    if let Ok(expect) = std::env::var("ESCOUADE_VOICE_TEST_EXPECT") {
        let norm = |s: &str| s.to_lowercase().replace(|c: char| !c.is_alphanumeric(), "");
        for word in expect.split(',') {
            assert!(
                norm(&text).contains(&norm(word)),
                "« {word} » not in « {text} »"
            );
        }
    }
}

#[test]
fn called_off_while_the_microphone_opens_it_never_listens() {
    let h = harness("opening", "", 16_000);
    put_model(&h.voice.store, "base");
    *h.script.delay.lock() = Duration::from_millis(300);
    let voice = Arc::new(h.voice);
    let v = voice.clone();
    let t = std::thread::spawn(move || v.start(&choice("base", "fr")));
    std::thread::sleep(Duration::from_millis(80));
    // A second start meanwhile is refused, and the cancel does not wait for the open.
    assert_eq!(
        voice.start(&choice("base", "fr")).unwrap_err(),
        VoiceError::AlreadyListening
    );
    let asked = Instant::now();
    voice.cancel();
    assert!(asked.elapsed() < Duration::from_millis(100));
    assert!(t.join().unwrap().is_err());
    assert_eq!(voice.listening(), None);
    assert!(h.script.dropped.load(Ordering::Acquire));
    // The next one listens.
    *h.script.delay.lock() = Duration::ZERO;
    assert!(matches!(
        voice.start(&choice("base", "fr")).unwrap(),
        Started::Listening { .. }
    ));
    voice.cancel();
}

include!("conv_tests.rs");

fn online_choice(base: &str) -> Choice {
    let mut spec = online::Spec::new(online::Provider::Groq, "gsk-test-key".into());
    spec.base = base.into();
    Choice {
        online: Some(spec),
        ..choice("base", "fr")
    }
}

#[test]
fn on_line_no_model_is_needed_and_the_fake_transcriber_stands_for_the_service() {
    let mut h = harness("online-fake", "Bonjour en ligne", 16_000);
    h.voice.fake_online = true;
    // No model on the disk: it listens all the same.
    let id = session(h.voice.start(&online_choice("http://127.0.0.1:9")).unwrap());
    h.script.push(&voice_tone(16_000, 1.0));
    assert_eq!(h.voice.stop(id).unwrap(), "Bonjour en ligne");
    assert_eq!(*h.fake.calls.lock(), vec![(16_000, "fr".to_string())]);
    assert!(!h.voice.store.dir.exists());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn on_line_the_text_comes_from_the_service_and_its_failure_is_told_as_such() {
    let (url, asked) = online::tests::serve(200, r#"{"text":"Depuis le service"}"#).await;
    let h = harness("online-real", "never this", 16_000);
    let id = session(h.voice.start(&online_choice(&url)).unwrap());
    h.script.push(&voice_tone(16_000, 1.0));
    let v = Arc::new(h.voice);
    let v2 = v.clone();
    let text = tokio::task::spawn_blocking(move || v2.stop(id))
        .await
        .unwrap();
    assert_eq!(text.unwrap(), "Depuis le service");
    assert_eq!(
        asked.lock()[0].line,
        "POST /v1/audio/transcriptions HTTP/1.1"
    );
    assert!(h.fake.calls.lock().is_empty());
    // A key refused: the window says so, as the service's.
    let (url, _) = online::tests::serve(401, r#"{"error":{"message":"Invalid API Key"}}"#).await;
    let id = session(v.start(&online_choice(&url)).unwrap());
    h.script.push(&voice_tone(16_000, 1.0));
    let v2 = v.clone();
    let e = tokio::task::spawn_blocking(move || v2.stop(id))
        .await
        .unwrap()
        .unwrap_err();
    assert_eq!(
        e,
        VoiceError::Online(online::OnlineError::KeyRefused(online::Provider::Groq))
    );
    assert_eq!(
        e.message(crate::i18n::Lang::Fr, "windows"),
        "Groq refuse la clé. Vérifie-la dans Réglages › Voix."
    );
}

#[test]
fn the_settings_choose_the_service_only_when_on_line_is_on() {
    let mut s = crate::model::VoiceSettings::default();
    assert_eq!(s.transcription, "local");
    assert_eq!(online_provider(&s), None);
    s.online_service = "deepgram".into();
    assert_eq!(online_provider(&s), None);
    s.transcription = "online".into();
    assert_eq!(online_provider(&s), Some(online::Provider::Deepgram));
    s.online_service = "nope".into();
    assert_eq!(online_provider(&s), None);
}

// « Transcription en temps réel » for the dictation (ESC-136): the sound streamed as it is heard, the text written in its
// panel, the final one given at the stop.

/// A dictation heard by `f`'s ElevenLabs, as one speaks.
fn live_choice(f: &live::tests::FakeLive) -> Choice {
    Choice {
        online: Some(f.spec(online::Provider::ElevenLabs)),
        ..choice("base", "fr")
    }
}

/// Pushes `secs` of a voice in real time (20 ms at a time).
fn speak(h: &Harness, secs: f32) {
    for chunk in voice_tone(16_000, secs).chunks(320) {
        h.script.push(chunk);
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn dictated_as_one_speaks_the_text_is_written_and_the_final_one_comes_at_the_stop() {
    let f = live::tests::fake(online::Provider::ElevenLabs, live::tests::Mode::Speaks);
    let h = harness("dictation-live", "never this", 16_000);
    let id = session(h.voice.start(&live_choice(&f)).unwrap());
    speak(&h, 0.8);
    let end = Instant::now() + Duration::from_secs(5);
    while !h
        .events
        .lock()
        .iter()
        .any(|e| matches!(e, VoiceEvent::Partial { .. }))
    {
        assert!(Instant::now() < end, "no text while it was said");
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(h.events.lock().contains(&VoiceEvent::Partial {
        session: id,
        text: live::tests::PARTIAL.into()
    }));
    let stopped = Instant::now();
    assert_eq!(h.voice.stop(id).unwrap(), live::tests::FINAL);
    assert!(stopped.elapsed() < Duration::from_secs(2));
    // All of it went, once; neither Whisper nor a request.
    let c = f.conns();
    assert_eq!(c.len(), 1);
    assert!(c[0].ended);
    assert!(c[0].audio >= 32_000 * 7 / 10, "{} bytes", c[0].audio);
    assert!(f.requests.lock().is_empty());
    assert!(h.fake.calls.lock().is_empty());
}

#[test]
fn a_dictation_whose_stream_fails_goes_whole_by_request_never_lost() {
    for mode in [
        live::tests::Mode::Fails,
        live::tests::Mode::Refuses(401),
        live::tests::Mode::Mute,
    ] {
        let f = live::tests::fake(online::Provider::ElevenLabs, mode);
        let h = harness("dictation-live-fails", "never this", 16_000);
        let id = session(h.voice.start(&live_choice(&f)).unwrap());
        speak(&h, 0.5);
        assert_eq!(
            h.voice.stop(id).unwrap(),
            live::tests::BY_REQUEST,
            "{mode:?}"
        );
        assert_eq!(
            *f.requests.lock(),
            ["POST /v1/speech-to-text HTTP/1.1"],
            "{mode:?}"
        );
    }
}

#[test]
fn a_dictation_unticked_or_with_the_fake_transcriber_streams_nothing() {
    let f = live::tests::fake(online::Provider::ElevenLabs, live::tests::Mode::Speaks);
    let h = harness("dictation-live-off", "never this", 16_000);
    let mut c = live_choice(&f);
    c.online = c.online.map(|s| s.with_live(None));
    let id = session(h.voice.start(&c).unwrap());
    speak(&h, 0.3);
    assert_eq!(h.voice.stop(id).unwrap(), live::tests::BY_REQUEST);
    assert!(f.conns().is_empty());
    // The end-to-end tests' fake transcriber stands for the service: no stream either.
    let mut h = harness("dictation-live-fake", "Bonjour", 16_000);
    h.voice.fake_online = true;
    let id = session(h.voice.start(&live_choice(&f)).unwrap());
    speak(&h, 0.3);
    assert_eq!(h.voice.stop(id).unwrap(), "Bonjour");
    assert!(f.conns().is_empty());
}
