//! The five voice services as plugins (INT-13, ESC-285), played through their processes against the same fake servers
//! as the core's own tests (`online_tests`, `live_tests`, `online_speech_tests`): what the service says through
//! `ProcessVoiceService` is what `CoreService` says. Needs `escouade-plugins` built beside the tests
//! (`cargo build --bin escouade-plugins`).

use super::live::tests::{fake, speech, Mode};
use super::live::{LiveStream, FINAL_WAIT};
use super::online::tests::{serve, KEY};
use super::online::{OnlineError, Provider, Spec, PROVIDERS};
use super::online_speech::SpeakChoice;
use super::plugin_host::ProcessVoiceService;
use super::service::{CoreService, VoiceService};
use crate::engine::bench::bench_dir;
use crate::integrations::secrets::MemoryStore;
use crate::paths::DataDir;
use crate::plugins::builtin;
use crate::plugins::runtime::{Limits, Runtime};
use crate::plugins::Plugins;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};

const VERSION: &str = "2.2.0";

fn app_dir() -> std::path::PathBuf {
    let exe = std::env::current_exe().unwrap();
    let dir = exe.parent().unwrap().parent().unwrap().to_path_buf();
    assert!(
        dir.join(builtin::program_name()).is_file(),
        "{} is not built beside the tests (cargo build --bin escouade-plugins)",
        dir.join(builtin::program_name()).display()
    );
    dir
}

fn runtime(name: &str) -> Arc<Runtime> {
    let data = DataDir::new(bench_dir(&format!("int13-{name}")).join("data"));
    let plugins = Plugins::load_in(data.clone(), VERSION, app_dir());
    Runtime::new(
        plugins,
        Arc::new(MemoryStore::default()),
        &data,
        VERSION,
        Limits {
            initialize: Duration::from_secs(20),
            ..Limits::default()
        },
    )
}

/// The service `p` as its plugin serves it.
async fn through(rt: &Arc<Runtime>, p: Provider) -> Arc<ProcessVoiceService> {
    ProcessVoiceService::connect(rt.clone(), &format!("dev.escouade.{}", p.id()))
        .await
        .unwrap_or_else(|e| panic!("{}: {e}", p.id()))
}

fn spec(p: Provider, base: &str) -> Spec {
    Spec {
        base: base.into(),
        ..Spec::new(p, KEY.into(), reqwest::Client::new())
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn each_plugin_describes_the_service_the_core_had() {
    let rt = runtime("describe");
    for p in PROVIDERS {
        let svc = through(&rt, p).await;
        let core = CoreService(p);
        assert_eq!(svc.id(), core.id());
        assert_eq!(svc.label(), core.label());
        assert_eq!(svc.account_entry(), core.account_entry());
        assert_eq!(svc.default_model(), core.default_model());
        assert_eq!(svc.offers_live(), core.offers_live());
        assert_eq!(svc.live_model("x"), core.live_model("x"));
        assert_eq!(svc.max_chars(), core.max_chars());
        let (a, b) = (svc.clone(), p);
        let catalog = tokio::task::spawn_blocking(move || a.catalog(None, None))
            .await
            .unwrap();
        assert_eq!(catalog, core.catalog(None, None), "{}", b.id());
    }
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
            let stream = svc.open_live(&f.spec(p), "fr", 48_000, Box::new(move |t| h.lock().push(t)));
            for frame in sound.chunks(960) {
                stream.push(frame);
            }
            let via = stream
                .finish(FINAL_WAIT)
                .unwrap_or_else(|e| panic!("{}: {e}", p.id()));

            let f2 = fake(p, Mode::Speaks);
            let stream = LiveStream::open(&f2.spec(p), "fr", 48_000, Box::new(|_| {}));
            for frame in sound.chunks(960) {
                stream.push(frame);
            }
            let direct = stream.finish(FINAL_WAIT).unwrap();
            assert_eq!(via.text, direct.text, "{}", p.id());
            // The same sound reached the service, in the same bytes.
            assert_eq!(
                f.conns().iter().map(|c| c.audio).sum::<usize>(),
                f2.conns().iter().map(|c| c.audio).sum::<usize>(),
                "{}",
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
    use super::online::tests::{Reply, Got};
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
    assert!(matches!(e, OnlineError::Network(Provider::Groq, _)), "{e:?}");
    rt.shutdown();
}

/// INT-13, voice: the delay of the final text after the end of a sentence, in the core and through the plugin's
/// process, the numbers of `docs/PLUGINS.md` §1.4. By hand (`cargo nextest run --run-ignored only int13_measure
/// --no-capture`), as INT-12's: two medians of a few ms on a loaded runner would fail at random.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "INT-13 measure: run by hand"]
async fn int13_measure_the_live_delay_through_the_plugin() {
    let rt = runtime("measure");
    let sound = speech(0.5);
    for p in PROVIDERS.into_iter().filter(|p| p.offers_live()) {
        let svc = through(&rt, p).await;
        let n = 30;
        let (a, snd) = (svc.clone(), sound.clone());
        let (core, plug) = tokio::task::spawn_blocking(move || {
            let delay = |open: &dyn Fn(&Spec) -> LiveStream| {
                let f = fake(p, Mode::Speaks);
                let s = f.spec(p);
                let mut ms = Vec::new();
                for _ in 0..n {
                    let stream = open(&s);
                    for frame in snd.chunks(960) {
                        stream.push(frame);
                    }
                    let end = Instant::now();
                    stream.finish(FINAL_WAIT).expect("a final text");
                    ms.push(end.elapsed().as_secs_f64() * 1000.0);
                }
                ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
                (ms[n / 2], ms[(n * 99 / 100).min(n - 1)])
            };
            let core = delay(&|s| LiveStream::open(s, "fr", 48_000, Box::new(|_| {})));
            let plug = delay(&|s| a.open_live(s, "fr", 48_000, Box::new(|_| {})));
            (core, plug)
        })
        .await
        .unwrap();
        eprintln!(
            "INT-13 {}: core median {:.1} ms (p99 {:.1}), plugin median {:.1} ms (p99 {:.1})",
            p.id(),
            core.0,
            core.1,
            plug.0,
            plug.1
        );
        assert!(plug.1 < core.1 + 5.0 + 50.0, "{}: the p99 grew", p.id());
    }
    rt.shutdown();
}
