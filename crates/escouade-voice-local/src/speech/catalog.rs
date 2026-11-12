//! The voices the app offers: a list checked against the rule of their licences, shipped with the app and updated at
//! its releases (decided with the user, 2026-10-16), not PiperHub's catalog read live.
//!
//! The rule (decided with the user, 2026-10-15): the model's licence permissive (MIT, Apache-2.0, BSD, CC0, CC BY and
//! CC BY-SA with their attribution, public domain; the GPL family waits for a confirmation); its dataset's too when it is
//! known (no NC, no ND, no « research only »); and no voice of a character, of a known person or of a brand. The licence
//! read on Hugging Face (the model's card, the MODEL_CARD of an official voice, the card of a dataset) prevails over
//! PiperHub's field, which is sometimes wrong. A voice of the community whose recordings are unknown is only offered
//! once checked by hand (`CHECKED`): its licence alone does not tell whose voice it is.
//!
//! `scripts/piper-voices.mjs` takes a snapshot of PiperHub's catalog with what the rule needs; the ignored test
//! `write_the_voice_catalog` applies `admit` to it and writes `voices.json`, which the app reads.

// The snapshot's models are only read by the tests and by the generator of `voices.json`.
#![cfg_attr(not(test), allow(dead_code))]

use serde::{Deserialize, Serialize};

/// The repository of the official voices (Rhasspy, then the Open Home Foundation).
pub const OFFICIAL: &str = "rhasspy/piper-voices";

/// What a licence lets one do, as the rule sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Licence {
    /// Free to use; `attribution`: the author must be credited (CC BY, CC BY-SA).
    Permissive {
        attribution: bool,
    },
    /// GPL, AGPL, LGPL: compatible with Escouade's, but waiting for a confirmation.
    Copyleft,
    NonCommercial,
    NoDerivatives,
    /// For research only, or a company's name where a licence should be.
    Restricted,
    Unknown,
}

/// What a licence's text (an SPDX id, a name, an URL) says, as the rule sees it.
pub fn licence(text: Option<&str>) -> Licence {
    let Some(raw) = text.map(str::trim).filter(|t| !t.is_empty()) else {
        return Licence::Unknown;
    };
    let t = raw.to_lowercase().replace(['_', ' '], "-");
    let has = |s: &str| t.contains(s);
    if has("see-url") || has("see-license") || t == "other" || t == "unknown" {
        return Licence::Unknown;
    }
    if has("research") || has("blizzard") {
        return Licence::Restricted;
    }
    if has("-nd") || has("/nd") || has("noderiv") {
        return Licence::NoDerivatives;
    }
    if has("-nc") || has("/nc") || has("noncommercial") || has("non-commercial") {
        return Licence::NonCommercial;
    }
    if has("gpl") {
        return Licence::Copyleft;
    }
    if has("cc0") || has("publicdomain/zero") || has("public-domain") || t == "unlicense" {
        return Licence::Permissive { attribution: false };
    }
    if has("by-sa") || has("cc-by") || has("licenses/by/") || has("cc-by-4") {
        return Licence::Permissive { attribution: true };
    }
    if t == "mit" || t.starts_with("mit-") || has("apache") || t.starts_with("bsd") || has("isc") {
        return Licence::Permissive { attribution: false };
    }
    // A company's or a game's name (« Valve », « Riot Games », « Fortune's Café »).
    Licence::Restricted
}

/// A model of the snapshot (`scripts/piper-voices.mjs`).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    pub slug: String,
    pub name: String,
    pub lang: String,
    pub quality: String,
    pub repo: String,
    pub onnx: String,
    pub json: String,
    #[serde(default)]
    pub speakers: u32,
    #[serde(default)]
    pub speaker_names: Vec<String>,
    #[serde(default)]
    pub sample_rate: u32,
    pub phoneme_type: Option<String>,
    pub espeak_voice: Option<String>,
    pub usable: bool,
    pub hub: Hub,
    pub hf: Hf,
    pub card: Option<Card>,
    pub files: Files,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Hub {
    pub license: Option<String>,
    /// PiperHub's « universe » of a voice: « default », or the game, film or brand it comes from.
    pub universe: Option<String>,
    pub certified: bool,
    pub dataset: Option<HubDataset>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct HubDataset {
    pub repo: Option<String>,
    /// What its recordings come from (a game, a character).
    pub source: Option<String>,
    pub license: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Hf {
    pub license: Option<String>,
    pub dataset_license: Option<String>,
}

/// What the MODEL_CARD of an official voice says.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    pub dataset_url: Option<String>,
    pub dataset_license: Option<String>,
    pub base: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Files {
    pub onnx: Option<FileInfo>,
    pub json: Option<FileInfo>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct FileInfo {
    pub size: u64,
    pub sha256: Option<String>,
}

/// Names that are a character's, a known person's or a brand's, whatever their licence says.
const NOT_OURS: &[&str] = &[
    "glados",
    "wheatley",
    "cave_johnson",
    "cave-johnson",
    "announcer",
    "death",
    "eminem",
    "picard",
    "hal-9000",
    "hal9000",
    "bender",
    "jarvis",
    "huberman",
    "optimus",
    "ahsoka",
    "kronk",
    "bmo",
    "rocket",
    "tiktok",
    "candycrush",
    "babymetal",
    "overwatch",
    "subnautica",
    "fedcomp",
    "unclefred",
    "scaramouche",
    "kuroki",
    "patrick",
    "fortune",
    "fuze",
    "dota",
    "valorant",
    "counter-strike",
    "css-",
    "data_7024",
    "mattheo",
];

/// Voices of the community checked by hand: whose recordings they are, and that they may be used.
const CHECKED: &[(&str, &str)] = &[
    (
        "colin-english-en-high",
        "nardocolin/nardocolin-pipertts: the author's own voice, CC BY 4.0 on its card",
    ),
    (
        "librivox-au-english-au-medium-10-voice",
        "DataCraftsmanAustralia: ten LibriVox readers (public domain recordings), the model CC BY 4.0",
    ),
];

/// A voice offered by a decision of the user against the rule: one id and one file at a time, never a loosening of the
/// rule, which still refuses every other voice of the same kind. What its licences are was read by hand, and replaces
/// the catalog's fields.
#[derive(Debug)]
pub struct Exception {
    pub id: &'static str,
    /// The model the decision covers: another file under the same id is not admitted.
    pub onnx_sha256: &'static str,
    /// The licence its card declares (`None`: none).
    pub model_licence: Option<&'static str>,
    pub dataset_licence: &'static str,
    pub dataset_url: &'static str,
    pub credit: &'static str,
}

const EXCEPTIONS: &[Exception] = &[
    // 2026-10-17, decided by the user after trying it on their machine (ESC-114): a French voice of high quality, with
    // 14 styles. The rule refuses it: PiperHub puts it in the « fuze » universe, its model declares no licence on Hugging
    // Face (« SandBlock Studios » on PiperHub, not certified), and its dataset, CC BY 4.0 on its card, was generated with
    // ElevenLabs v4 (its voice « Fuze », a gaming YouTuber's style).
    Exception {
        id: "fuze-french-fr-multi-high",
        onnx_sha256: "8920448e34b46cbfce0f46c6fa57fc91065d96bb795958071aeb7b79a4278225",
        model_licence: None,
        dataset_licence: "CC BY 4.0",
        dataset_url: "https://huggingface.co/datasets/fox3000foxy/fuze-eleven-v4-piper",
        credit: "Fuze - French (FR) Multi High — dataset by fox3000foxy, https://huggingface.co/datasets/fox3000foxy/fuze-eleven-v4-piper (CC BY 4.0), generated with ElevenLabs v4",
    },
];

/// The exception decided for the voice `id` whose model is `sha256`, if any.
pub fn exception(id: &str, sha256: Option<&str>) -> Option<&'static Exception> {
    EXCEPTIONS
        .iter()
        .find(|e| e.id == id && Some(e.onnx_sha256) == sha256)
}

/// Why a voice is not offered, or that it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Admitted,
    /// Not a voice Piper reads as the app does (its phonemes are not eSpeak's), or one PiperHub marks unusable.
    Unusable,
    ModelLicence(Licence),
    DatasetLicence(Licence),
    /// A character's, a known person's or a brand's voice.
    Character(String),
    /// Of the community, its recordings unknown, not checked by hand.
    UnknownOrigin,
    /// A copy of an official voice: the official one is offered.
    Copy,
}

/// The model's licence as the rule reads it: Hugging Face's card first, then PiperHub's field.
pub fn model_licence(m: &Model) -> Option<&str> {
    m.hf.license.as_deref().or(m.hub.license.as_deref())
}

/// The dataset's: the official MODEL_CARD, the dataset's card on Hugging Face, then PiperHub's.
pub fn dataset_licence(m: &Model) -> Option<&str> {
    m.card
        .as_ref()
        .and_then(|c| c.dataset_license.as_deref())
        .or(m.hf.dataset_license.as_deref())
        .or(m.hub.dataset.as_ref().and_then(|d| d.license.as_deref()))
}

/// The rule, for `m` among `all` (to tell a copy of an official voice).
pub fn admit(m: &Model, all: &[Model]) -> Verdict {
    if !m.usable
        || m.phoneme_type.as_deref() != Some("espeak")
        || m.files.onnx.is_none()
        || m.files.json.is_none()
    {
        return Verdict::Unusable;
    }
    let sha = m.files.onnx.as_ref().and_then(|f| f.sha256.as_deref());
    if exception(&m.slug, sha).is_some() {
        return Verdict::Admitted;
    }
    let official = m.repo == OFFICIAL;
    let lower = format!("{} {} {} {}", m.slug, m.name, m.repo, m.onnx).to_lowercase();
    if let Some(name) = NOT_OURS.iter().find(|n| lower.contains(*n)) {
        return Verdict::Character((*name).to_string());
    }
    if let Some(u) = m.hub.universe.as_deref().filter(|u| *u != "default") {
        return Verdict::Character(u.to_string());
    }
    if let Some(s) = m.hub.dataset.as_ref().and_then(|d| d.source.as_deref()) {
        return Verdict::Character(s.to_string());
    }
    match licence(model_licence(m)) {
        Licence::Permissive { .. } => {}
        other => return Verdict::ModelLicence(other),
    }
    if !official
        && sha.is_some()
        && all.iter().any(|o| {
            o.repo == OFFICIAL && o.files.onnx.as_ref().and_then(|f| f.sha256.as_deref()) == sha
        })
    {
        return Verdict::Copy;
    }
    let checked = official || CHECKED.iter().any(|(s, _)| *s == m.slug);
    match licence(dataset_licence(m)) {
        Licence::Permissive { .. } | Licence::Unknown if checked => Verdict::Admitted,
        Licence::Permissive { .. } | Licence::Unknown => Verdict::UnknownOrigin,
        other => Verdict::DatasetLicence(other),
    }
}

/// A voice the app offers: what `voices.json` holds for it.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VoiceInfo {
    /// The name of its file without `.onnx` for an official voice (`fr_FR-siwis-medium`), PiperHub's slug otherwise.
    pub id: String,
    /// The language of the interface it speaks (`fr`, `en`), and its own locale (`en_GB`) when it is known.
    pub lang: String,
    pub locale: Option<String>,
    pub name: String,
    pub quality: String,
    #[serde(default)]
    pub speakers: u32,
    #[serde(default)]
    pub speaker_names: Vec<String>,
    #[serde(default)]
    pub sample_rate: u32,
    pub repo: String,
    pub onnx: String,
    pub json: String,
    pub onnx_size: u64,
    pub onnx_sha256: String,
    pub json_size: u64,
    pub json_sha256: String,
    pub license: String,
    pub dataset_license: Option<String>,
    pub dataset_url: Option<String>,
    /// What must be shown with it (CC BY): the work, its author or source, and the licence.
    pub attribution: Option<String>,
    /// Offered by a decision of the user against the rule (`EXCEPTIONS`): Réglages › Voix says so.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub exception: bool,
}

/// `m`, admitted, as the app ships it.
pub fn voice_info(m: &Model) -> Option<VoiceInfo> {
    let onnx = m.files.onnx.as_ref()?;
    let json = m.files.json.as_ref()?;
    let stem = m
        .onnx
        .rsplit('/')
        .next()?
        .trim_end_matches(".onnx")
        .to_string();
    let official = m.repo == OFFICIAL;
    let locale = official
        .then(|| stem.split('-').next().map(str::to_string))
        .flatten();
    let id = if official { stem } else { m.slug.clone() };
    if let Some(e) = exception(&id, onnx.sha256.as_deref()) {
        return Some(VoiceInfo {
            id,
            lang: m.lang.clone(),
            locale,
            name: m.name.clone(),
            quality: m.quality.clone(),
            speakers: m.speakers,
            speaker_names: m.speaker_names.clone(),
            sample_rate: m.sample_rate,
            repo: m.repo.clone(),
            onnx: m.onnx.clone(),
            json: m.json.clone(),
            onnx_size: onnx.size,
            onnx_sha256: e.onnx_sha256.to_string(),
            json_size: json.size,
            json_sha256: json.sha256.clone()?,
            license: e.model_licence.unwrap_or_default().to_string(),
            dataset_license: Some(e.dataset_licence.to_string()),
            dataset_url: Some(e.dataset_url.to_string()),
            attribution: Some(e.credit.to_string()),
            exception: true,
        });
    }
    let model_lic = model_licence(m).unwrap_or_default().to_string();
    let dataset_lic = dataset_licence(m).map(str::to_string);
    let dataset_url = m.card.as_ref().and_then(|c| c.dataset_url.clone());
    let mut credit = Vec::new();
    if let Licence::Permissive { attribution: true } = licence(Some(&model_lic)) {
        credit.push(format!(
            "{} — https://huggingface.co/{} ({model_lic})",
            m.name, m.repo
        ));
    }
    if let Some(d) = dataset_lic.as_deref() {
        if let Licence::Permissive { attribution: true } = licence(Some(d)) {
            let src = dataset_url
                .clone()
                .or_else(|| {
                    m.hub
                        .dataset
                        .as_ref()
                        .and_then(|x| x.repo.clone())
                        .map(|r| format!("https://huggingface.co/datasets/{r}"))
                })
                .unwrap_or_else(|| format!("https://huggingface.co/{}", m.repo));
            credit.push(format!("{src} ({d})"));
        }
    }
    Some(VoiceInfo {
        id,
        lang: m.lang.clone(),
        locale,
        name: m.name.clone(),
        quality: m.quality.clone(),
        speakers: m.speakers,
        speaker_names: m.speaker_names.clone(),
        sample_rate: m.sample_rate,
        repo: m.repo.clone(),
        onnx: m.onnx.clone(),
        json: m.json.clone(),
        onnx_size: onnx.size,
        onnx_sha256: onnx.sha256.clone()?,
        json_size: json.size,
        json_sha256: json.sha256.clone()?,
        license: model_lic,
        dataset_license: dataset_lic,
        dataset_url,
        attribution: (!credit.is_empty()).then(|| credit.join(" · ")),
        exception: false,
    })
}

/// What the rule still says of a voice of the list as shipped: its licences (the dataset's when known) permissive,
/// and its name none of a character, a person or a brand. A voice that no longer passes (the rule made stricter) is
/// not offered, even before the list is written again. A voice of `EXCEPTIONS` passes, its id and its model those of
/// the decision.
pub fn allowed(v: &VoiceInfo) -> bool {
    if exception(&v.id, Some(&v.onnx_sha256)).is_some() {
        return true;
    }
    let lower = format!("{} {} {}", v.id, v.name, v.repo).to_lowercase();
    matches!(licence(Some(&v.license)), Licence::Permissive { .. })
        && matches!(
            licence(v.dataset_license.as_deref()),
            Licence::Permissive { .. } | Licence::Unknown
        )
        && !NOT_OURS.iter().any(|n| lower.contains(n))
}

/// The voices the app ships.
pub fn shipped() -> &'static [VoiceInfo] {
    static LIST: std::sync::OnceLock<Vec<VoiceInfo>> = std::sync::OnceLock::new();
    LIST.get_or_init(|| {
        let all: Vec<VoiceInfo> =
            serde_json::from_str(include_str!("voices.json")).expect("voices.json is valid");
        all.into_iter().filter(allowed).collect()
    })
}

/// The voice of a first conversation in `lang`, when the settings name none (decided at implementation: an official
/// voice of medium quality, its dataset in the public domain or under CC BY, ~63 MB).
pub fn default_voice(lang: &str) -> &'static str {
    match lang {
        "fr" => "fr_FR-siwis-medium",
        _ => "en_US-ljspeech-medium",
    }
}

pub fn find(id: &str) -> Option<&'static VoiceInfo> {
    shipped().iter().find(|v| v.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn excerpt() -> Vec<Model> {
        #[derive(Deserialize)]
        struct Snapshot {
            models: Vec<Model>,
        }
        let s: Snapshot =
            serde_json::from_str(include_str!("testdata/catalog-excerpt.json")).unwrap();
        s.models
    }

    #[test]
    fn licences_are_read_from_their_ids_names_and_urls() {
        let p = |a| Licence::Permissive { attribution: a };
        assert_eq!(licence(Some("MIT")), p(false));
        assert_eq!(licence(Some("apache-2.0")), p(false));
        assert_eq!(licence(Some("CC0")), p(false));
        assert_eq!(licence(Some("public domain")), p(false));
        assert_eq!(licence(Some("CC-BY 4.0")), p(true));
        assert_eq!(
            licence(Some("https://creativecommons.org/licenses/by/4.0/")),
            p(true)
        );
        assert_eq!(
            licence(Some("http://creativecommons.org/licenses/by-sa/4.0/")),
            p(true)
        );
        assert_eq!(licence(Some("CC BY-NC-SA 4.0")), Licence::NonCommercial);
        assert_eq!(
            licence(Some("https://creativecommons.org/licenses/by-nc/4.0/")),
            Licence::NonCommercial
        );
        assert_eq!(licence(Some("cc-by-nc-nd-4.0")), Licence::NoDerivatives);
        assert_eq!(licence(Some("AGPLv3")), Licence::Copyleft);
        assert_eq!(licence(Some("gpl-3.0")), Licence::Copyleft);
        assert_eq!(
            licence(Some(
                "https://www.cstr.ed.ac.uk/projects/blizzard/2013/lessac_blizzard2013/license.html"
            )),
            Licence::Restricted
        );
        assert_eq!(licence(Some("Valve")), Licence::Restricted);
        assert_eq!(licence(Some("See URL")), Licence::Unknown);
        assert_eq!(licence(None), Licence::Unknown);
    }

    #[test]
    fn the_rule_sorts_an_excerpt_of_the_catalog() {
        let all = excerpt();
        let verdict = |slug: &str| admit(all.iter().find(|m| m.slug == slug).expect(slug), &all);
        // Official, their datasets permissive (CC BY, CC BY-SA, CC0, public domain) or unknown.
        for ok in [
            "siwis-french-fr-medium",
            "upmc-french-fr-medium",
            "gilles-french-fr-low",
            "libritts-english-us-high",
            "ljspeech-english-us-medium",
            "alan-english-gb-medium",
            // Of the community, checked by hand.
            "colin-english-en-high",
            // By a decision of the user (`EXCEPTIONS`).
            "fuze-french-fr-multi-high",
        ] {
            assert_eq!(verdict(ok), Verdict::Admitted, "{ok}");
        }
        // Their datasets: AGPL, non-commercial, Blizzard's licence.
        assert_eq!(
            verdict("tom-french-fr-medium"),
            Verdict::DatasetLicence(Licence::Copyleft)
        );
        assert_eq!(
            verdict("ryan-english-us-high"),
            Verdict::DatasetLicence(Licence::NonCommercial)
        );
        assert_eq!(
            verdict("lessac-english-us-medium-1"),
            Verdict::DatasetLicence(Licence::Restricted)
        );
        // Their models: NC-ND, GPL (to confirm).
        assert_eq!(
            verdict("miro-french-fr-medium-openvoiceos"),
            Verdict::ModelLicence(Licence::NoDerivatives)
        );
        assert_eq!(
            verdict("mei-english-us-high"),
            Verdict::ModelLicence(Licence::Copyleft)
        );
        // Characters, persons and brands, MIT or CC BY as they may be.
        for c in [
            "glados-portal-1-french-fr-clean",
            "wheatley-french-fr-medium",
            "death-english-us-high",
            "eminem-english-en-medium",
            "astra-valorant-piper-tts",
            "itsrealfortune-french-fr-high",
        ] {
            assert!(
                matches!(verdict(c), Verdict::Character(_)),
                "{c}: {:?}",
                verdict(c)
            );
        }
        // A copy of Siwis elsewhere: the official one is offered.
        assert_eq!(verdict("siwis-french-medium-piper-voices"), Verdict::Copy);
        // MIT, but whose voice it is nobody says.
        assert_eq!(
            verdict("sarah-atlas-english-us-medium"),
            Verdict::UnknownOrigin
        );
    }

    #[test]
    fn an_exception_admits_its_voice_and_no_other_of_the_same_kind() {
        let all = excerpt();
        let fuze = all
            .iter()
            .find(|m| m.slug == "fuze-french-fr-multi-high")
            .unwrap()
            .clone();
        assert_eq!(admit(&fuze, &all), Verdict::Admitted);
        // The same voice under another id, or another model under its id: the rule again.
        let mut other = fuze.clone();
        other.slug = "fuze-french-fr-multi-medium".into();
        assert!(
            matches!(admit(&other, &all), Verdict::Character(_)),
            "{:?}",
            admit(&other, &all)
        );
        let mut other = fuze.clone();
        other.files.onnx.as_mut().unwrap().sha256 = Some("0".repeat(64));
        assert!(matches!(admit(&other, &all), Verdict::Character(_)));
        // Its kind: a character's universe, no licence for its model, or a licence that is a company's name.
        let mut other = all
            .iter()
            .find(|m| m.slug == "siwis-french-fr-medium")
            .unwrap()
            .clone();
        other.slug = "someone-french-fr-multi-high".into();
        other.repo = "someone/piper-checkpoints".into();
        other.hub.universe = Some("someone".into());
        assert_eq!(admit(&other, &all), Verdict::Character("someone".into()));
        other.hub.universe = None;
        other.hf.license = None;
        other.hub.license = None;
        assert_eq!(admit(&other, &all), Verdict::ModelLicence(Licence::Unknown));
        other.hub.license = Some("SandBlock Studios".into());
        assert_eq!(
            admit(&other, &all),
            Verdict::ModelLicence(Licence::Restricted)
        );

        // As shipped: its licences as read by hand, not PiperHub's « SandBlock Studios ».
        let v = voice_info(&fuze).unwrap();
        assert!(v.exception);
        assert_eq!(v.license, "", "its model declares none");
        assert_eq!(v.dataset_license.as_deref(), Some("CC BY 4.0"));
        let credit = v.attribution.as_deref().unwrap();
        assert!(
            credit.contains("fox3000foxy") && credit.contains("ElevenLabs v4"),
            "{credit}"
        );
        assert_eq!(&v, find("fuze-french-fr-multi-high").expect("shipped"));
        assert_eq!(v.speakers, 14);
        assert_eq!(v.speaker_names[0], "normal");
        assert!(allowed(&v));
        let mut w = v.clone();
        w.id = "fuze-french-fr-multi-medium".into();
        assert!(!allowed(&w));
        let mut w = v.clone();
        w.onnx_sha256 = "0".repeat(64);
        assert!(!allowed(&w));
        // The other voices of the list are not exceptions.
        assert_eq!(shipped().iter().filter(|v| v.exception).count(), 1);
    }

    #[test]
    fn hugging_face_s_licence_prevails_over_piperhub_s() {
        let all = excerpt();
        let mut m = all
            .iter()
            .find(|m| m.slug == "siwis-french-fr-medium")
            .unwrap()
            .clone();
        m.hub.license = Some("SandBlock Studios".into());
        assert_eq!(admit(&m, &all), Verdict::Admitted);
        m.hf.license = Some("cc-by-nc-4.0".into());
        assert_eq!(
            admit(&m, &all),
            Verdict::ModelLicence(Licence::NonCommercial)
        );
    }

    #[test]
    fn a_cc_by_voice_carries_its_attribution() {
        let all = excerpt();
        let siwis = voice_info(
            all.iter()
                .find(|m| m.slug == "siwis-french-fr-medium")
                .unwrap(),
        )
        .unwrap();
        assert_eq!(siwis.id, "fr_FR-siwis-medium");
        assert_eq!(siwis.locale.as_deref(), Some("fr_FR"));
        assert!(
            siwis
                .attribution
                .as_deref()
                .unwrap()
                .contains("datashare.is.ed.ac.uk"),
            "{:?}",
            siwis.attribution
        );
        assert!(siwis.attribution.as_deref().unwrap().contains("CC-BY 4.0"));
        let gilles = voice_info(
            all.iter()
                .find(|m| m.slug == "gilles-french-fr-low")
                .unwrap(),
        )
        .unwrap();
        assert_eq!(gilles.attribution, None, "CC0 asks for none");
        let colin = voice_info(
            all.iter()
                .find(|m| m.slug == "colin-english-en-high")
                .unwrap(),
        )
        .unwrap();
        assert!(colin.attribution.unwrap().contains("nardocolin"));
    }

    #[test]
    fn a_voice_of_the_list_that_breaks_the_rule_is_not_offered() {
        let mut v = find("fr_FR-siwis-medium").unwrap().clone();
        assert!(allowed(&v));
        v.dataset_license = Some("CC BY-NC 4.0".into());
        assert!(!allowed(&v));
        let mut v = find("fr_FR-siwis-medium").unwrap().clone();
        v.name = "GLaDOS".into();
        assert!(!allowed(&v));
        let all: Vec<VoiceInfo> = serde_json::from_str(include_str!("voices.json")).unwrap();
        assert_eq!(
            all.len(),
            shipped().len(),
            "the list as written passes the rule"
        );
    }

    #[test]
    fn the_shipped_list_holds_only_admitted_voices_with_their_checksums() {
        let list = shipped();
        assert!(list.iter().any(|v| v.lang == "fr") && list.iter().any(|v| v.lang == "en"));
        for lang in ["fr", "en"] {
            assert!(find(default_voice(lang)).is_some(), "{lang}");
        }
        for v in list {
            assert_eq!(v.onnx_sha256.len(), 64, "{}", v.id);
            assert_eq!(v.json_sha256.len(), 64, "{}", v.id);
            if v.exception {
                assert!(exception(&v.id, Some(&v.onnx_sha256)).is_some(), "{}", v.id);
                continue;
            }
            assert!(
                matches!(licence(Some(&v.license)), Licence::Permissive { .. }),
                "{}",
                v.id
            );
            let lower = v.id.to_lowercase();
            assert!(!NOT_OURS.iter().any(|n| lower.contains(n)), "{}", v.id);
        }
    }

    /// Writes `voices.json` from the snapshot of `scripts/piper-voices.mjs`, and prints why each voice left out is.
    #[test]
    #[ignore]
    fn write_the_voice_catalog() {
        #[derive(Deserialize)]
        struct Snapshot {
            models: Vec<Model>,
        }
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let s: Snapshot = serde_json::from_str(
            &std::fs::read_to_string(root.join("../../scripts/piper-voices/snapshot.json"))
                .unwrap(),
        )
        .unwrap();
        let mut out = Vec::new();
        for m in &s.models {
            match admit(m, &s.models) {
                Verdict::Admitted => out.push(voice_info(m).expect("its files are known")),
                v => println!("{:5} {:45} {v:?}", m.lang, m.slug),
            }
        }
        out.sort_by(|a, b| (&a.lang, &a.id).cmp(&(&b.lang, &b.id)));
        println!("{} voices admitted", out.len());
        std::fs::write(
            root.join("src/speech/voices.json"),
            serde_json::to_string_pretty(&out).unwrap() + "\n",
        )
        .unwrap();
    }
}
