//! The models and voices of each service, read from a fake server that lists them as its documentation shows; the
//! documentation's when it lists none; and the model chosen kept, or its default when it is gone.

use super::super::online::tests::{serve, serve_with, Got, Reply, KEY};
use super::*;
use crate::voice::online::{OnlineError, Spec};
use escouade_voice::catalog::{default_speak_model, ModelOption};
use std::sync::Arc;

/// What the plugin of the service lists.
async fn fetch(spec: &Spec) -> Result<Listed, OnlineError> {
    crate::voice::service::of(spec.provider).list(spec).await
}

fn spec(p: Provider, base: &str) -> Spec {
    Spec {
        base: base.into(),
        ..Spec::new(p, KEY.into())
    }
}

fn opt(id: &str, label: &str, note: String) -> ModelOption {
    ModelOption {
        id: id.into(),
        label: label.into(),
        note,
        langs: Vec::new(),
    }
}

fn ids(m: &Option<Vec<ModelOption>>) -> Vec<&str> {
    m.as_ref().unwrap().iter().map(|m| m.id.as_str()).collect()
}

/// A fake service answering `routes` by the start of the request's line.
async fn routes(list: Vec<(&'static str, String)>) -> (String, Arc<parking_lot::Mutex<Vec<Got>>>) {
    serve_with(Arc::new(move |g: &Got| {
        list.iter()
            .find(|(start, _)| g.line.starts_with(start))
            .map(|(_, body)| Reply::json(200, body))
            .unwrap_or_else(|| Reply::json(404, r#"{"error":{"message":"no route"}}"#))
    }))
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn openai_lists_what_transcribes_and_what_speaks_and_no_voice() {
    let (url, got) = serve(
        200,
        r#"{"object":"list","data":[{"id":"gpt-4o"},{"id":"tts-1"},{"id":"gpt-4o-mini-tts"},{"id":"whisper-1"},
        {"id":"gpt-4o-transcribe"},{"id":"gpt-4o-transcribe-diarize"},{"id":"gpt-4o-mini-transcribe"},{"id":"gpt-4o-mini-tts-2025-12-15"}]}"#,
    )
    .await;
    let l = fetch(&spec(Provider::OpenAi, &url)).await.unwrap();
    // The default first, in the documentation's order, with its name and note.
    assert_eq!(
        ids(&l.transcribe),
        ["gpt-4o-mini-transcribe", "gpt-4o-transcribe", "whisper-1"]
    );
    assert_eq!(
        ids(&l.speak),
        ["gpt-4o-mini-tts", "tts-1", "gpt-4o-mini-tts-2025-12-15"]
    );
    assert_eq!(l.speak.as_ref().unwrap()[0].label, "GPT-4o mini TTS");
    assert_eq!(l.voices, None);
    let g = got.lock()[0].clone();
    assert_eq!(g.line, "GET /v1/models HTTP/1.1");
    assert_eq!(g.header("authorization"), Some(&*format!("Bearer {KEY}")));
    // No voice list: a free field, with the default voice and where its voices are shown.
    let c = catalog(Provider::OpenAi, Some(&l), None);
    assert!(c.voices.is_none() && c.listed);
    assert_eq!(c.default_voices["gpt-4o-mini-tts|fr"], "marin");
    assert_eq!(c.default_voices["tts-1|en"], "alloy");
    assert!(
        c.voice_example.contains("marin")
            && c.voices_url.starts_with("https://platform.openai.com")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn groq_lists_whisper_and_orpheus_in_english_only() {
    let (url, got) = serve(
        200,
        r#"{"data":[{"id":"whisper-large-v3"},{"id":"whisper-large-v3-turbo"},{"id":"llama-3.3-70b-versatile"},
        {"id":"canopylabs/orpheus-v1-english"},{"id":"canopylabs/orpheus-arabic-saudi"},{"id":"playai-tts"}]}"#,
    )
    .await;
    let base = format!("{url}/openai");
    let l = fetch(&spec(Provider::Groq, &base)).await.unwrap();
    assert_eq!(got.lock()[0].line, "GET /openai/v1/models HTTP/1.1");
    assert_eq!(
        ids(&l.transcribe),
        ["whisper-large-v3-turbo", "whisper-large-v3"]
    );
    assert_eq!(ids(&l.speak), ["canopylabs/orpheus-v1-english"]);
    let c = catalog(Provider::Groq, Some(&l), None);
    assert_eq!(
        c.default_voices.get("canopylabs/orpheus-v1-english|fr"),
        None
    );
    assert_eq!(
        c.default_voices["canopylabs/orpheus-v1-english|en"],
        "hannah"
    );
    assert_eq!(c.speed, None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn elevenlabs_lists_its_models_and_the_voices_of_the_account_page_by_page() {
    let models = r#"[
        {"model_id":"eleven_multilingual_v2","name":"Eleven Multilingual v2","can_do_text_to_speech":true,"languages":[{"language_id":"en","name":"English"},{"language_id":"fr","name":"French"}]},
        {"model_id":"eleven_v4_turbo","name":"Eleven v4 Turbo","can_do_text_to_speech":true,"languages":[{"language_id":"en","name":"English"},{"language_id":"fr","name":"French"}]},
        {"model_id":"eleven_flash_v2","name":"Eleven Flash v2","can_do_text_to_speech":true,"languages":[{"language_id":"en","name":"English"}]},
        {"model_id":"eleven_multilingual_sts_v2","name":"STS","can_do_text_to_speech":false},
        {"model_id":"eleven_flash_v2_5","name":"Eleven Flash v2.5","can_do_text_to_speech":true,"languages":[{"language_id":"fr","name":"French"}]}
    ]"#;
    let page1 = r#"{"voices":[
        {"voice_id":"JBFqnCBsd6RMkjVDRZzb","name":"George","category":"premade","labels":{"gender":"male","accent":"british"},"verified_languages":[{"language":"en","model_id":"eleven_multilingual_v2"},{"language":"fr","model_id":"eleven_multilingual_v2"}]},
        {"voice_id":"myclone123","name":"Ma voix","category":"cloned","labels":{},"verified_languages":[]}
    ],"has_more":true,"next_page_token":"p2"}"#;
    let page2 = r#"{"voices":[{"voice_id":"XB0fDUnXU5powFXDhCwa","name":"Charlotte","category":"premade","labels":{"gender":"female","descriptive":"seductive"}}],"has_more":false,"next_page_token":null}"#;
    let (url, got) = routes(vec![
        ("GET /v1/models", models.into()),
        (
            "GET /v2/voices?page_size=100&next_page_token=p2",
            page2.into(),
        ),
        ("GET /v2/voices?page_size=100", page1.into()),
    ])
    .await;
    let l = fetch(&spec(Provider::ElevenLabs, &url)).await.unwrap();
    assert_eq!(l.transcribe, None, "its API lists no transcription model");
    assert_eq!(
        ids(&l.speak),
        [
            "eleven_flash_v2_5",
            "eleven_v4_turbo",
            "eleven_multilingual_v2",
            "eleven_flash_v2"
        ]
    );
    let flash2 = &l.speak.as_ref().unwrap()[3];
    assert_eq!(
        (flash2.label.as_str(), flash2.langs.clone()),
        ("Eleven Flash v2", vec!["en".to_string()])
    );
    let voices = l.voices.clone().unwrap();
    assert_eq!(voices.len(), 3);
    assert_eq!(voices[0].name, "George");
    assert_eq!(voices[0].langs, ["en", "fr"]);
    assert_eq!(voices[0].note, "male, british");
    assert!(
        !voices[0].own && voices[1].own,
        "the cloned voice is the user's own"
    );
    assert_eq!(voices[2].note, "female, seductive");
    assert!(got
        .lock()
        .iter()
        .all(|g| g.header("xi-api-key") == Some(KEY)));
    let c = catalog(Provider::ElevenLabs, Some(&l), None);
    // The documentation's transcription models; an English-only model reads no French.
    assert_eq!(c.transcribe[0].id, "scribe_v2");
    assert_eq!(c.default_voices.get("eleven_flash_v2|fr"), None);
    assert_eq!(
        c.default_voices["eleven_flash_v2_5|fr"],
        "JBFqnCBsd6RMkjVDRZzb"
    );
    assert_eq!(c.speed, Some([0.7, 1.2]));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_key_that_may_not_read_the_voices_leaves_the_field_free() {
    let (url, _) = serve_with(Arc::new(|g: &Got| {
        if g.line.starts_with("GET /v1/models") {
            Reply::json(200, r#"[{"model_id":"eleven_flash_v2_5","name":"Flash","can_do_text_to_speech":true}]"#)
        } else {
            Reply::json(401, r#"{"detail":{"status":"missing_permissions","message":"The API key you used is missing the permission voices_read"}}"#)
        }
    }))
    .await;
    let l = fetch(&spec(Provider::ElevenLabs, &url)).await.unwrap();
    assert_eq!(ids(&l.speak), ["eleven_flash_v2_5"]);
    assert_eq!(l.voices, None);
    assert!(catalog(Provider::ElevenLabs, Some(&l), None)
        .voices
        .is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mistral_lists_voxtral_and_its_voices_presets_and_the_user_s() {
    let models = r#"{"object":"list","data":[{"id":"mistral-large-latest"},{"id":"voxtral-mini-latest"},{"id":"voxtral-mini-2507"},
        {"id":"voxtral-small-latest"},{"id":"voxtral-mini-transcribe-realtime-2602"},{"id":"voxtral-mini-tts-2603"}]}"#;
    let voices = r#"{"items":[
        {"id":"0b5c","slug":"fr_marie_neutral","name":"Marie - Neutral","languages":["fr"],"gender":"female","type":"preset","user_id":null,"created_at":"2026-03-01"},
        {"id":"9a1e","slug":null,"name":"Ma voix","languages":[],"type":"custom","user_id":"u1","created_at":"2026-10-01"},
        {"id":"7c2d","slug":"en_paul_happy","name":"Paul - Happy","languages":[],"type":"preset","user_id":null,"created_at":"2026-03-01"}
    ],"total":3,"page":1,"page_size":100,"total_pages":1}"#;
    let (url, got) = routes(vec![
        ("GET /v1/models", models.into()),
        ("GET /v1/audio/voices", voices.into()),
    ])
    .await;
    let l = fetch(&spec(Provider::Mistral, &url)).await.unwrap();
    assert_eq!(
        ids(&l.transcribe),
        ["voxtral-mini-latest", "voxtral-mini-2507"]
    );
    assert_eq!(ids(&l.speak), ["voxtral-mini-tts-2603"]);
    let v = l.voices.unwrap();
    assert_eq!(
        v.iter().map(|v| (v.id.as_str(), v.own)).collect::<Vec<_>>(),
        [
            ("fr_marie_neutral", false),
            ("9a1e", true),
            ("en_paul_happy", false)
        ]
    );
    assert_eq!(v[0].langs, ["fr"]);
    // A preset's language read from its slug.
    assert_eq!(v[2].langs, ["en"]);
    assert_eq!(
        got.lock()[1].line,
        "GET /v1/audio/voices?limit=100 HTTP/1.1"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn deepgram_lists_its_architectures_and_each_voice_with_its_model_and_languages() {
    let body = r#"{
        "stt":[{"name":"general","canonical_name":"nova-3-general","architecture":"nova-3","languages":["en","fr"],"batch":true,"streaming":true},
               {"name":"general","canonical_name":"nova-2-general","architecture":"nova-2","languages":["en"],"batch":true},
               {"name":"flux","canonical_name":"flux-general-en","architecture":"flux","languages":["en"],"batch":false,"streaming":true}],
        "tts":[{"name":"agathe","canonical_name":"aura-2-agathe-fr","architecture":"aura-2","languages":["fr","fr-FR"],"metadata":{"accent":"French","tags":["feminine","warm"]}},
               {"name":"thalia","canonical_name":"aura-2-thalia-en","architecture":"aura-2","languages":["en","en-US"],"metadata":{"accent":"American","tags":["feminine"]}},
               {"name":"asteria","canonical_name":"aura-asteria-en","architecture":"aura","languages":["en"],"metadata":{}}]}"#;
    let (url, got) = serve(200, body).await;
    let l = fetch(&spec(Provider::Deepgram, &url)).await.unwrap();
    assert_eq!(
        got.lock()[0].header("authorization"),
        Some(&*format!("Token {KEY}"))
    );
    assert_eq!(
        ids(&l.transcribe),
        ["nova-3", "nova-2"],
        "the stream-only one left out"
    );
    assert_eq!(ids(&l.speak), ["aura-2", "aura"]);
    let v = l.voices.clone().unwrap();
    assert_eq!(
        (
            v[0].id.as_str(),
            v[0].name.as_str(),
            v[0].model.as_str(),
            v[0].langs.clone()
        ),
        (
            "aura-2-agathe-fr",
            "Agathe",
            "aura-2",
            vec!["fr".to_string()]
        )
    );
    assert_eq!(v[0].note, "French, feminine, warm");
    let c = catalog(Provider::Deepgram, Some(&l), None);
    assert!(c.voices_by_model);
    assert_eq!(c.default_voices.get("aura|fr"), None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_refused_key_lists_nothing_and_says_so() {
    let (url, _) = serve(401, r#"{"error":{"message":"Invalid API Key"}}"#).await;
    assert_eq!(
        fetch(&spec(Provider::Groq, &url)).await.unwrap_err(),
        OnlineError::KeyRefused(Provider::Groq)
    );
    // Then the documentation's, with the error.
    let c = catalog(Provider::Groq, None, Some("refused".into()));
    assert!(!c.listed && c.error.is_some());
    assert_eq!(c.transcribe[0].id, "whisper-large-v3-turbo");
}

fn store(name: &str) -> CatalogStore {
    let dir =
        std::env::temp_dir().join(format!("escouade-esc132-cat-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    CatalogStore::in_dir(&dir)
}

#[test]
fn the_model_chosen_is_kept_while_offered_else_its_default_said() {
    let s = store("checked");
    // Nothing chosen: today's model, the dictation unchanged.
    for p in super::super::online::PROVIDERS {
        assert_eq!(
            s.checked_model(p, Purpose::Transcribe, ""),
            (p.model().to_string(), None)
        );
        assert_eq!(
            s.checked_model(p, Purpose::Speak, "").0,
            default_speak_model(p)
        );
    }
    // Shipped: offered.
    assert_eq!(
        s.checked_model(Provider::OpenAi, Purpose::Transcribe, "gpt-4o-transcribe"),
        ("gpt-4o-transcribe".into(), None)
    );
    // Listed and kept: what the service offers now.
    let listed = Listed {
        at: 1,
        transcribe: Some(vec![opt("gpt-5-transcribe", "x", String::new())]),
        speak: None,
        voices: None,
    };
    s.put(Provider::OpenAi, &listed).unwrap();
    assert_eq!(s.get(Provider::OpenAi), Some(listed));
    assert_eq!(
        s.checked_model(Provider::OpenAi, Purpose::Transcribe, "gpt-5-transcribe")
            .0,
        "gpt-5-transcribe"
    );
    let (m, said) = s.checked_model(Provider::OpenAi, Purpose::Transcribe, "gpt-4o-transcribe");
    assert_eq!(m, "gpt-4o-mini-transcribe");
    assert!(said.unwrap().contains("gpt-4o-transcribe"));
    // The other services' are kept apart.
    s.put(Provider::Groq, &Listed::default()).unwrap();
    assert!(s.get(Provider::OpenAi).is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_dictation_sends_the_model_chosen_for_its_service() {
    let (url, got) = serve(200, r#"{"text":"ok"}"#).await;
    let s = spec(Provider::Groq, &url).with_model("whisper-large-v3");
    tokio::task::spawn_blocking(move || {
        let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
        crate::voice::service::of(s.provider)
            .transcribe(&s, &[0.1; 1600], "fr", &cancel)
            .unwrap()
    })
    .await
    .unwrap();
    assert_eq!(
        got.lock()[0].field("model").as_deref(),
        Some("whisper-large-v3")
    );
    // In the settings, by service; none: the default.
    let mut settings = crate::model::Settings::default();
    settings.voice.transcription = "online".into();
    settings.voice.online_service = "groq".into();
    let keys = crate::integrations::secrets::MemoryStore::default();
    super::super::online::save_key(&keys, "groq", KEY).unwrap();
    let cat = store("dictation");
    let spec_of = |s: &crate::model::Settings| {
        super::super::online::spec_of(s, &keys, &cat)
            .unwrap()
            .unwrap()
            .model
    };
    assert_eq!(spec_of(&settings), "whisper-large-v3-turbo");
    settings
        .voice
        .online_models
        .insert("groq".into(), "whisper-large-v3".into());
    settings
        .voice
        .online_models
        .insert("openai".into(), "whisper-1".into());
    assert_eq!(spec_of(&settings), "whisper-large-v3");
}
