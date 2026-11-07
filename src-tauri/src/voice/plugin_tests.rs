//! The five voice services as plugins (INT-13, ESC-285, ESC-350), played by their modules against the same fake servers as
//! the tests of the dictation, of a stream and of a reading. `sh scripts/plugins-wasm.sh` builds them.

use super::live::tests::{fake, speech, Mode};
use super::live::{LiveStream, FINAL_WAIT};
use super::online::tests::{serve, KEY};
use super::online::{OnlineError, Provider, Spec, PROVIDERS};
use super::online_speech::SpeakChoice;
use super::plugged::{runtime, through};
use super::service::VoiceService;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};

fn spec(p: Provider, base: &str) -> Spec {
    Spec {
        base: base.into(),
        ..Spec::new(p, KEY.into())
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn each_plugin_describes_the_service_the_core_had() {
    let rt = runtime("describe");
    for p in PROVIDERS {
        let svc = through(&rt, p).await;
        assert_eq!(svc.id(), p.id());
        assert_eq!(svc.label(), p.label());
        assert_eq!(svc.account_entry(), p.entry());
        assert_eq!(svc.default_model(), p.model());
        assert_eq!(svc.offers_live(), p.offers_live());
        assert_eq!(svc.live_model("x"), p.live_model("x"));
        assert_eq!(svc.max_chars(), escouade_voice::speech::max_chars(p));
        let (a, b) = (svc.clone(), p);
        let catalog = tokio::task::spawn_blocking(move || a.catalog(None, None))
            .await
            .unwrap();
        assert_eq!(
            catalog,
            super::online_catalog::catalog(p, None, None),
            "{}",
            b.id()
        );
    }
    rt.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_service_of_the_registry_starts_its_module_at_its_first_call_only() {
    use crate::plugins::runtime::PhaseName;
    let told = runtime("told");
    let rt = runtime("lazy");
    for p in PROVIDERS {
        let plugin = format!("dev.escouade.{}", p.id());
        let svc = super::plugin_host::ProcessVoiceService::described(rt.clone(), &plugin, p);
        let module = through(&told, p).await;
        // What the settings read of it starts nothing, and is what its module says.
        assert_eq!(svc.id(), module.id());
        assert_eq!(svc.label(), module.label());
        assert_eq!(svc.default_model(), module.default_model());
        assert_eq!(svc.offers_live(), module.offers_live());
        assert_eq!(svc.live_model("x"), module.live_model("x"));
        assert_eq!(svc.max_chars(), module.max_chars());
        assert!(
            matches!(rt.status(&plugin).phase, PhaseName::Idle),
            "{plugin}"
        );
        let catalog = tokio::task::spawn_blocking(move || svc.catalog(None, None))
            .await
            .unwrap();
        assert!(!catalog.speak.is_empty());
        assert!(
            matches!(rt.status(&plugin).phase, PhaseName::Running),
            "{plugin}"
        );
    }
    told.shutdown();
    rt.shutdown();
}

/// What each service answers a transcription, as its documentation shows it.
fn answer(p: Provider) -> &'static str {
    match p {
        Provider::Deepgram => {
            r#"{"results":{"channels":[{"alternatives":[{"transcript":"Bonjour le monde"}]}]}}"#
        }
        _ => r#"{"text":"Bonjour le monde"}"#,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_dictation_of_a_file_is_transcribed_through_the_plugin_and_its_refusals_come_back() {
    let rt = runtime("transcribe");
    for p in PROVIDERS {
        let svc = through(&rt, p).await;
        let (ok, got) = serve(200, answer(p)).await;
        let (s, samples) = (spec(p, &ok), vec![0.1f32; 16_000]);
        let a = svc.clone();
        let text = tokio::task::spawn_blocking(move || {
            a.transcribe(&s, &samples, "fr", &Arc::new(AtomicBool::new(false)))
        })
        .await
        .unwrap();
        assert_eq!(text.unwrap(), "Bonjour le monde", "{}", p.id());
        assert!(!got.lock().is_empty(), "{}: the fake got a request", p.id());

        let (refused, _) = serve(401, "{}").await;
        let s = spec(p, &refused);
        let a = svc.clone();
        let e = tokio::task::spawn_blocking(move || {
            a.transcribe(&s, &[0.0; 1600], "fr", &Arc::new(AtomicBool::new(false)))
        })
        .await
        .unwrap()
        .unwrap_err();
        assert_eq!(e, OnlineError::KeyRefused(p), "{}", p.id());
    }
    rt.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_live_transcription_goes_through_the_plugin_and_says_what_the_core_says() {
    let rt = runtime("live");
    for p in PROVIDERS.into_iter().filter(|p| p.offers_live()) {
        let svc = through(&rt, p).await;
        // The fake servers have a runtime of their own: made off the test's.
        tokio::task::spawn_blocking(move || {
            let f = fake(p, Mode::Speaks);
            let sound = speech(0.5);
            let heard = Arc::new(parking_lot::Mutex::new(Vec::<String>::new()));
            let h = heard.clone();
            let stream = svc.open_live(
                &f.spec(p),
                "fr",
                48_000,
                Box::new(move |t| h.lock().push(t)),
            );
            for frame in sound.chunks(960) {
                stream.push(frame);
            }
            let via = stream
                .finish(FINAL_WAIT)
                .unwrap_or_else(|e| panic!("{}: {e}", p.id()));

            assert_eq!(via.text, super::live::tests::FINAL, "{}", p.id());
            // All of the sound reached the service, at the rate it takes: half a second, in 16 bits. ElevenLabs' commit rides
            // on a hundredth of a second of zeros.
            let rate = if p == Provider::OpenAi {
                24_000
            } else {
                16_000
            };
            let bytes = f.conns().iter().map(|c| c.audio).sum::<usize>();
            let whole = rate;
            assert!(
                bytes >= whole - rate / 50 && bytes <= whole + rate / 25,
                "{}: {bytes} bytes",
                p.id()
            );
            assert!(!heard.lock().is_empty(), "{}: partial texts came", p.id());
        })
        .await
        .unwrap();
    }
    rt.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_sentence_is_read_through_the_plugin_its_sound_given_as_it_comes() {
    use super::online::tests::{Got, Reply};
    let rt = runtime("speak");
    let svc = through(&rt, Provider::OpenAi).await;
    let pcm: Vec<u8> = (0..4800usize)
        .flat_map(|i| (((i as f32 / 10.0).sin() * 8000.0) as i16).to_le_bytes())
        .collect();
    let size = pcm.len().div_ceil(4);
    let reply = Reply {
        status: 200,
        content_type: "audio/pcm".into(),
        chunks: pcm.chunks(size).map(<[u8]>::to_vec).collect(),
        gap: Duration::from_millis(5),
    };
    let (url, got) = super::online::tests::serve_with(Arc::new(move |_: &Got| reply.clone())).await;
    let s = spec(Provider::OpenAi, &url);
    let choice = SpeakChoice {
        model: "gpt-4o-mini-tts".into(),
        voice: "marin".into(),
        lang: "fr".into(),
        rate: 1.0,
    };
    let mut heard = Vec::new();
    let first = svc
        .speak(&s, &choice, "Bonjour.", &AtomicBool::new(false), &mut |a| {
            heard.push(a)
        })
        .await
        .unwrap();
    assert!(first.is_some());
    assert!(heard.len() >= 2, "sound came in pieces: {}", heard.len());
    assert_eq!(heard.iter().map(|a| a.samples.len()).sum::<usize>(), 4800);
    assert!(got.lock()[0].body_text().contains("Bonjour."));
    rt.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_plugin_that_is_gone_leaves_its_service_answering_with_an_error() {
    let rt = runtime("gone");
    let svc = through(&rt, Provider::Groq).await;
    rt.stop("dev.escouade.groq");
    let (url, _) = serve(200, answer(Provider::Groq)).await;
    let s = spec(Provider::Groq, &url);
    let e = tokio::task::spawn_blocking(move || {
        svc.transcribe(&s, &[0.0; 1600], "fr", &Arc::new(AtomicBool::new(false)))
    })
    .await
    .unwrap()
    .unwrap_err();
    assert!(
        matches!(e, OnlineError::Network(Provider::Groq, _)),
        "{e:?}"
    );
    rt.shutdown();
}

/// A dictation of 10 minutes: its sound crosses in one line, then one request, within the memory of a module and its turns.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_long_dictation_goes_through_the_module() {
    let rt = runtime("long");
    let p = Provider::Groq;
    let svc = through(&rt, p).await;
    let (url, got) = serve(200, answer(p)).await;
    let s = spec(p, &url);
    let text = tokio::task::spawn_blocking(move || {
        let sound = vec![0.1f32; 10 * 60 * 16_000];
        svc.transcribe(&s, &sound, "fr", &Arc::new(AtomicBool::new(false)))
    })
    .await
    .unwrap();
    assert_eq!(text.unwrap(), "Bonjour le monde");
    assert!(got.lock()[0].body.len() > 19 * 1000 * 1000);
    rt.shutdown();
}

/// INT-13, voice: the delay of the final text after the end of a sentence through the plugin's module, the numbers of `docs/PLUGINS.md` §1.4. By hand (`cargo nextest run --run-ignored only int13_measure
/// --no-capture`), as INT-12's: two medians of a few ms on a loaded runner would fail at random.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "INT-13 measure: run by hand"]
async fn int13_measure_the_live_delay_through_the_plugin() {
    let rt = runtime("measure");
    let sound = speech(0.3);
    for p in PROVIDERS.into_iter().filter(|p| p.offers_live()) {
        let svc = through(&rt, p).await;
        let n = 20;
        let (a, snd) = (svc.clone(), sound.clone());
        let plug = tokio::task::spawn_blocking(move || {
            let delay = |open: &dyn Fn(&Spec) -> LiveStream| {
                let f = fake(p, Mode::Speaks);
                let s = f.spec(p);
                let mut ms = Vec::new();
                for _ in 0..n {
                    let stream = open(&s);
                    // As the microphone gives it: a frame of 20 ms every 20 ms.
                    for frame in snd.chunks(960) {
                        stream.push(frame);
                        std::thread::sleep(Duration::from_millis(20));
                    }
                    let end = Instant::now();
                    stream.finish(FINAL_WAIT).expect("a final text");
                    ms.push(end.elapsed().as_secs_f64() * 1000.0);
                }
                ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
                (ms[n / 2], ms[(n * 99 / 100).min(n - 1)])
            };
            delay(&|s| a.open_live(s, "fr", 48_000, Box::new(|_| {})))
        })
        .await
        .unwrap();
        eprintln!(
            "INT-13 {}: module median {:.1} ms (p99 {:.1})",
            p.id(),
            plug.0,
            plug.1
        );
    }
    rt.shutdown();
}

/// The real ElevenLabs through its plugin (INT-13), as the core's three real trials (`real_elevenlabs`,
/// `real_elevenlabs_live`, `real_service`) play it: `ELEVENLABS_API_KEY` of the machine, passed to this process only,
/// never written. By hand: `cargo nextest run --run-ignored only int13_real_elevenlabs --no-capture`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "real service: run by hand with ELEVENLABS_API_KEY"]
async fn int13_real_elevenlabs_through_the_plugin() {
    let key = std::env::var("ELEVENLABS_API_KEY").expect("ELEVENLABS_API_KEY");
    let p = Provider::ElevenLabs;
    let rt = runtime("real");
    let svc = through(&rt, p).await;
    let wav =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/voice/testdata/question-fr.wav");
    let mut r = hound::WavReader::open(wav).unwrap();
    let rate = r.spec().sample_rate;
    let samples: Vec<f32> = r
        .samples::<i16>()
        .map(|s| s.unwrap() as f32 / i16::MAX as f32)
        .collect();
    let new_spec =
        move |key: &str| Spec::new(p, key.into()).with_live(Some("scribe_v2_realtime".into()));

    // The dictation of the file, and a wrong key refused.
    let (a, k, snd) = (
        svc.clone(),
        key.clone(),
        crate::voice::audio::resample(&samples, rate, 16_000),
    );
    let started = Instant::now();
    let (text, refused) = tokio::task::spawn_blocking(move || {
        let no = Arc::new(AtomicBool::new(false));
        (
            a.transcribe(&new_spec(&k), &snd, "fr", &no),
            a.transcribe(&new_spec("not-a-key-0000"), &snd, "fr", &no),
        )
    })
    .await
    .unwrap();
    println!(
        "dictation in {} ms: {:?}",
        started.elapsed().as_millis(),
        text
    );
    assert!(!text.unwrap().is_empty());
    assert_eq!(refused.unwrap_err(), OnlineError::KeyRefused(p));

    // As one speaks.
    let (a, k, snd) = (svc.clone(), key.clone(), samples.clone());
    let (seen, said, ms, text) = tokio::task::spawn_blocking(move || {
        let started = Instant::now();
        let seen = Arc::new(parking_lot::Mutex::new(Vec::<(u128, String)>::new()));
        let s2 = seen.clone();
        let stream = a.open_live(
            &new_spec(&k),
            "fr",
            rate,
            Box::new(move |t| s2.lock().push((started.elapsed().as_millis(), t))),
        );
        for frame in snd.chunks(rate as usize / 50) {
            stream.push(frame);
            std::thread::sleep(Duration::from_millis(20));
        }
        let said = started.elapsed().as_millis();
        let end = Instant::now();
        let text = stream.finish(FINAL_WAIT).unwrap().text;
        let seen = seen.lock().clone();
        (seen, said, end.elapsed().as_millis(), text)
    })
    .await
    .unwrap();
    for (at, t) in &seen {
        println!("partial at {at} ms: {t}");
    }
    println!("said in {said} ms; final in {ms} ms after the end: {text}");
    assert!(
        seen.iter().any(|(at, _)| *at < said),
        "no text while it was said"
    );
    assert!(!text.is_empty());

    // The voice, and the catalog.
    let s = new_spec(&key);
    let model = escouade_voice::catalog::default_speak_model(p);
    let voice = escouade_voice::catalog::default_voice(p, model, "fr").unwrap();
    let choice = SpeakChoice {
        model: model.into(),
        voice: voice.into(),
        lang: "fr".into(),
        rate: 1.0,
    };
    let started = Instant::now();
    let mut pieces = 0;
    let mut heard = 0usize;
    let first = svc
        .speak(
            &s,
            &choice,
            "Bonjour, les tests passent.",
            &AtomicBool::new(false),
            &mut |a| {
                pieces += 1;
                heard += a.samples.len()
            },
        )
        .await
        .unwrap();
    println!(
        "voice: first sound in {} ms, {pieces} pieces, {heard} samples, all in {} ms",
        first.unwrap().as_millis(),
        started.elapsed().as_millis()
    );
    assert!(pieces > 0 && heard > 0);
    let listed = svc.list(&s).await.unwrap();
    println!(
        "listed: {:?} speak models, {:?} voices",
        listed.speak.map(|m| m.len()),
        listed.voices.map(|v| v.len())
    );
    rt.shutdown();
}

/// The real Mistral through its plugin (ESC-327), the process that serves Vibe too: `MISTRAL_API_KEY` given to this
/// process only, never written. By hand: `cargo nextest run --run-ignored only real_mistral_through --no-capture`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "real service: run by hand with MISTRAL_API_KEY"]
async fn real_mistral_through_the_plugin() {
    let key = std::env::var("MISTRAL_API_KEY").expect("MISTRAL_API_KEY");
    let p = Provider::Mistral;
    let rt = runtime("real-mistral");
    let svc = through(&rt, p).await;
    let wav =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/voice/testdata/question-fr.wav");
    let mut r = hound::WavReader::open(wav).unwrap();
    let rate = r.spec().sample_rate;
    let samples: Vec<f32> = r
        .samples::<i16>()
        .map(|s| s.unwrap() as f32 / i16::MAX as f32)
        .collect();
    let sound = crate::voice::audio::resample(&samples, rate, 16_000);
    let spec = move |key: &str| Spec::new(p, key.into());
    let (text, refused) = tokio::task::spawn_blocking(move || {
        let no = Arc::new(AtomicBool::new(false));
        (
            svc.transcribe(&spec(&key), &sound, "fr", &no),
            svc.transcribe(&spec("not-a-key-0000"), &sound, "fr", &no),
        )
    })
    .await
    .unwrap();
    println!("the dictation: {text:?}");
    assert!(!text.unwrap().is_empty());
    assert_eq!(refused.unwrap_err(), OnlineError::KeyRefused(p));
}
