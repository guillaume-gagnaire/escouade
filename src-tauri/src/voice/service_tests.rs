use crate::voice::live::tests::{fake, speech, Mode};
use crate::voice::live::{LiveStream, FINAL_WAIT};
use crate::voice::online::Provider;
use std::time::{Duration, Instant};

/// Delay of the final text after the end of a sentence (ESC-136, ESC-137), median and worst of `n` sentences, in ms.
fn final_delay(
    p: Provider,
    n: usize,
    open: &dyn Fn(&crate::voice::online::Spec) -> LiveStream,
) -> (f64, f64) {
    let f = fake(p, Mode::Speaks);
    let spec = f.spec(p);
    let sound = speech(0.5);
    let mut ms = Vec::new();
    for _ in 0..n {
        let stream = open(&spec);
        for frame in sound.chunks(960) {
            stream.push(frame);
        }
        let end = Instant::now();
        stream.finish(FINAL_WAIT).expect("a final text");
        ms.push(end.elapsed().as_secs_f64() * 1000.0);
    }
    ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
    (ms[ms.len() / 2], ms[ms.len() - 1])
}

/// The live transcription through the contract gives its final text, well within the bound of ESC-137.
#[test]
fn the_live_transcription_runs_through_the_contract() {
    for p in crate::voice::online::PROVIDERS
        .into_iter()
        .filter(|p| p.offers_live())
    {
        let svc = super::of(p);
        let direct = final_delay(p, 5, &|s| svc.open_live(s, "fr", 48_000, Box::new(|_| {})));
        eprintln!(
            "INT-12 direct {}: median {:.1} ms, max {:.1} ms",
            p.id(),
            direct.0,
            direct.1
        );
        assert!(direct.1 < Duration::from_secs(2).as_secs_f64() * 1000.0);
    }
}

#[test]
fn the_registry_holds_the_five_services_with_their_accounts() {
    let all = super::services();
    let ids: Vec<String> = all.iter().map(|s| s.id()).collect();
    assert_eq!(ids, ["openai", "groq", "mistral", "elevenlabs", "deepgram"]);
    for s in &all {
        let p = crate::voice::online::Provider::of(&s.id());
        assert_eq!(s.label(), p.label());
        assert_eq!(s.account_entry(), format!("dictation-{}", p.id()));
        assert_eq!(s.default_model(), p.model());
        assert_eq!(s.offers_live(), p.offers_live());
        assert_eq!(s.live_model("x"), p.live_model("x"));
        assert!(s.max_chars() > 0);
        assert!(!s.catalog(None, None).transcribe.is_empty());
        assert!(super::find(&s.id()).is_some());
    }
    assert!(super::find("whisper").is_none());
    assert!(!super::find("groq").unwrap().offers_live());
}
