// The conversation (ESC-97): the microphone scripted in real time, a fake voice heard nowhere. Included in `tests.rs`,
// whose harness it uses.

use super::conversation::{ConvChoice, ConvStarted};
use super::speech::{self, SpeakOptions};

/// A voice of the catalog on the disk: its two files, of their sizes.
fn put_voice(v: &Voice, id: &str) {
    let info = speech::catalog::find(id).unwrap();
    std::fs::create_dir_all(&v.voices.dir).unwrap();
    std::fs::File::create(v.voices.model(id)).unwrap().set_len(info.onnx_size).unwrap();
    std::fs::File::create(v.voices.config(id)).unwrap().set_len(info.json_size).unwrap();
}

struct Conv {
    h: Harness,
    engine: Arc<speech::Fake>,
}

fn conv_harness(name: &str, text: &str) -> Conv {
    conv_harness_with(name, text, speech::Fake::new(None))
}

fn conv_harness_with(name: &str, text: &str, engine: speech::Fake) -> Conv {
    let mut h = harness(name, text, 16_000);
    let engine = Arc::new(engine);
    h.voice = h.voice.with_speech(engine.clone(), Arc::new(speech::output::Silent));
    Conv { h, engine }
}

fn conv_choice() -> ConvChoice {
    ConvChoice {
        device: String::new(),
        model: "base".into(),
        language: "fr".into(),
        voice: "fr_FR-siwis-medium".into(),
        speak: SpeakOptions::default(),
        volume: 1.0,
        silence_ms: 600,
        online: None,
        online_error: None,
        online_notice: None,
        transcription: None,
        transcription_error: None,
    }
}

/// `secs` of what the microphone hears, in real time: a tone of `amp` (0: a quiet room).
fn hear(h: &Harness, secs: f32, amp: f32) {
    let mut t = 0usize;
    for _ in 0..(secs * 50.0) as usize {
        let chunk: Vec<f32> = (0..320)
            .map(|i| {
                t += 1;
                let noise = ((t * 7919 + i) % 13) as f32 / 13.0 * 0.004 - 0.002;
                (t as f32 * 0.09).sin() * amp + noise
            })
            .collect();
        h.script.push(&chunk);
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn conv_events(h: &Harness) -> Vec<VoiceEvent> {
    h.events
        .lock()
        .iter()
        .filter(|e| !matches!(e, VoiceEvent::ConvLevel { .. } | VoiceEvent::Level { .. }))
        .cloned()
        .collect()
}

/// Waits (5 s at most) for an event `f` accepts.
fn wait_for(h: &Harness, f: impl Fn(&VoiceEvent) -> bool) -> VoiceEvent {
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(e) = h.events.lock().iter().find(|e| f(e)) {
            return e.clone();
        }
        assert!(Instant::now() < end, "never came: {:?}", conv_events(h));
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn listening(s: ConvStarted) -> u64 {
    match s {
        ConvStarted::Listening { session, .. } => session,
        other => panic!("not listening: {other:?}"),
    }
}

#[test]
fn a_conversation_asks_for_whisper_s_model_then_for_its_voice() {
    let c = conv_harness("conv-needs", "");
    let v = &c.h.voice;
    assert!(matches!(v.conversation_start(conv_choice()).unwrap(), ConvStarted::NeedsModel { .. }));
    put_model(&v.store, "base");
    let info = speech::catalog::find("fr_FR-siwis-medium").unwrap();
    assert_eq!(
        v.conversation_start(conv_choice()).unwrap(),
        ConvStarted::NeedsVoice { voice: info.id.clone(), size: info.onnx_size + info.json_size }
    );
    assert!(c.h.script.opened.lock().is_empty(), "the microphone stayed closed");
}

#[test]
fn a_sentence_is_sent_once_its_silence_comes_then_the_passage_is_read_and_it_listens_again() {
    let c = conv_harness("conv-turn", "ajoute un test");
    let h = &c.h;
    put_model(&h.voice.store, "base");
    put_voice(&h.voice, "fr_FR-siwis-medium");
    let session = listening(h.voice.conversation_start(conv_choice()).unwrap());
    // Dictation waits for the conversation to be over.
    assert_eq!(h.voice.start(&choice("base", "fr")).unwrap_err(), VoiceError::AlreadyListening);
    hear(h, 0.4, 0.0);
    hear(h, 1.0, 0.3);
    wait_for(h, |e| matches!(e, VoiceEvent::ConvSpeech { .. }));
    hear(h, 1.0, 0.0);
    let heard = wait_for(h, |e| matches!(e, VoiceEvent::ConvHeard { .. }));
    assert_eq!(heard, VoiceEvent::ConvHeard { session, text: "ajoute un test".into(), error: None });
    // The whole sentence went to Whisper (its first syllables too), at 16 kHz, in French.
    let calls = c.h.fake.calls.lock().clone();
    assert_eq!(calls.len(), 1);
    assert!(calls[0].0 >= 16_000 && calls[0].0 <= 16_000 * 2, "{calls:?}");
    assert_eq!(calls[0].1, "fr");
    // Holding while the agent works: a sentence now is not taken.
    h.voice.conversation_hold(session);
    hear(h, 0.8, 0.3);
    hear(h, 0.8, 0.0);
    assert_eq!(c.h.fake.calls.lock().len(), 1);
    // The passage is read, sentence by sentence; once heard, it listens again.
    let reading = h.voice.conversation_say(session, "C'est fait. Les tests passent.").unwrap();
    let spoken = wait_for(h, |e| matches!(e, VoiceEvent::ConvSpoken { .. }));
    assert_eq!(spoken, VoiceEvent::ConvSpoken { session, reading, error: None, paused: false });
    assert_eq!(*c.engine.said.lock(), vec!["C'est fait.".to_string(), "Les tests passent.".to_string()]);
    hear(h, 0.5, 0.0);
    hear(h, 0.8, 0.3);
    hear(h, 1.0, 0.0);
    let n = conv_events(h).iter().filter(|e| matches!(e, VoiceEvent::ConvHeard { .. })).count();
    assert_eq!(n, 2, "{:?}", conv_events(h));
    h.voice.conversation_stop();
    assert!(h.script.dropped.load(Ordering::Acquire), "the microphone was let go");
    assert_eq!(h.voice.conversing(), None);
}

#[test]
fn each_sentence_read_is_told_as_it_starts_to_be_heard_with_the_level_of_what_is_played() {
    let c = conv_harness("conv-subtitles", "");
    let h = &c.h;
    put_model(&h.voice.store, "base");
    put_voice(&h.voice, "fr_FR-siwis-medium");
    let session = listening(h.voice.conversation_start(conv_choice()).unwrap());
    let started = Instant::now();
    let reading = h
        .voice
        .conversation_say(session, "Première phrase lue. Deuxième phrase.")
        .unwrap();
    hear(h, 0.3, 0.0);
    wait_for(h, |e| matches!(e, VoiceEvent::ConvSpoken { .. }));
    let took = started.elapsed();
    let events = h.events.lock().clone();
    let subtitles: Vec<(u32, String)> = events
        .iter()
        .filter_map(|e| match e {
            VoiceEvent::ConvSentence { session: s, reading: r, index, text } if *s == session && *r == reading => {
                Some((*index, text.clone()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        subtitles,
        vec![(0, "Première phrase lue.".to_string()), (1, "Deuxième phrase.".to_string())]
    );
    // The second one comes once the first was heard (20 characters: 1 s of the fake voice), not at once.
    let second = events
        .iter()
        .position(|e| matches!(e, VoiceEvent::ConvSentence { index: 1, .. }))
        .unwrap();
    let levels_before = events[..second]
        .iter()
        .filter(|e| matches!(e, VoiceEvent::ConvLevel { .. }))
        .count();
    // One each 50 ms at most; the macOS runners' sleeps last longer (6 to 9 seen there in this second).
    assert!(levels_before >= 4, "{levels_before} levels before the second sentence");
    assert!(took >= Duration::from_millis(1700), "{took:?}");
    // What is played is measured: loud while it speaks.
    assert!(events
        .iter()
        .any(|e| matches!(e, VoiceEvent::ConvLevel { played, .. } if *played > 0.5)));
    h.voice.conversation_stop();
}

#[test]
fn a_hold_that_comes_late_does_not_silence_the_reading_under_way() {
    let c = conv_harness("conv-late-hold", "");
    let h = &c.h;
    put_model(&h.voice.store, "base");
    put_voice(&h.voice, "fr_FR-siwis-medium");
    let session = listening(h.voice.conversation_start(conv_choice()).unwrap());
    let reading = h.voice.conversation_say(session, "Tu as choisi SQLite.").unwrap();
    // The window's hold after an answer, arriving once the agent's passage is being read.
    h.voice.conversation_hold(session);
    hear(h, 1.5, 0.0);
    let spoken = wait_for(h, |e| matches!(e, VoiceEvent::ConvSpoken { .. }));
    assert_eq!(spoken, VoiceEvent::ConvSpoken { session, reading, error: None, paused: false });
    h.voice.conversation_stop();
}

#[test]
fn the_agent_s_voice_heard_by_the_microphone_is_neither_a_barge_in_nor_a_sentence() {
    let c = conv_harness("conv-echo", "jamais");
    let h = &c.h;
    put_model(&h.voice.store, "base");
    put_voice(&h.voice, "fr_FR-siwis-medium");
    let session = listening(h.voice.conversation_start(conv_choice()).unwrap());
    hear(h, 0.3, 0.0);
    h.voice.conversation_say(session, "Voici un résumé assez long à lire pour durer.").unwrap();
    // The speakers heard by the microphone, a third as loud as what is played (the fake voice plays at 0.3), for as long
    // as it plays: once it is over, a sound would be the user's.
    let end = Instant::now() + Duration::from_secs(10);
    while !h.events.lock().iter().any(|e| matches!(e, VoiceEvent::ConvSpoken { .. })) {
        assert!(Instant::now() < end, "never read to its end: {:?}", conv_events(h));
        hear(h, 0.1, 0.1);
    }
    let events = conv_events(h);
    assert!(!events.iter().any(|e| matches!(e, VoiceEvent::ConvBargeIn { .. })), "{events:?}");
    hear(h, 0.8, 0.0);
    assert!(c.h.fake.calls.lock().is_empty(), "the echo was transcribed");
    h.voice.conversation_stop();
}

#[test]
fn speaking_over_the_agent_stops_its_reading_and_the_sentence_is_heard() {
    let c = conv_harness("conv-barge", "attends, plutôt la page d'accueil");
    let h = &c.h;
    put_model(&h.voice.store, "base");
    put_voice(&h.voice, "fr_FR-siwis-medium");
    let session = listening(h.voice.conversation_start(conv_choice()).unwrap());
    hear(h, 0.3, 0.0);
    let reading = h
        .voice
        .conversation_say(
            session,
            "Une première phrase assez longue pour durer quatre secondes entières à la lecture.              Une deuxième phrase tout aussi longue, qui dure elle aussi ses quatre secondes.              Une troisième phrase, la dernière, aussi longue que les deux autres réunies.",
        )
        .unwrap();
    // Some echo while the coupling is learnt (the clock of the test may be slow: macOS's coalesced timers).
    hear(h, 0.8, 0.05);
    // The user, close to the microphone, until the agent stops (12 s of reading at most).
    let end = Instant::now() + Duration::from_secs(8);
    while !h.events.lock().iter().any(|e| matches!(e, VoiceEvent::ConvBargeIn { .. })) {
        assert!(Instant::now() < end, "no barge-in: {:?}", conv_events(h));
        hear(h, 0.1, 0.4);
    }
    let barge = wait_for(h, |e| matches!(e, VoiceEvent::ConvBargeIn { .. }));
    assert_eq!(barge, VoiceEvent::ConvBargeIn { session, reading });
    hear(h, 0.6, 0.4);
    hear(h, 1.0, 0.0);
    let heard = wait_for(h, |e| matches!(e, VoiceEvent::ConvHeard { .. }));
    assert!(matches!(heard, VoiceEvent::ConvHeard { ref text, .. } if text == "attends, plutôt la page d'accueil"));
    // The reading stopped: never heard to its end (its sentences were all made by then, the fake voice is quick).
    std::thread::sleep(Duration::from_millis(300));
    assert!(!conv_events(h).iter().any(|e| matches!(e, VoiceEvent::ConvSpoken { .. })));
    assert_eq!(c.engine.said.lock().len(), 3);
    h.voice.conversation_stop();
}

#[test]
fn a_voice_is_downloaded_with_its_size_and_checksum_checked() {
    let dir = crate::paths::test_dir("speech-download");
    let store = speech::VoiceStore { dir: dir.join("voices") };
    let body = b"onnx bytes".to_vec();
    let json = b"{}".to_vec();
    let sha = |b: &[u8]| {
        use sha2::Digest;
        format!("{:x}", sha2::Sha256::digest(b))
    };
    let mut info = speech::catalog::find("fr_FR-siwis-medium").unwrap().clone();
    info.onnx_size = body.len() as u64;
    info.onnx_sha256 = sha(&body);
    info.json_size = json.len() as u64;
    info.json_sha256 = sha(&json);
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let asked = Arc::new(Mutex::new(Vec::<String>::new()));
        let a = asked.clone();
        let (b2, j2) = (body.clone(), json.clone());
        tokio::spawn(async move {
            loop {
                let (mut sock, _) = listener.accept().await.unwrap();
                let mut req = vec![0u8; 4096];
                let n = sock.read(&mut req).await.unwrap();
                let line = String::from_utf8_lossy(&req[..n]).lines().next().unwrap_or_default().to_string();
                a.lock().push(line.clone());
                let bytes = if line.contains(".onnx.json") { j2.clone() } else { b2.clone() };
                let head = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", bytes.len());
                sock.write_all(head.as_bytes()).await.unwrap();
                sock.write_all(&bytes).await.unwrap();
            }
        });
        let client = reqwest::Client::new();
        let base = format!("http://127.0.0.1:{port}");
        let mut seen = Vec::new();
        store.download(&client, &base, &info, &AtomicBool::new(false), |d| seen.push(d)).await.unwrap();
        assert!(store.has(&info));
        assert_eq!(*seen.last().unwrap(), info.onnx_size + info.json_size);
        let asked = asked.lock().clone();
        assert!(
            asked.iter().any(|l| l.contains("/rhasspy/piper-voices/resolve/main/fr/fr_FR/siwis/medium/fr_FR-siwis-medium.onnx ")),
            "{asked:?}"
        );
        // Bytes that are not the voice: not kept.
        store.delete(&info.id).unwrap();
        let mut wrong = info.clone();
        wrong.onnx_sha256 = sha(b"something else");
        let e = store.download(&client, &base, &wrong, &AtomicBool::new(false), |_| {}).await.unwrap_err();
        assert!(e.is::<models::ChecksumMismatch>(), "{e:#}");
        assert!(!store.model(&info.id).exists());
        assert!(!dir.join("voices").join(format!("{}.onnx.part", info.id)).exists());
    });
}

/// A fake voice that takes `ms` to load, as Piper's does (0.9 s on the user's machine), and fails ahead if `fails`.
fn slow_voice(ms: u64, fails: bool) -> speech::Fake {
    let mut f = speech::Fake::new(None);
    f.load_time = Duration::from_millis(ms);
    f.preload_fails = fails;
    f
}

/// Waits (5 s at most) until `n` voices were loaded.
fn wait_loads(c: &Conv, n: usize) -> Vec<speech::FakeLoad> {
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        let loads = c.engine.loads.lock().clone();
        if loads.len() >= n {
            return loads;
        }
        assert!(Instant::now() < end, "{n} loads never came: {loads:?}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn the_voice_is_loaded_as_the_conversation_starts_and_its_first_sentence_does_not_wait_for_it() {
    let c = conv_harness_with("conv-preload", "", slow_voice(600, false));
    let h = &c.h;
    put_model(&h.voice.store, "base");
    put_voice(&h.voice, "fr_FR-siwis-medium");
    let started = Instant::now();
    let session = listening(h.voice.conversation_start(conv_choice()).unwrap());
    // Not on the caller's thread: the start answers at once.
    assert!(started.elapsed() < Duration::from_millis(400), "{:?}", started.elapsed());
    let model = h.voice.voices.model("fr_FR-siwis-medium");
    assert_eq!(wait_loads(&c, 1), vec![speech::FakeLoad { model: model.clone(), ahead: true }]);
    // The agent answers: its first sentence is heard without the voice's loading.
    let said = Instant::now();
    h.voice.conversation_say(session, "C'est fait.").unwrap();
    wait_for(h, |e| matches!(e, VoiceEvent::ConvSentence { index: 0, .. }));
    assert!(said.elapsed() < Duration::from_millis(400), "{:?}", said.elapsed());
    assert_eq!(c.engine.loads.lock().len(), 1, "loaded once");
    // Leaving the conversation keeps it: the next one is quick too.
    h.voice.conversation_stop();
    listening(h.voice.conversation_start(conv_choice()).unwrap());
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(c.engine.loads.lock().len(), 1, "not loaded again");
    h.voice.conversation_stop();
    // Deleted, it goes: the voice downloaded again is loaded again.
    h.voice.delete_voice("fr_FR-siwis-medium").unwrap();
    put_voice(&h.voice, "fr_FR-siwis-medium");
    listening(h.voice.conversation_start(conv_choice()).unwrap());
    assert_eq!(wait_loads(&c, 2)[1], speech::FakeLoad { model, ahead: true });
    h.voice.conversation_stop();
}

#[test]
fn a_voice_that_cannot_be_loaded_ahead_is_loaded_at_its_first_sentence() {
    let c = conv_harness_with("conv-preload-fails", "", slow_voice(0, true));
    let h = &c.h;
    put_model(&h.voice.store, "base");
    put_voice(&h.voice, "fr_FR-siwis-medium");
    let session = listening(h.voice.conversation_start(conv_choice()).unwrap());
    std::thread::sleep(Duration::from_millis(100));
    let reading = h.voice.conversation_say(session, "C'est fait.").unwrap();
    hear(h, 0.3, 0.0);
    let spoken = wait_for(h, |e| matches!(e, VoiceEvent::ConvSpoken { .. }));
    assert_eq!(spoken, VoiceEvent::ConvSpoken { session, reading, error: None, paused: false });
    assert_eq!(
        *c.engine.loads.lock(),
        vec![speech::FakeLoad { model: h.voice.voices.model("fr_FR-siwis-medium"), ahead: false }]
    );
    h.voice.conversation_stop();
}

#[test]
fn a_voice_or_a_language_changed_during_the_conversation_is_loaded_and_taken() {
    let c = conv_harness_with("conv-tune", "hello", slow_voice(300, false));
    let h = &c.h;
    put_model(&h.voice.store, "base");
    put_voice(&h.voice, "fr_FR-siwis-medium");
    let session = listening(h.voice.conversation_start(conv_choice()).unwrap());
    wait_loads(&c, 1);
    // An English voice, not downloaded: the conversation keeps its own.
    let english = ConvChoice { language: "en".into(), voice: "en_US-amy-medium".into(), ..conv_choice() };
    h.voice.conversation_tune(&english);
    std::thread::sleep(Duration::from_millis(400));
    assert_eq!(c.engine.loads.lock().len(), 1);
    // Downloaded: loaded at once, ahead of the next reading, which it reads; what is heard is in English.
    put_voice(&h.voice, "en_US-amy-medium");
    h.voice.conversation_tune(&english);
    let amy = h.voice.voices.model("en_US-amy-medium");
    assert_eq!(wait_loads(&c, 2)[1], speech::FakeLoad { model: amy.clone(), ahead: true });
    h.voice.conversation_say(session, "Done.").unwrap();
    hear(h, 0.3, 0.0);
    wait_for(h, |e| matches!(e, VoiceEvent::ConvSpoken { .. }));
    assert_eq!(c.engine.loads.lock().len(), 2, "read in the voice loaded ahead");
    // Longer than the tail after a reading, which hears nothing: the room's noise is learnt from what comes after.
    hear(h, 0.6, 0.0);
    hear(h, 1.0, 0.3);
    hear(h, 1.0, 0.0);
    wait_for(h, |e| matches!(e, VoiceEvent::ConvHeard { .. }));
    assert_eq!(c.h.fake.calls.lock()[0].1, "en");
    // Back to French: loaded again (one voice is kept at a time, as Piper does).
    h.voice.conversation_tune(&conv_choice());
    assert_eq!(wait_loads(&c, 3)[2].model, h.voice.voices.model("fr_FR-siwis-medium"));
    h.voice.conversation_stop();
}

// The voice on line (ESC-132): a fake service, which reads each sentence as a tone streamed in pieces, in place of
// Piper; its failures, and Piper standing in.

use super::conversation::OnlineVoice;
use super::online::tests::{serve_with, Got, Reply, KEY};
use super::online::{OnlineError, Provider, Spec};
use super::online_speech::SpeakChoice;

/// A fake OpenAI that reads: 50 ms of a tone a character, in pieces of 100 ms; or answers `status` when it is not 200.
struct FakeService {
    _rt: tokio::runtime::Runtime,
    url: String,
    got: Arc<Mutex<Vec<Got>>>,
    status: Arc<std::sync::atomic::AtomicU16>,
}

fn fake_service() -> FakeService {
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
    let status = Arc::new(std::sync::atomic::AtomicU16::new(200));
    let st = status.clone();
    let (url, got) = rt.block_on(serve_with(Arc::new(move |g: &Got| {
        let status = st.load(Ordering::Acquire);
        if status != 200 {
            return Reply::json(status, r#"{"error":{"message":"The server had an error"}}"#);
        }
        let body: serde_json::Value = serde_json::from_slice(&g.body).unwrap();
        let chars = body["input"].as_str().unwrap().chars().count();
        let n = (chars * 24_000 / 20).min(24_000 * 4);
        let pcm: Vec<u8> = (0..n)
            .flat_map(|i| (((i as f32 * 0.09).sin() * 0.3 * 32767.0) as i16).to_le_bytes())
            .collect();
        Reply {
            status: 200,
            content_type: "application/octet-stream".into(),
            chunks: pcm.chunks(4_800).map(<[u8]>::to_vec).collect(),
            gap: Duration::from_millis(20),
        }
    })));
    FakeService { _rt: rt, url, got, status }
}

fn online_voice(url: &str, voice: &str) -> OnlineVoice {
    OnlineVoice {
        spec: Spec { base: url.into(), ..Spec::new(Provider::OpenAi, KEY.into()) },
        choice: SpeakChoice { model: "gpt-4o-mini-tts".into(), voice: voice.into(), lang: "fr".into(), rate: 1.0 },
        default_voice: "marin".into(),
    }
}

impl FakeService {
    fn online(&self, voice: &str) -> OnlineVoice {
        online_voice(&self.url, voice)
    }

    /// The sentences it was asked to read, and with which voice.
    fn asked(&self) -> Vec<(String, String)> {
        self.got
            .lock()
            .iter()
            .map(|g| {
                let b: serde_json::Value = serde_json::from_slice(&g.body).unwrap();
                (b["input"].as_str().unwrap().to_string(), b["voice"].as_str().unwrap().to_string())
            })
            .collect()
    }
}

fn conv_online(o: OnlineVoice) -> ConvChoice {
    ConvChoice { online: Some(o), ..conv_choice() }
}

fn notices(h: &Harness) -> Vec<String> {
    conv_events(h)
        .into_iter()
        .filter_map(|e| match e {
            VoiceEvent::ConvNotice { message, .. } => Some(message),
            _ => None,
        })
        .collect()
}

#[test]
fn the_voice_on_line_reads_the_passage_sentence_by_sentence_without_piper() {
    let c = conv_harness("conv-online", "");
    let h = &c.h;
    let service = fake_service();
    put_model(&h.voice.store, "base");
    // No voice of Piper's downloaded: none is asked for.
    let session = listening(h.voice.conversation_start(conv_online(service.online("cedar"))).unwrap());
    let started = Instant::now();
    let reading = h.voice.conversation_say(session, "C'est fait. Les tests passent.").unwrap();
    let first = wait_for(h, |e| matches!(e, VoiceEvent::ConvSentence { index: 0, .. }));
    let first_at = started.elapsed();
    assert!(matches!(first, VoiceEvent::ConvSentence { ref text, .. } if text == "C'est fait."));
    let spoken = wait_for(h, |e| matches!(e, VoiceEvent::ConvSpoken { .. }));
    let all = started.elapsed();
    assert_eq!(spoken, VoiceEvent::ConvSpoken { session, reading, error: None, paused: false });
    assert_eq!(
        service.asked(),
        [("C'est fait.".to_string(), "cedar".to_string()), ("Les tests passent.".into(), "cedar".into())]
    );
    // Heard in real time, its first sentence long before the end (29 characters: about 1.5 s of sound).
    assert!(all >= Duration::from_millis(1200), "{all:?}");
    assert!(first_at < Duration::from_millis(700), "first sentence heard after {first_at:?}");
    assert!(c.engine.said.lock().is_empty(), "Piper read nothing");
    assert!(conv_events(h).iter().any(|e| matches!(e, VoiceEvent::ConvSentence { index: 1, .. })));
    h.voice.conversation_stop();
}

#[test]
fn speaking_over_the_voice_on_line_stops_it_and_the_sentence_is_heard() {
    let c = conv_harness("conv-online-barge", "attends");
    let h = &c.h;
    let service = fake_service();
    put_model(&h.voice.store, "base");
    let session = listening(h.voice.conversation_start(conv_online(service.online("marin"))).unwrap());
    hear(h, 0.3, 0.0);
    let reading = h
        .voice
        .conversation_say(
            session,
            "Une première phrase assez longue pour durer quatre secondes entières à la lecture. Une deuxième phrase tout aussi longue, qui dure elle aussi ses quatre secondes. Une troisième phrase, la dernière, aussi longue que les deux autres.",
        )
        .unwrap();
    hear(h, 0.8, 0.05);
    let end = Instant::now() + Duration::from_secs(8);
    while !h.events.lock().iter().any(|e| matches!(e, VoiceEvent::ConvBargeIn { .. })) {
        assert!(Instant::now() < end, "no barge-in: {:?}", conv_events(h));
        hear(h, 0.1, 0.4);
    }
    assert_eq!(
        wait_for(h, |e| matches!(e, VoiceEvent::ConvBargeIn { .. })),
        VoiceEvent::ConvBargeIn { session, reading }
    );
    hear(h, 0.6, 0.4);
    hear(h, 1.0, 0.0);
    let heard = wait_for(h, |e| matches!(e, VoiceEvent::ConvHeard { .. }));
    assert!(matches!(heard, VoiceEvent::ConvHeard { ref text, .. } if text == "attends"));
    std::thread::sleep(Duration::from_millis(300));
    assert!(!conv_events(h).iter().any(|e| matches!(e, VoiceEvent::ConvSpoken { .. })));
    h.voice.conversation_stop();
}

#[test]
fn the_service_failing_says_so_once_and_piper_reads_the_rest_of_the_conversation() {
    let c = conv_harness("conv-online-down", "");
    let h = &c.h;
    let service = fake_service();
    service.status.store(503, Ordering::Release);
    put_model(&h.voice.store, "base");
    put_voice(&h.voice, "fr_FR-siwis-medium");
    let session = listening(h.voice.conversation_start(conv_online(service.online("marin"))).unwrap());
    let reading = h.voice.conversation_say(session, "C'est fait. Les tests passent.").unwrap();
    let spoken = wait_for(h, |e| matches!(e, VoiceEvent::ConvSpoken { .. }));
    assert_eq!(spoken, VoiceEvent::ConvSpoken { session, reading, error: None, paused: false });
    assert_eq!(*c.engine.said.lock(), ["C'est fait.", "Les tests passent."]);
    let n = notices(h);
    assert_eq!(n.len(), 1, "{n:?}");
    assert!(n[0].contains("OpenAI") && n[0].contains("503") && n[0].contains("Piper"), "{}", n[0]);
    assert!(!n[0].contains(KEY));
    // The next passage: Piper still, the service not asked again, nothing said again.
    hear(h, 0.5, 0.0);
    h.voice.conversation_say(session, "Encore une.").unwrap();
    wait_for(h, |e| matches!(e, VoiceEvent::ConvSpoken { reading: r, .. } if *r == reading + 1));
    assert_eq!(service.asked().len(), 1);
    assert_eq!(c.engine.said.lock().len(), 3);
    assert_eq!(notices(h).len(), 1);
    h.voice.conversation_stop();
}

#[test]
fn the_service_failing_without_a_voice_of_piper_pauses_the_conversation() {
    let c = conv_harness("conv-online-pause", "");
    let h = &c.h;
    let service = fake_service();
    service.status.store(429, Ordering::Release);
    put_model(&h.voice.store, "base");
    let session = listening(h.voice.conversation_start(conv_online(service.online("marin"))).unwrap());
    let reading = h.voice.conversation_say(session, "C'est fait.").unwrap();
    let spoken = wait_for(h, |e| matches!(e, VoiceEvent::ConvSpoken { .. }));
    let VoiceEvent::ConvSpoken { reading: r, error: Some(error), paused: true, .. } = spoken else {
        panic!("not paused: {spoken:?}");
    };
    assert_eq!(r, reading);
    assert!(error.contains("OpenAI") && error.contains("quota") && error.contains("pause"), "{error}");
    // On pause: what the microphone hears is not taken.
    hear(h, 0.8, 0.3);
    hear(h, 0.8, 0.0);
    assert!(c.h.fake.calls.lock().is_empty());
    // The service back, « Reprendre »: it reads again.
    service.status.store(200, Ordering::Release);
    h.voice.conversation_listen(session);
    h.voice.conversation_say(session, "Reprise.").unwrap();
    let spoken = wait_for(h, |e| matches!(e, VoiceEvent::ConvSpoken { reading: r, .. } if *r == reading + 1));
    assert!(matches!(spoken, VoiceEvent::ConvSpoken { error: None, paused: false, .. }), "{spoken:?}");
    h.voice.conversation_stop();
}

#[test]
fn a_voice_the_service_refuses_is_said_and_its_default_voice_reads() {
    let c = conv_harness("conv-online-voice", "");
    let h = &c.h;
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(1).enable_all().build().unwrap();
    let (url, got) = rt.block_on(serve_with(Arc::new(|g: &Got| {
        let b: serde_json::Value = serde_json::from_slice(&g.body).unwrap();
        if b["voice"] == "gone" {
            return Reply::json(400, r#"{"error":{"message":"Invalid voice gone."}}"#);
        }
        Reply { status: 200, content_type: "audio/pcm".into(), chunks: vec![vec![0x10; 9_600]], gap: Duration::ZERO }
    })));
    put_model(&h.voice.store, "base");
    let session = listening(h.voice.conversation_start(conv_online(online_voice(&url, "gone"))).unwrap());
    h.voice.conversation_say(session, "Un. Deux.").unwrap();
    let spoken = wait_for(h, |e| matches!(e, VoiceEvent::ConvSpoken { .. }));
    assert!(matches!(spoken, VoiceEvent::ConvSpoken { error: None, .. }), "{spoken:?}");
    let voices: Vec<String> = got
        .lock()
        .iter()
        .map(|g| serde_json::from_slice::<serde_json::Value>(&g.body).unwrap()["voice"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(voices, ["gone", "marin", "marin"]);
    let n = notices(h);
    assert_eq!(n.len(), 1);
    assert!(n[0].contains("gone") && n[0].contains("marin") && n[0].contains("Invalid voice"), "{}", n[0]);
    h.voice.conversation_stop();
}

#[test]
fn a_service_without_its_key_leaves_the_reading_to_piper_or_does_not_start() {
    let c = conv_harness("conv-online-nokey", "");
    let h = &c.h;
    put_model(&h.voice.store, "base");
    let no_key = ConvChoice { online_error: Some(OnlineError::NoKey(Provider::ElevenLabs)), ..conv_choice() };
    // No voice of Piper's either: it does not start, and says why.
    assert_eq!(
        h.voice.conversation_start(no_key.clone()).unwrap_err(),
        VoiceError::Online(OnlineError::NoKey(Provider::ElevenLabs))
    );
    put_voice(&h.voice, "fr_FR-siwis-medium");
    let ConvStarted::Listening { session, notice: Some(notice) } = h.voice.conversation_start(no_key).unwrap() else {
        panic!("no notice");
    };
    assert!(notice.contains("ElevenLabs") && notice.contains("Piper"), "{notice}");
    h.voice.conversation_say(session, "Bonjour.").unwrap();
    wait_for(h, |e| matches!(e, VoiceEvent::ConvSpoken { .. }));
    assert_eq!(*c.engine.said.lock(), ["Bonjour."]);
    h.voice.conversation_stop();
}

#[test]
fn the_voice_on_line_chosen_during_the_conversation_reads_the_next_passage() {
    let c = conv_harness("conv-online-tune", "");
    let h = &c.h;
    let service = fake_service();
    put_model(&h.voice.store, "base");
    put_voice(&h.voice, "fr_FR-siwis-medium");
    let session = listening(h.voice.conversation_start(conv_choice()).unwrap());
    h.voice.conversation_say(session, "Piper.").unwrap();
    wait_for(h, |e| matches!(e, VoiceEvent::ConvSpoken { reading: 1, .. }));
    h.voice.conversation_tune(&conv_online(service.online("coral")));
    hear(h, 0.4, 0.0);
    h.voice.conversation_say(session, "En ligne.").unwrap();
    wait_for(h, |e| matches!(e, VoiceEvent::ConvSpoken { reading: 2, .. }));
    assert_eq!(*c.engine.said.lock(), ["Piper."]);
    assert_eq!(service.asked(), [("En ligne.".to_string(), "coral".to_string())]);
    // Back to Piper.
    h.voice.conversation_tune(&conv_choice());
    hear(h, 0.4, 0.0);
    h.voice.conversation_say(session, "Local.").unwrap();
    wait_for(h, |e| matches!(e, VoiceEvent::ConvSpoken { reading: 3, .. }));
    assert_eq!(c.engine.said.lock().len(), 2);
    h.voice.conversation_stop();
}

/// « Transcription : En ligne » (ESC-134): the service of the dictation, at `base`.
fn heard_on_line(base: &str) -> ConvChoice {
    let mut spec = Spec::new(Provider::Groq, "gsk-test-key".into());
    spec.base = base.into();
    ConvChoice { transcription: Some(spec), ..conv_choice() }
}

/// A sentence said, then its silence: what it was heard as.
fn say_a_sentence(h: &Harness) -> VoiceEvent {
    hear(h, 0.4, 0.0);
    hear(h, 1.0, 0.3);
    hear(h, 1.0, 0.0);
    wait_for(h, |e| matches!(e, VoiceEvent::ConvHeard { .. }))
}

#[test]
fn locally_each_sentence_goes_to_whisper_with_its_model_loaded_ahead() {
    let c = conv_harness("conv-local-ear", "en local");
    let h = &c.h;
    put_model(&h.voice.store, "base");
    put_voice(&h.voice, "fr_FR-siwis-medium");
    let session = listening(h.voice.conversation_start(conv_choice()).unwrap());
    assert_eq!(say_a_sentence(h), VoiceEvent::ConvHeard { session, text: "en local".into(), error: None });
    let model = h.voice.store.path("base");
    assert_eq!(*h.fake.models.lock(), std::slice::from_ref(&model));
    assert_eq!(*h.fake.preloaded.lock(), [model]);
    h.voice.conversation_stop();
}

#[test]
fn on_line_the_conversation_starts_without_whisper_s_model_and_the_service_hears_each_sentence() {
    let mut c = conv_harness("conv-online-ear", "depuis le service|la suite");
    c.h.voice.fake_online = true;
    let h = &c.h;
    put_voice(&h.voice, "fr_FR-siwis-medium");
    // No model of Whisper's on the disk: it listens all the same.
    let choice = ConvChoice { language: "auto".into(), ..heard_on_line("http://127.0.0.1:9") };
    let session = listening(h.voice.conversation_start(choice).unwrap());
    assert_eq!(say_a_sentence(h), VoiceEvent::ConvHeard { session, text: "depuis le service".into(), error: None });
    h.voice.conversation_listen(session);
    hear(h, 0.4, 0.0);
    hear(h, 1.0, 0.3);
    hear(h, 1.0, 0.0);
    let n = conv_events(h).iter().filter(|e| matches!(e, VoiceEvent::ConvHeard { .. })).count();
    assert_eq!(n, 2, "{:?}", conv_events(h));
    // Neither a model given nor one loaded; the dictation's language told.
    assert_eq!(*h.fake.models.lock(), [PathBuf::new(), PathBuf::new()]);
    assert!(h.fake.preloaded.lock().is_empty());
    assert!(h.fake.calls.lock().iter().all(|(_, l)| l == "auto"));
    assert!(!h.voice.store.dir.exists());
    h.voice.conversation_stop();
}

#[test]
fn on_line_without_the_service_s_key_the_conversation_does_not_start_as_the_dictation() {
    let c = conv_harness("conv-online-ear-nokey", "");
    let h = &c.h;
    put_voice(&h.voice, "fr_FR-siwis-medium");
    let choice = ConvChoice {
        transcription_error: Some(VoiceError::Online(OnlineError::NoKey(Provider::Groq))),
        ..conv_choice()
    };
    assert_eq!(
        h.voice.conversation_start(choice).unwrap_err(),
        VoiceError::Online(OnlineError::NoKey(Provider::Groq))
    );
    assert!(h.script.opened.lock().is_empty(), "the microphone stayed closed");
}

#[test]
fn on_line_the_text_comes_from_the_service_and_its_failure_is_said_as_the_dictation_says_it() {
    let c = conv_harness("conv-online-ear-real", "never this");
    let h = &c.h;
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(1).enable_all().build().unwrap();
    let refused = Arc::new(AtomicBool::new(false));
    let r = refused.clone();
    let (url, got) = rt.block_on(serve_with(Arc::new(move |_: &Got| {
        if r.load(Ordering::Acquire) {
            Reply::json(401, r#"{"error":{"message":"Invalid API Key"}}"#)
        } else {
            Reply::json(200, r#"{"text":" Depuis le service "}"#)
        }
    })));
    put_voice(&h.voice, "fr_FR-siwis-medium");
    let session = listening(h.voice.conversation_start(heard_on_line(&url)).unwrap());
    assert_eq!(say_a_sentence(h), VoiceEvent::ConvHeard { session, text: "Depuis le service".into(), error: None });
    assert_eq!(got.lock()[0].line, "POST /v1/audio/transcriptions HTTP/1.1");
    assert!(h.fake.calls.lock().is_empty(), "Whisper heard nothing");
    // The key refused: the sentence is said lost once, as the dictation says it, and it listens again.
    refused.store(true, Ordering::Release);
    h.events.lock().clear();
    h.voice.conversation_listen(session);
    let heard = say_a_sentence(h);
    let said = VoiceError::Online(OnlineError::KeyRefused(Provider::Groq))
        .message(crate::i18n::ui(), std::env::consts::OS);
    assert_eq!(heard, VoiceEvent::ConvHeard { session, text: String::new(), error: Some(said) });
    assert_eq!(got.lock().len(), 2, "asked once, never again by itself");
    assert!(h.fake.calls.lock().is_empty());
    h.voice.conversation_stop();
}

/// The real ElevenLabs (Scribe) hears a sentence of the conversation (`testdata/question-fr.wav`, played in real time),
/// with the user's key given to this process only (`ELEVENLABS_API_KEY`): run by hand, never in CI, the key written
/// nowhere. Prints how long the text took once the sentence's silence came.
#[test]
#[ignore]
fn real_elevenlabs_conversation_sentence() {
    let key = std::env::var("ELEVENLABS_API_KEY").expect("ELEVENLABS_API_KEY");
    let c = conv_harness("conv-real-elevenlabs", "never this");
    let h = &c.h;
    put_voice(&h.voice, "fr_FR-siwis-medium");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/voice/testdata/question-fr.wav");
    let (samples, rate) = super::source::read_wav(&path).unwrap();
    let samples = super::audio::resample(&samples, rate, 16_000);
    let secs = samples.len() as f32 / 16_000.0;
    let spec = Spec::new(Provider::ElevenLabs, key);
    let session = listening(h.voice.conversation_start(ConvChoice { transcription: Some(spec), ..conv_choice() }).unwrap());
    // Played on a thread of its own, the room quiet after it; the events watched every 5 ms meanwhile.
    let script = h.script.clone();
    let quiet = vec![0.001f32; 320];
    let player = std::thread::spawn(move || {
        for chunk in std::iter::repeat_n(&quiet[..], 30).chain(samples.chunks(320)).chain(std::iter::repeat_n(&quiet[..], 150)) {
            script.push(chunk);
            std::thread::sleep(Duration::from_millis(20));
        }
    });
    let mut ended = None;
    let end = Instant::now() + Duration::from_secs(30);
    let heard = loop {
        let events = conv_events(h);
        if ended.is_none() && events.iter().any(|e| matches!(e, VoiceEvent::ConvTranscribing { .. })) {
            ended = Some(Instant::now());
        }
        if let Some(e) = events.into_iter().find(|e| matches!(e, VoiceEvent::ConvHeard { .. })) {
            break e;
        }
        assert!(Instant::now() < end, "never heard: {:?}", conv_events(h));
        std::thread::sleep(Duration::from_millis(5));
    };
    println!(
        "{secs:.1} s of sound, heard by ElevenLabs {} ms after its silence: {heard:?}",
        ended.map(|t| t.elapsed().as_millis()).unwrap_or_default()
    );
    player.join().unwrap();
    let VoiceEvent::ConvHeard { session: s, text, error: None } = heard else {
        panic!("not heard: {heard:?}");
    };
    assert_eq!(s, session);
    assert!(!text.is_empty());
    assert!(h.fake.calls.lock().is_empty(), "Whisper heard nothing");
    h.voice.conversation_stop();
}

// « Transcription en temps réel » (ESC-136): each sentence streamed to a fake service as it is said.

use super::live::tests::{fake as fake_live, Mode as LiveMode, BY_REQUEST, FINAL, PARTIAL};

/// The conversation heard by `f`'s ElevenLabs, as one speaks.
fn heard_live(f: &super::live::tests::FakeLive) -> ConvChoice {
    ConvChoice { transcription: Some(f.spec(Provider::ElevenLabs)), ..conv_choice() }
}

/// The bytes of 16 kHz sound (16 bits) of `secs`.
fn pcm_bytes(secs: f32) -> usize {
    (secs * 32_000.0) as usize
}

#[test]
fn as_one_speaks_the_text_is_written_and_its_final_one_goes_to_the_agent_without_the_room_before_it() {
    let f = fake_live(Provider::ElevenLabs, LiveMode::Speaks);
    let c = conv_harness("conv-live", "never this");
    let h = &c.h;
    put_voice(&h.voice, "fr_FR-siwis-medium");
    let session = listening(h.voice.conversation_start(heard_live(&f)).unwrap());
    // A quiet room: nothing opens, nothing goes.
    hear(h, 1.0, 0.0);
    assert!(f.conns().is_empty());
    hear(h, 1.0, 0.3);
    // Written while it is said, before its end.
    let partial = wait_for(h, |e| matches!(e, VoiceEvent::ConvPartial { .. }));
    assert_eq!(partial, VoiceEvent::ConvPartial { session, text: PARTIAL.into() });
    assert!(!conv_events(h).iter().any(|e| matches!(e, VoiceEvent::ConvTranscribing { .. })));
    hear(h, 1.0, 0.0);
    let heard = wait_for(h, |e| matches!(e, VoiceEvent::ConvHeard { .. }));
    assert_eq!(heard, VoiceEvent::ConvHeard { session, text: FINAL.into(), error: None });
    // Committed once, at its end; nothing by request, nothing by Whisper.
    let conns = f.conns();
    assert_eq!(conns.len(), 1);
    assert_eq!(conns[0].commits, 1, "{:?}", conns[0].texts);
    assert!(f.requests.lock().is_empty());
    assert!(h.fake.calls.lock().is_empty());
    // What went: the sentence (its first syllables kept from before, 0.54 s at most) and the silence that ended it
    // (0.6 s), as the request sends it; never the second of quiet before it, nor what came after its end.
    let sent = conns[0].audio;
    assert!(sent >= pcm_bytes(1.0 + 0.6), "{sent} bytes");
    assert!(sent <= pcm_bytes(0.54 + 1.0 + 0.6 + 0.05), "{sent} bytes: the room went");
    h.voice.conversation_stop();
}

#[test]
fn nothing_goes_while_the_conversation_holds_and_it_streams_again_once_it_listens() {
    let f = fake_live(Provider::ElevenLabs, LiveMode::Speaks);
    let c = conv_harness("conv-live-hold", "never this");
    let h = &c.h;
    put_voice(&h.voice, "fr_FR-siwis-medium");
    let session = listening(h.voice.conversation_start(heard_live(&f)).unwrap());
    // The agent works: the user speaks, nothing goes.
    h.voice.conversation_hold(session);
    hear(h, 0.4, 0.0);
    hear(h, 1.0, 0.3);
    hear(h, 0.6, 0.0);
    assert!(f.conns().is_empty(), "{:?}", f.conns());
    // A sentence started, then a hold: its stream closes, nothing more sent, no text.
    h.voice.conversation_listen(session);
    hear(h, 0.4, 0.0);
    hear(h, 0.6, 0.3);
    h.voice.conversation_hold(session);
    hear(h, 1.0, 0.3);
    hear(h, 0.6, 0.0);
    let conns = f.conns();
    assert_eq!(conns.len(), 1);
    assert!(!conns[0].ended);
    assert!(conns[0].audio <= pcm_bytes(0.54 + 0.6 + 0.05), "{} bytes", conns[0].audio);
    assert!(!conv_events(h).iter().any(|e| matches!(e, VoiceEvent::ConvHeard { .. })));
    // Listening again: the sentence cut by the hold goes on, whole, by request (never lost); the next is streamed.
    h.voice.conversation_listen(session);
    assert_eq!(say_a_sentence(h), VoiceEvent::ConvHeard { session, text: BY_REQUEST.into(), error: None });
    h.events.lock().clear();
    h.voice.conversation_listen(session);
    assert_eq!(say_a_sentence(h), VoiceEvent::ConvHeard { session, text: FINAL.into(), error: None });
    assert_eq!(f.conns().len(), 2);
    h.voice.conversation_stop();
}

#[test]
fn a_stream_that_fails_sends_the_sentence_whole_by_request_and_the_next_ones_too() {
    for mode in [LiveMode::Fails, LiveMode::Closes, LiveMode::Refuses(403)] {
        let f = fake_live(Provider::ElevenLabs, mode);
        let c = conv_harness("conv-live-fails", "never this");
        let h = &c.h;
        put_voice(&h.voice, "fr_FR-siwis-medium");
        let session = listening(h.voice.conversation_start(heard_live(&f)).unwrap());
        assert_eq!(
            say_a_sentence(h),
            VoiceEvent::ConvHeard { session, text: BY_REQUEST.into(), error: None },
            "{mode:?}"
        );
        assert_eq!(*f.requests.lock(), ["POST /v1/speech-to-text HTTP/1.1"], "{mode:?}");
        // The next sentence goes by request at once: no stream tried again.
        h.events.lock().clear();
        h.voice.conversation_listen(session);
        assert_eq!(say_a_sentence(h), VoiceEvent::ConvHeard { session, text: BY_REQUEST.into(), error: None });
        assert_eq!(f.conns().len(), 1, "{mode:?}");
        assert_eq!(f.requests.lock().len(), 2, "{mode:?}");
        h.voice.conversation_stop();
    }
}

#[test]
fn unticked_each_sentence_goes_whole_as_before() {
    let f = fake_live(Provider::ElevenLabs, LiveMode::Speaks);
    let c = conv_harness("conv-live-off", "never this");
    let h = &c.h;
    put_voice(&h.voice, "fr_FR-siwis-medium");
    let spec = f.spec(Provider::ElevenLabs).with_live(None);
    let session = listening(h.voice.conversation_start(ConvChoice { transcription: Some(spec), ..conv_choice() }).unwrap());
    assert_eq!(say_a_sentence(h), VoiceEvent::ConvHeard { session, text: BY_REQUEST.into(), error: None });
    assert!(f.conns().is_empty());
    assert!(!conv_events(h).iter().any(|e| matches!(e, VoiceEvent::ConvPartial { .. })));
    h.voice.conversation_stop();
}

// The whole sentence goes to its stream (ESC-137): its quiet, its soft start, its end said low, its soft resume after a
// pause; nothing before it, nothing after its end, and a single commit, at its end.

/// `testdata/question-fr.wav` at 16 kHz: its speech from 0.88 s to 4.56 s, a pause in it from 3.2 s to 3.62 s.
fn question() -> Vec<f32> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/voice/testdata/question-fr.wav");
    let (samples, rate) = super::source::read_wav(&path).unwrap();
    super::audio::resample(&samples, rate, 16_000)
}

const SPEECH_FROM: f32 = 0.88;
const SPEECH_TO: f32 = 4.56;

fn at(secs: f32) -> usize {
    (secs * 16_000.0) as usize
}

/// `s` with what is said from `from` to `to` (seconds) brought down to a whisper, under what the endpointer hears.
fn said_low(mut s: Vec<f32>, from: f32, to: f32) -> Vec<f32> {
    for x in &mut s[at(from)..at(to)] {
        *x *= 0.03;
    }
    s
}

/// The test sounds: a start said low (240 ms), an end said low (« la suite », 560 ms), and a pause made longer
/// (720 ms) with the words after it resumed low (240 ms).
fn soft_start() -> Vec<f32> {
    said_low(question(), SPEECH_FROM, SPEECH_FROM + 0.24)
}

fn soft_end() -> Vec<f32> {
    said_low(question(), 4.0, SPEECH_TO)
}

/// Its speech then ends 0.3 s later.
fn pause_then_soft_resume() -> Vec<f32> {
    let mut s = said_low(question(), 3.62, 3.86);
    let pause = vec![0f32; at(0.3)];
    s.splice(at(3.4)..at(3.4), pause);
    s
}

/// What the stream must have got: the 16-bit sound the service is sent.
fn pcm16(samples: &[f32]) -> Vec<i16> {
    samples.iter().map(|s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16).collect()
}

/// `sound` said to a conversation heard live by a fake ElevenLabs (with « Délai avant l'envoi » at its default,
/// 1.2 s), after a quiet room; what its stream got, and how many commits.
fn streamed(name: &str, sound: &[f32]) -> (Vec<i16>, usize) {
    let f = fake_live(Provider::ElevenLabs, LiveMode::Speaks);
    let c = conv_harness(name, "never this");
    let h = &c.h;
    put_voice(&h.voice, "fr_FR-siwis-medium");
    let choice = ConvChoice { silence_ms: 1200, ..heard_live(&f) };
    let session = listening(h.voice.conversation_start(choice).unwrap());
    // Played four times faster than said: the conversation takes its frames as they come.
    for chunk in played(sound).chunks(320) {
        h.script.push(chunk);
        std::thread::sleep(Duration::from_millis(5));
    }
    let heard = wait_for(h, |e| matches!(e, VoiceEvent::ConvHeard { .. }));
    assert_eq!(heard, VoiceEvent::ConvHeard { session, text: FINAL.into(), error: None });
    h.voice.conversation_stop();
    let conns = f.conns();
    assert_eq!(conns.len(), 1, "{conns:?}");
    let pcm = conns[0].pcm.chunks(2).map(|b| i16::from_le_bytes([b[0], b[1]])).collect();
    (pcm, conns[0].commits)
}

/// What the microphone hears: 0.6 s of a quiet room, then `sound`.
fn played(sound: &[f32]) -> Vec<f32> {
    let mut all = vec![0.001f32; at(0.6)];
    all.extend_from_slice(sound);
    all
}

/// `got` is the sound played from its start to its end, in order, without a hole: where it starts and ends in it.
fn span_of(got: &[i16], sound: &[f32]) -> (usize, usize) {
    let all = pcm16(&played(sound));
    // ElevenLabs' commit may ride on a hundredth of a second of zeros.
    let mut got = got;
    if !all.windows(got.len()).any(|w| w == got) && got.len() > 160 && got[got.len() - 160..].iter().all(|x| *x == 0) {
        got = &got[..got.len() - 160];
    }
    let start = all
        .windows(got.len().max(1))
        .position(|w| w == got)
        .unwrap_or_else(|| panic!("{} samples got, not the sound played in one piece (a hole, or out of order)", got.len()));
    (start, start + got.len())
}

/// The sentence went whole: from a little before its first sound (its lead, 0.5 s at most) to its end and the silence
/// that ended it (no more), with its single commit at the end.
fn assert_whole(got: &[i16], commits: usize, sound: &[f32], speech_to: f32) {
    let (start, end) = span_of(got, sound);
    let lead = at(0.6);
    let (from, to) = (lead + at(SPEECH_FROM), lead + at(speech_to));
    assert!(start <= from, "its start is cut: sent from {:.2} s, it starts at {:.2} s", start as f32 / 16e3, from as f32 / 16e3);
    assert!(start + at(0.5) >= from, "sent from {:.2} s, well before its start", start as f32 / 16e3);
    assert!(end >= to, "its end is cut: sent up to {:.2} s, it ends at {:.2} s", end as f32 / 16e3, to as f32 / 16e3);
    assert!(end <= to + at(1.2 + 0.2), "sent up to {:.2} s, past its silence", end as f32 / 16e3);
    assert_eq!(commits, 1, "committed before the end of the sentence");
}

#[test]
fn a_start_said_low_goes_to_the_stream() {
    let sound = soft_start();
    let (got, commits) = streamed("conv-live-soft-start", &sound);
    assert_whole(&got, commits, &sound, SPEECH_TO);
}

#[test]
fn an_end_said_low_goes_to_the_stream() {
    let sound = soft_end();
    let (got, commits) = streamed("conv-live-soft-end", &sound);
    assert_whole(&got, commits, &sound, SPEECH_TO);
}

#[test]
fn a_pause_and_the_soft_words_after_it_go_to_the_stream_without_a_commit() {
    let sound = pause_then_soft_resume();
    let (got, commits) = streamed("conv-live-pause", &sound);
    assert_whole(&got, commits, &sound, SPEECH_TO + 0.3);
}

/// The real Scribe v2 Realtime hears a sentence of the conversation as it is said (`testdata/question-fr.wav`, in real
/// time), the user's key given to this process only (`ELEVENLABS_API_KEY`): run by hand, never in CI. Prints the texts
/// written while it was said, and when the final one was there.
#[test]
#[ignore]
fn real_elevenlabs_conversation_live() {
    let key = std::env::var("ELEVENLABS_API_KEY").expect("ELEVENLABS_API_KEY");
    let c = conv_harness("conv-real-elevenlabs-live", "never this");
    let h = &c.h;
    put_voice(&h.voice, "fr_FR-siwis-medium");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/voice/testdata/question-fr.wav");
    let (samples, rate) = super::source::read_wav(&path).unwrap();
    let samples = super::audio::resample(&samples, rate, 16_000);
    let spec = Spec::new(Provider::ElevenLabs, key).with_live(Some("scribe_v2_realtime".into()));
    let session = listening(h.voice.conversation_start(ConvChoice { transcription: Some(spec), ..conv_choice() }).unwrap());
    let script = h.script.clone();
    let quiet = vec![0.001f32; 320];
    let started = Instant::now();
    let player = std::thread::spawn(move || {
        for chunk in std::iter::repeat_n(&quiet[..], 30).chain(samples.chunks(320)).chain(std::iter::repeat_n(&quiet[..], 150)) {
            script.push(chunk);
            std::thread::sleep(Duration::from_millis(20));
        }
    });
    let mut partials: Vec<(u128, String)> = Vec::new();
    let mut ended = None;
    let end = Instant::now() + Duration::from_secs(30);
    let heard = loop {
        let events = conv_events(h);
        for e in &events {
            if let VoiceEvent::ConvPartial { text, .. } = e {
                if !partials.iter().any(|(_, t)| t == text) {
                    partials.push((started.elapsed().as_millis(), text.clone()));
                }
            }
        }
        if ended.is_none() && events.iter().any(|e| matches!(e, VoiceEvent::ConvTranscribing { .. })) {
            ended = Some(Instant::now());
        }
        if let Some(e) = events.into_iter().find(|e| matches!(e, VoiceEvent::ConvHeard { .. })) {
            break e;
        }
        assert!(Instant::now() < end, "never heard: {:?}", conv_events(h));
        std::thread::sleep(Duration::from_millis(5));
    };
    let after_silence = ended.map(|t| t.elapsed().as_millis()).unwrap_or_default();
    for (at, t) in &partials {
        println!("written at {at} ms: {t}");
    }
    let silence = conv_choice().silence_ms as u128;
    let end_at = ended.map(|t| t.duration_since(started).as_millis()).unwrap_or_default();
    // The last word: the sentence's end less its silence.
    let last_word = end_at.saturating_sub(silence);
    let final_at = partials.last().map(|(at, _)| *at).unwrap_or_default();
    println!(
        "heard {after_silence} ms after the end of the sentence ({silence} ms of silence, at {end_at} ms); its final text written at {final_at} ms, {} ms after the last word: {heard:?}",
        final_at.saturating_sub(last_word)
    );
    player.join().unwrap();
    let VoiceEvent::ConvHeard { session: s, text, error: None } = heard else {
        panic!("not heard: {heard:?}");
    };
    assert_eq!(s, session);
    assert!(!text.is_empty());
    assert!(partials.len() >= 2, "{partials:?}");
    assert!(h.fake.calls.lock().is_empty(), "Whisper heard nothing");
    h.voice.conversation_stop();
}


/// The test sounds of ESC-137 (and the sentence as it was said) heard by the real ElevenLabs, live (Scribe v2 Realtime)
/// then by request (Scribe), in real time, with « Délai avant l'envoi » at its default (1.2 s); the key given to this
/// process only (`ELEVENLABS_API_KEY`): run by hand, never in CI. Prints both texts, and when the final one was there
/// after the end of the sentence (its silence over).
#[test]
#[ignore]
fn real_elevenlabs_live_and_by_request_hear_the_whole_sentence() {
    let key = std::env::var("ELEVENLABS_API_KEY").expect("ELEVENLABS_API_KEY");
    let hear_real = |name: &str, sound: &[f32], live: bool| -> (String, u128) {
        let c = conv_harness(&format!("conv-real-137-{name}-{live}"), "never this");
        let h = &c.h;
        put_voice(&h.voice, "fr_FR-siwis-medium");
        let spec = Spec::new(Provider::ElevenLabs, key.clone())
            .with_live(live.then(|| "scribe_v2_realtime".to_string()));
        let choice = ConvChoice { transcription: Some(spec), silence_ms: 1200, ..conv_choice() };
        let session = listening(h.voice.conversation_start(choice).unwrap());
        let script = h.script.clone();
        let all = played(sound);
        let player = std::thread::spawn(move || {
            for chunk in all.chunks(320) {
                script.push(chunk);
                std::thread::sleep(Duration::from_millis(20));
            }
        });
        let mut ended = None;
        let end = Instant::now() + Duration::from_secs(30);
        let heard = loop {
            let events = conv_events(h);
            if ended.is_none() && events.iter().any(|e| matches!(e, VoiceEvent::ConvTranscribing { .. })) {
                ended = Some(Instant::now());
            }
            if let Some(e) = events.into_iter().find(|e| matches!(e, VoiceEvent::ConvHeard { .. })) {
                break e;
            }
            assert!(Instant::now() < end, "never heard: {:?}", conv_events(h));
            std::thread::sleep(Duration::from_millis(5));
        };
        let after = ended.map(|t| t.elapsed().as_millis()).unwrap_or_default();
        h.voice.conversation_stop();
        player.join().unwrap();
        let VoiceEvent::ConvHeard { session: s, text, error: None } = heard else {
            panic!("not heard: {heard:?}");
        };
        assert_eq!(s, session);
        assert!(h.fake.calls.lock().is_empty(), "Whisper heard nothing");
        (text, after)
    };
    for (name, sound) in [
        ("as-said", question()),
        ("soft-start", soft_start()),
        ("soft-end", soft_end()),
        ("pause-soft-resume", pause_then_soft_resume()),
    ] {
        let (live, live_ms) = hear_real(name, &sound, true);
        let (request, request_ms) = hear_real(name, &sound, false);
        println!("{name}: live, {live_ms} ms after the end: {live:?}");
        println!("{name}: by request, {request_ms} ms after the end: {request:?}");
    }
}
