//! Each service's voice against a fake server that answers as its documentation says: the request (its address, its
//! key, its model, its voice, the language and the pace), its sound streamed in pieces (raw PCM, a WAV file, Mistral's
//! events of float PCM), its refusals (401, 429, 5xx, a voice it does not know) and the network cut.

use super::super::online::tests::{nowhere, serve, serve_with, Got, Reply, KEY};
use super::*;
use crate::voice::online::Provider;
use base64::Engine as _;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// The reading, as its plugin serves it.
async fn speak(
    spec: &Spec,
    choice: &SpeakChoice,
    sentence: &str,
    cancel: &AtomicBool,
    on: &mut dyn FnMut(Audio),
) -> Result<Option<Duration>, OnlineError> {
    crate::voice::service::of(spec.provider)
        .speak(spec, choice, sentence, cancel, on)
        .await
}

fn spec(p: Provider, base: &str) -> Spec {
    Spec {
        base: base.into(),
        ..Spec::new(p, KEY.into())
    }
}

fn choice(model: &str, voice: &str, lang: &str, rate: f32) -> SpeakChoice {
    SpeakChoice {
        model: model.into(),
        voice: voice.into(),
        lang: lang.into(),
        rate,
    }
}

/// `n` samples of a tone, as 16-bit PCM.
fn pcm16(n: usize) -> Vec<u8> {
    (0..n)
        .flat_map(|i| (((i as f32 / 10.0).sin() * 8000.0) as i16).to_le_bytes())
        .collect()
}

/// `n` samples of a tone, as 32-bit float PCM.
fn pcm_f32(n: usize) -> Vec<u8> {
    (0..n)
        .flat_map(|i| ((i as f32 / 10.0).sin() * 0.25).to_le_bytes())
        .collect()
}

/// A stream of `pieces` of `bytes`, `gap` apart.
fn streamed(content_type: &str, bytes: Vec<u8>, pieces: usize, gap_ms: u64) -> Reply {
    let size = bytes.len().div_ceil(pieces);
    Reply {
        status: 200,
        content_type: content_type.into(),
        chunks: bytes.chunks(size).map(<[u8]>::to_vec).collect(),
        gap: Duration::from_millis(gap_ms),
    }
}

/// What reading `sentence` gave: its pieces of sound, and when the first one came.
async fn read(
    s: &Spec,
    c: &SpeakChoice,
    sentence: &str,
) -> Result<(Vec<Audio>, Option<Duration>), OnlineError> {
    let mut got = Vec::new();
    let first = speak(s, c, sentence, &AtomicBool::new(false), &mut |a| {
        got.push(a)
    })
    .await?;
    Ok((got, first))
}

fn json_of(g: &Got) -> serde_json::Value {
    serde_json::from_slice(&g.body).expect("a JSON body")
}

fn samples(pieces: &[Audio]) -> usize {
    pieces.iter().map(|a| a.samples.len()).sum()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn openai_reads_a_sentence_in_raw_pcm_heard_as_it_streams() {
    let reply = streamed("application/octet-stream", pcm16(24_000), 4, 150);
    let (url, got) = serve_with(Arc::new(move |_: &Got| reply.clone())).await;
    let s = spec(Provider::OpenAi, &url);
    let started = Instant::now();
    let (pieces, first) = read(
        &s,
        &choice("gpt-4o-mini-tts", "marin", "fr", 1.0),
        "Les tests passent.",
    )
    .await
    .unwrap();
    let all = started.elapsed();
    assert_eq!(samples(&pieces), 24_000);
    assert!(pieces.iter().all(|a| a.rate == 24_000));
    assert!(pieces.len() >= 2, "streamed in pieces: {}", pieces.len());
    // The first sound came long before the last: it is heard as it streams.
    let first = first.unwrap();
    assert!(
        first + Duration::from_millis(300) < all,
        "{first:?} of {all:?}"
    );
    let g = got.lock()[0].clone();
    assert_eq!(g.line, "POST /v1/audio/speech HTTP/1.1");
    assert_eq!(g.header("authorization"), Some(&*format!("Bearer {KEY}")));
    let b = json_of(&g);
    assert_eq!(b["model"], "gpt-4o-mini-tts");
    assert_eq!(b["input"], "Les tests passent.");
    assert_eq!(b["voice"], "marin");
    assert_eq!(b["response_format"], "pcm");
    assert!(b["instructions"].as_str().unwrap().contains("français"));
    // Its own pace: no speed asked.
    assert!(b.get("speed").is_none());
    read(&s, &choice("tts-1", "alloy", "en", 1.5), "Done.")
        .await
        .unwrap();
    let b = json_of(&got.lock()[1]);
    assert_eq!(
        (b["model"].as_str(), b["speed"].as_f64()),
        (Some("tts-1"), Some(1.5))
    );
    // TTS-1 takes no instructions.
    assert!(b.get("instructions").is_none());
}

/// A WAV file of `n` samples at `rate`, 16 bits, mono, its sizes those of a stream (unknown).
fn wav_stream(n: usize, rate: u32) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&u32::MAX.to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"LIST");
    out.extend_from_slice(&4u32.to_le_bytes());
    out.extend_from_slice(b"INFO");
    out.extend_from_slice(b"data");
    out.extend_from_slice(&u32::MAX.to_le_bytes());
    out.extend_from_slice(&pcm16(n));
    out
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn groq_reads_with_orpheus_in_a_wav_file_its_header_split_across_pieces() {
    // Pieces of 7 bytes: the header comes in several.
    let wav = wav_stream(4_800, 48_000);
    let pieces = wav.len().div_ceil(7);
    let reply = streamed("audio/wav", wav, pieces, 0);
    let (url, got) = serve_with(Arc::new(move |_: &Got| reply.clone())).await;
    let base = format!("{url}/openai");
    let s = spec(Provider::Groq, &base);
    let (pieces, _) = read(
        &s,
        &choice("canopylabs/orpheus-v1-english", "hannah", "en", 1.4),
        "Done.",
    )
    .await
    .unwrap();
    assert_eq!(samples(&pieces), 4_800);
    assert!(pieces.iter().all(|a| a.rate == 48_000));
    let g = got.lock()[0].clone();
    assert_eq!(g.line, "POST /openai/v1/audio/speech HTTP/1.1");
    assert_eq!(g.header("authorization"), Some(&*format!("Bearer {KEY}")));
    let b = json_of(&g);
    assert_eq!(b["model"], "canopylabs/orpheus-v1-english");
    assert_eq!(b["voice"], "hannah");
    assert_eq!(b["response_format"], "wav");
    // It takes no pace.
    assert!(b.get("speed").is_none());
    // Not a WAV file: said.
    let (url, _) = serve(200, "nothing like a sound").await;
    let e = read(
        &spec(Provider::Groq, &url),
        &choice("m", "hannah", "en", 1.0),
        "Hi.",
    )
    .await
    .unwrap_err();
    assert!(
        matches!(e, OnlineError::Speech(Provider::Groq, 200, _)),
        "{e:?}"
    );
}

/// Mistral's events: `audio_data` in base64, of float PCM.
fn mistral_events(bytes: &[u8], pieces: usize) -> Vec<u8> {
    let size = bytes.len().div_ceil(pieces) / 4 * 4;
    let mut out = String::new();
    for c in bytes.chunks(size.max(4)) {
        let b64 = base64::engine::general_purpose::STANDARD.encode(c);
        out.push_str(&format!(
            "event: speech.audio.delta\ndata: {{\"type\":\"speech.audio.delta\",\"audio_data\":\"{b64}\"}}\n\n"
        ));
    }
    out.push_str(
        "event: speech.audio.done\ndata: {\"type\":\"speech.audio.done\",\"usage\":{}}\n\n",
    );
    out.into_bytes()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mistral_reads_with_voxtral_its_float_pcm_in_events() {
    let events = mistral_events(&pcm_f32(12_000), 5);
    // Cut anywhere: an event may come in two pieces.
    let reply = streamed("text/event-stream", events, 9, 30);
    let (url, got) = serve_with(Arc::new(move |_: &Got| reply.clone())).await;
    let s = spec(Provider::Mistral, &url);
    let (pieces, _) = read(
        &s,
        &choice("voxtral-mini-tts-2603", "fr_marie_neutral", "fr", 1.0),
        "Bonjour.",
    )
    .await
    .unwrap();
    assert_eq!(samples(&pieces), 12_000);
    assert!(pieces.iter().all(|a| a.rate == 24_000));
    let peak = pieces
        .iter()
        .flat_map(|a| a.samples.iter())
        .fold(0f32, |m, s| m.max(s.abs()));
    assert!((peak - 0.25).abs() < 0.01, "floats read as floats: {peak}");
    let g = got.lock()[0].clone();
    assert_eq!(g.line, "POST /v1/audio/speech HTTP/1.1");
    assert_eq!(g.header("authorization"), Some(&*format!("Bearer {KEY}")));
    let b = json_of(&g);
    assert_eq!(b["model"], "voxtral-mini-tts-2603");
    assert_eq!(b["voice_id"], "fr_marie_neutral");
    assert_eq!(b["response_format"], "pcm");
    assert_eq!(b["stream"], true);
    // Not streamed: one JSON of it.
    let one = format!(
        r#"{{"audio_data":"{}"}}"#,
        base64::engine::general_purpose::STANDARD.encode(pcm_f32(2_400))
    );
    let (url, _) = serve(200, &one).await;
    let (pieces, _) = read(
        &spec(Provider::Mistral, &url),
        &choice("m", "v", "fr", 1.0),
        "Oui.",
    )
    .await
    .unwrap();
    assert_eq!(samples(&pieces), 2_400);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn elevenlabs_streams_its_pcm_told_the_language_and_the_pace_it_takes() {
    let reply = streamed("audio/pcm", pcm16(9_600), 3, 20);
    let (url, got) = serve_with(Arc::new(move |_: &Got| reply.clone())).await;
    let s = spec(Provider::ElevenLabs, &url);
    let (pieces, _) = read(
        &s,
        &choice("eleven_flash_v2_5", "JBFqnCBsd6RMkjVDRZzb", "fr", 2.0),
        "Fait.",
    )
    .await
    .unwrap();
    assert_eq!(samples(&pieces), 9_600);
    let g = got.lock()[0].clone();
    assert_eq!(
        g.line,
        "POST /v1/text-to-speech/JBFqnCBsd6RMkjVDRZzb/stream?output_format=pcm_24000 HTTP/1.1"
    );
    assert_eq!(g.header("xi-api-key"), Some(KEY));
    assert_eq!(g.header("authorization"), None);
    let b = json_of(&g);
    assert_eq!(b["text"], "Fait.");
    assert_eq!(b["model_id"], "eleven_flash_v2_5");
    assert_eq!(b["language_code"], "fr");
    // Twice as fast is more than it takes: its fastest.
    assert_eq!(b["voice_settings"]["speed"].as_f64(), Some(1.2f32 as f64));
    // Multilingual v2 is not told the language.
    read(
        &s,
        &choice("eleven_multilingual_v2", "abc", "fr", 1.0),
        "Fait.",
    )
    .await
    .unwrap();
    let b = json_of(&got.lock()[1]);
    assert!(
        b.get("language_code").is_none() && b.get("voice_settings").is_none(),
        "{b}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn deepgram_reads_with_the_aura_voice_asked_as_its_model() {
    let reply = streamed("audio/l16;rate=24000", pcm16(4_800), 2, 10);
    let (url, got) = serve_with(Arc::new(move |_: &Got| reply.clone())).await;
    let s = spec(Provider::Deepgram, &url);
    let (pieces, _) = read(
        &s,
        &choice("aura-2", "aura-2-agathe-fr", "fr", 1.3),
        "Fait.",
    )
    .await
    .unwrap();
    assert_eq!(samples(&pieces), 4_800);
    let g = got.lock()[0].clone();
    // Its pace is not asked in French.
    assert_eq!(
        g.line,
        "POST /v1/speak?model=aura-2-agathe-fr&encoding=linear16&sample_rate=24000&container=none HTTP/1.1"
    );
    assert_eq!(g.header("authorization"), Some(&*format!("Token {KEY}")));
    assert_eq!(json_of(&g), serde_json::json!({ "text": "Fait." }));
    read(
        &s,
        &choice("aura-2", "aura-2-thalia-en", "en", 1.3),
        "Done.",
    )
    .await
    .unwrap();
    assert_eq!(
        got.lock()[1].line,
        "POST /v1/speak?model=aura-2-thalia-en&encoding=linear16&sample_rate=24000&container=none&speed=1.30 HTTP/1.1"
    );
}

/// A choice each service takes.
fn any_choice(p: Provider) -> SpeakChoice {
    let model = escouade_voice::catalog::default_speak_model(p);
    let voice = escouade_voice::catalog::default_voice(p, model, "en").unwrap();
    choice(model, voice, "en", 1.0)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_refusals_of_every_service_say_what_to_do_without_the_key() {
    for p in super::super::online::PROVIDERS {
        let c = any_choice(p);
        let (url, _) = serve(
            401,
            &format!(r#"{{"error":{{"message":"bad key {KEY}"}}}}"#),
        )
        .await;
        let e = read(&spec(p, &url), &c, "Hi.").await.unwrap_err();
        assert_eq!(e, OnlineError::KeyRefused(p));
        let (url, _) = serve(429, r#"{"detail":{"status":"quota_exceeded","message":"This request exceeds your quota."}}"#).await;
        let e = read(&spec(p, &url), &c, "Hi.").await.unwrap_err();
        assert_eq!(
            e,
            OnlineError::Quota(p, "This request exceeds your quota.".into())
        );
        let m = e.message(crate::i18n::kit(crate::i18n::Lang::Fr));
        assert!(m.contains(p.label()) && m.contains("quota"), "{m}");
        let (url, _) = serve(503, &format!("upstream {KEY} down")).await;
        let e = read(&spec(p, &url), &c, "Hi.").await.unwrap_err();
        assert_eq!(e, OnlineError::Speech(p, 503, "upstream *** down".into()));
        assert!(!refuses_the_voice(&e));
        assert!(e
            .message(crate::i18n::kit(crate::i18n::Lang::En))
            .contains("did not read the passage"));
        let (url, _) = serve(404, r#"{"detail":{"status":"voice_not_found","message":"A voice with the voice_id x was not found."}}"#).await;
        let e = read(&spec(p, &url), &c, "Hi.").await.unwrap_err();
        assert!(refuses_the_voice(&e), "{e:?}");
        let e = read(&spec(p, &nowhere()), &c, "Hi.").await.unwrap_err();
        assert!(matches!(&e, OnlineError::Network(q, _) if *q == p), "{e:?}");
        assert!(e
            .message(crate::i18n::kit(crate::i18n::Lang::En))
            .contains("proxy"));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_barge_in_gives_up_the_stream_at_once() {
    // A long stream: 20 pieces, 200 ms apart.
    let reply = streamed("audio/pcm", pcm16(48_000), 20, 200);
    let (url, _) = serve_with(Arc::new(move |_: &Got| reply.clone())).await;
    let cancel = Arc::new(AtomicBool::new(false));
    let c = cancel.clone();
    // On what is heard, not on a delay: its service starts its module at this first call.
    let mut heard = None;
    let mut n = 0;
    let first = speak(
        &spec(Provider::OpenAi, &url),
        &choice("gpt-4o-mini-tts", "marin", "fr", 1.0),
        "Une longue phrase.",
        &cancel,
        &mut |a| {
            n += a.samples.len();
            heard.get_or_insert_with(Instant::now);
            c.store(true, Ordering::Release);
        },
    )
    .await
    .unwrap();
    assert!(first.is_some());
    assert!(n < 48_000);
    // Left to run, the stream lasts 4 s.
    let left = heard.unwrap().elapsed();
    assert!(left < Duration::from_secs(1), "{left:?}");
}

#[test]
fn a_passage_is_cut_in_sentences_none_longer_than_the_service_takes() {
    let text = "Les tests passent. Ensuite, il faut relire la documentation, vérifier le site, et publier la version.";
    assert_eq!(
        sentences(text, 4000),
        vec![
            "Les tests passent.".to_string(),
            "Ensuite, il faut relire la documentation, vérifier le site, et publier la version."
                .into()
        ]
    );
    let cut = sentences(text, 40);
    assert!(cut.iter().all(|s| s.chars().count() <= 40), "{cut:?}");
    assert_eq!(cut.join(" "), text);
    let long = "mot ".repeat(120);
    let cut = sentences(&long, 200);
    assert!(
        cut.len() >= 3 && cut.iter().all(|s| s.chars().count() <= 200),
        "{cut:?}"
    );
}

/// Settings that read the conversation with `service`, its key `key` in a keychain of the tests.
fn online_settings(
    service: &str,
) -> (
    crate::model::Settings,
    crate::integrations::secrets::MemoryStore,
) {
    let mut settings = crate::model::Settings::default();
    settings.voice.speech.engine = "online".into();
    settings.voice.speech.online_service = service.into();
    let store = crate::integrations::secrets::MemoryStore::default();
    (settings, store)
}

fn catalog_in(name: &str) -> super::super::online_catalog::CatalogStore {
    let dir = std::env::temp_dir().join(format!("escouade-esc132-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    super::super::online_catalog::CatalogStore::in_dir(&dir)
}

#[test]
fn the_settings_choose_the_service_its_model_and_voice_with_the_key_of_its_dictation() {
    let (mut settings, store) = online_settings("elevenlabs");
    let catalog = catalog_in("resolve");
    // Local, the default: nothing on line.
    let local = resolve(&crate::model::Settings::default(), &store, &catalog, "fr");
    assert!(local.online.is_none() && local.error.is_none() && local.notice.is_none());
    // No key yet: said, Piper reads.
    let r = resolve(&settings, &store, &catalog, "fr");
    assert_eq!(r.error, Some(OnlineError::NoKey(Provider::ElevenLabs)));
    // The key of the dictation: one entry by service.
    super::super::online::save_key(&store, "elevenlabs", KEY).unwrap();
    assert_eq!(store.entry("dictation-elevenlabs").as_deref(), Some(KEY));
    let r = resolve(&settings, &store, &catalog, "fr");
    let o = r.online.unwrap();
    assert_eq!(o.spec.key, KEY);
    assert_eq!(
        o.choice,
        choice("eleven_flash_v2_5", "JBFqnCBsd6RMkjVDRZzb", "fr", 1.0)
    );
    assert_eq!(o.default_voice, "JBFqnCBsd6RMkjVDRZzb");
    // The model and the voice chosen, by language; the pace of the settings.
    settings
        .voice
        .speech
        .online_models
        .insert("elevenlabs".into(), "eleven_v4".into());
    settings
        .voice
        .speech
        .online_voices
        .insert("elevenlabs|fr".into(), "my-own-voice".into());
    settings.voice.speech.rate = 1.1;
    let o = resolve(&settings, &store, &catalog, "fr").online.unwrap();
    assert_eq!(o.choice, choice("eleven_v4", "my-own-voice", "fr", 1.1));
    assert_eq!(o.spec.model, "eleven_v4");
    // In English, its default voice still.
    let o = resolve(&settings, &store, &catalog, "en").online.unwrap();
    assert_eq!(o.choice.voice, "JBFqnCBsd6RMkjVDRZzb");
    // A model it no longer offers: its default, said.
    settings
        .voice
        .speech
        .online_models
        .insert("elevenlabs".into(), "eleven_gone".into());
    let r = resolve(&settings, &store, &catalog, "fr");
    assert_eq!(r.online.unwrap().choice.model, "eleven_flash_v2_5");
    assert!(r.notice.unwrap().contains("eleven_gone"));
}

#[test]
fn a_service_that_does_not_speak_the_language_leaves_it_to_piper_and_says_so() {
    let (settings, store) = online_settings("groq");
    super::super::online::save_key(&store, "groq", KEY).unwrap();
    let catalog = catalog_in("groq-fr");
    let r = resolve(&settings, &store, &catalog, "fr");
    assert!(r.online.is_none() && r.error.is_none());
    let notice = r.notice.unwrap();
    assert!(
        notice.contains("Groq") && notice.contains("Piper"),
        "{notice}"
    );
    let o = resolve(&settings, &store, &catalog, "en").online.unwrap();
    assert_eq!(o.choice.voice, "hannah");
}

/// The real ElevenLabs, OpenAI… with the user's key given to this process only (`ESC132_SERVICE`, and the key in
/// `ESC132_KEY`): run by hand, never in CI, never with the key written anywhere. Prints how long the first sound took.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore]
async fn real_service() {
    let p = crate::voice::online::find(&std::env::var("ESC132_SERVICE").expect("ESC132_SERVICE"))
        .unwrap();
    let key = std::env::var("ESC132_KEY").expect("ESC132_KEY");
    let s = Spec::new(p, key);
    let model = escouade_voice::catalog::default_speak_model(p);
    let lang = if p == Provider::Groq { "en" } else { "fr" };
    let voice = escouade_voice::catalog::default_voice(p, model, lang).unwrap();
    let c = choice(model, voice, lang, 1.0);
    for text in [
        "Bonjour, les tests passent.",
        "La documentation est à jour.",
    ] {
        let started = Instant::now();
        let (pieces, first) = read(&s, &c, text).await.unwrap();
        println!(
            "{}: first sound in {} ms, {} pieces, {:.1} s of sound, all in {} ms",
            p.label(),
            first.unwrap().as_millis(),
            pieces.len(),
            samples(&pieces) as f32 / pieces[0].rate as f32,
            started.elapsed().as_millis()
        );
    }
    let listed = crate::voice::service::of(s.provider)
        .list(&s)
        .await
        .unwrap();
    println!(
        "listed: {:?} speak models, {:?} voices",
        listed
            .speak
            .map(|m| m.into_iter().map(|m| m.id).collect::<Vec<_>>()),
        listed.voices.map(|v| v.len())
    );
}
