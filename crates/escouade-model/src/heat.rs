//! The temperature of an agent's cache (ESC-398, docs/FEATURES.md §3): when its cache last served, how long it lives
//! without a request, and what its last turn read from it, wrote to it and read at full price. The window counts the
//! degrees down from `CacheHeat::at` over `CacheHeat::ttl` (100 °C → 0 °C); this module only keeps what the harness said.
//!
//! What each harness tells, and so what is shown:
//! - Claude Code: the API's `usage` of each message (`cache_read_input_tokens`, `cache_creation_input_tokens`, and the
//!   writes for 5 minutes or 1 hour in `cache_creation.ephemeral_5m_input_tokens` / `ephemeral_1h_input_tokens`): its
//!   cache lives 1 hour when its last request wrote for 1 hour, 5 minutes otherwise.
//! - The others tell their turn's tokens (`TurnUsage`), the cached ones in one figure (read and written together). The
//!   cache lives as its vendor documents it: 5 minutes without a request at Anthropic (a write for 1 hour is asked
//!   explicitly, which only Claude Code tells), 5 to 10 minutes at OpenAI (the shorter is taken; OpenAI writes cost
//!   nothing, so what is cached was read). Any other model's cache (Gemini, Mistral…) has a life no one tells: « — °C ».
//! - A harness that tells no cached token (Cursor CLI) has no temperature: « — °C ».

use crate::model::AgentMeta;
use crate::pricing::{self, Usage};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 5 minutes: Anthropic's cache without a request (and OpenAI's shortest).
pub const FIVE_MINUTES: u64 = 300;
/// 1 hour: Anthropic's cache written for an hour (`ephemeral_1h`).
pub const ONE_HOUR: u64 = 3600;

/// Why a cache went cold before its time: the next turn reads everything again.
pub mod broken {
    pub const MODEL: &str = "model";
    pub const COMPACTION: &str = "compaction";
    pub const PROMPT: &str = "prompt";
    pub const ACCOUNT: &str = "account";
    pub const CONVERSATION: &str = "conversation";
}

/// The state of an agent's cache (`AgentMeta::heat`), saved with it: the cache lives at its vendor, not here, so a
/// restart of the app keeps it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CacheHeat {
    /// When it last served (ms): the last request of the agent that read or wrote it.
    pub at: i64,
    /// How long it lives without a request (s); None: its harness does not tell (« — °C »).
    pub ttl: Option<u64>,
    /// Why it went cold before its time (`broken::*`): 0 °C until a turn warms it again.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub broken: Option<String>,
    /// The tokens of its last turn read from the cache; None when its harness tells them only with the written ones.
    pub read: Option<u64>,
    /// Its tokens written to the cache; None when told only with the read ones.
    pub written: Option<u64>,
    /// Read and written together, as every harness tells them.
    pub cached: u64,
    /// Its input read at full price.
    pub fresh: u64,
    /// What the context holds after it: what a next message reads again.
    pub context: u64,
    /// What the cache saved on its last turn (USD): what it read, at the price of the input less the price of a read.
    /// None when the price or the tokens read are not known.
    pub saved: Option<f64>,
    /// What the next message would cost to read its context, the cache warm, then cold (USD).
    pub warm: Option<f64>,
    pub cold: Option<f64>,
    /// The model of its last request, as its harness names it (priced by `pricing::price`).
    pub model: String,
    /// What it was warmed under: the agent's model, account and added instructions (`AgentMeta::append_prompt`,
    /// hashed). Any of them changed, the prefix the vendor cached is no longer the one sent: cold.
    pub set_model: String,
    pub account: String,
    pub prompt: u64,
    /// The local day (`YYYY-MM-DD`) of `saved_today`: what the cache saved on that day, summed.
    pub day: String,
    pub saved_today: f64,
}

/// A request's cache, as one turn adds them up.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Served {
    pub read: Option<u64>,
    pub written: Option<u64>,
    pub cached: u64,
    pub fresh: u64,
    pub saved: Option<f64>,
}

impl Served {
    fn add(self, o: Served) -> Served {
        let both = |a: Option<u64>, b: Option<u64>| Some(a? + b?);
        Served {
            read: both(self.read, o.read),
            written: both(self.written, o.written),
            cached: self.cached + o.cached,
            fresh: self.fresh + o.fresh,
            saved: match (self.saved, o.saved) {
                (Some(a), Some(b)) => Some(a + b),
                _ => None,
            },
        }
    }
}

/// The vendor of a model, by its name (`claude-opus-5-5`, `anthropic/claude-sonnet-4-5`, `gpt-6-luna`, `o4-mini`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vendor {
    Anthropic,
    OpenAi,
    Other,
}

pub fn vendor(model: &str) -> Vendor {
    let m = model.trim().to_ascii_lowercase();
    // A provider's prefix (OpenCode's `openai/gpt-5`) goes.
    let name = m.rsplit('/').next().unwrap_or(&m);
    let anthropic = ["claude", "opus", "sonnet", "haiku", "fable", "mythos"];
    if anthropic.iter().any(|w| name.contains(w)) {
        Vendor::Anthropic
    } else if name.starts_with("gpt-")
        || name.contains("codex")
        || (name.starts_with('o') && name[1..].starts_with(|c: char| c.is_ascii_digit()))
    {
        Vendor::OpenAi
    } else {
        Vendor::Other
    }
}

/// How long the cache lives without a request, for a model's vendor, when its harness tells only that something was
/// cached (`TurnUsage::cache`).
pub fn vendor_ttl(model: &str) -> Option<u64> {
    match vendor(model) {
        Vendor::Anthropic | Vendor::OpenAi => Some(FIVE_MINUTES),
        Vendor::Other => None,
    }
}

/// How long Claude Code's cache lives after a request whose API `usage` is `u`: 1 hour once it wrote for 1 hour, 5
/// minutes once it wrote for 5; one that only read keeps the life it had (`before`), 5 minutes when it had none told
/// (the API's default).
pub fn claude_ttl(u: &Value, before: Option<u64>) -> u64 {
    let n = |v: &Value| v.as_u64().unwrap_or(0);
    let split = &u["cache_creation"];
    if split.is_object() {
        if n(&split["ephemeral_1h_input_tokens"]) > 0 {
            return ONE_HOUR;
        }
        if n(&split["ephemeral_5m_input_tokens"]) > 0 {
            return FIVE_MINUTES;
        }
    } else if n(&u["cache_creation_input_tokens"]) > 0 {
        return FIVE_MINUTES;
    }
    before.unwrap_or(FIVE_MINUTES)
}

/// What a request of Claude Code's (its API `usage`) did with the cache, on `model`.
pub fn claude_served(model: &str, u: &Value) -> Served {
    let usage = Usage::from_api(u);
    let written = usage.write_5m + usage.write_1h;
    Served {
        read: Some(usage.cache_read),
        written: Some(written),
        cached: usage.cache_read + written,
        fresh: usage.input,
        saved: saved(model, usage.cache_read, usage.fast),
    }
}

/// What a turn told by its tokens (`TurnUsage`) did with the cache: read and written apart only for OpenAI, whose
/// writes do not exist (what is cached was read).
pub fn turn_served(model: &str, fresh: u64, cache: u64) -> Served {
    let apart = vendor(model) == Vendor::OpenAi;
    Served {
        read: apart.then_some(cache),
        written: apart.then_some(0),
        cached: cache,
        fresh,
        saved: if apart {
            saved(model, cache, false)
        } else {
            None
        },
    }
}

/// What `read` tokens read from the cache saved on `model` (USD), against reading them at full price.
fn saved(model: &str, read: u64, fast: bool) -> Option<f64> {
    let p = pricing::price(model)?;
    let x = if fast { 2.0 } else { 1.0 };
    Some(read as f64 * (p.input - p.cache_read).max(0.0) / 1e6 * x)
}

/// What a next message would cost to read `context` tokens again on `model`: the cache warm (read), then cold (read at
/// full price, and written again for the life it had: 1.25× for 5 minutes, 2× for 1 hour at Anthropic).
pub fn next_message(model: &str, context: u64, ttl: Option<u64>) -> (Option<f64>, Option<f64>) {
    // Its prompt is the context read again: Claude Haiku 5.5 prices a long one higher.
    let Some(p) = pricing::price_for(model, context) else {
        return (None, None);
    };
    let c = context as f64 / 1e6;
    let cold = match (vendor(model), ttl) {
        (Vendor::Anthropic, Some(ONE_HOUR)) => p.write_1h,
        (Vendor::Anthropic, _) => p.write_5m,
        _ => p.input,
    };
    (Some(c * p.cache_read), Some(c * cold))
}

/// A short hash of the added instructions, to tell when they change (FNV-1a: the same from one version to the next).
pub fn prompt_hash(prompt: Option<&str>) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in prompt.unwrap_or_default().trim().bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// The local day of `ms`, as `CacheHeat::day` keeps it.
pub fn day_of(ms: i64) -> String {
    use chrono::TimeZone;
    chrono::Local
        .timestamp_millis_opt(ms)
        .single()
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

/// The turn that warms the cache: what it served so far, its model and the life the cache has.
#[derive(Debug, Clone, Default)]
pub struct Warming {
    pub served: Served,
    pub model: String,
    pub ttl: Option<u64>,
    /// Its harness told it request by request (Claude Code): its `TurnUsage` at the end adds nothing.
    pub by_message: bool,
    /// What the cache had saved on its day when the turn began (`CacheHeat::day`, `saved_today`).
    pub base: Option<(String, f64)>,
}

impl Warming {
    /// A turn begins, after `before`.
    pub fn open(before: Option<&CacheHeat>) -> Warming {
        Warming {
            // Nothing served yet: every figure known, at zero, until a request tells one in a way that is not.
            served: Served {
                read: Some(0),
                written: Some(0),
                saved: Some(0.0),
                ..Default::default()
            },
            base: before.map(|b| (b.day.clone(), b.saved_today)),
            ..Default::default()
        }
    }

    pub fn add(&mut self, s: Served) {
        self.served = self.served.add(s);
    }
}

impl CacheHeat {
    /// The cache served at `at` for `meta`: what the turn did so far, its context after it.
    pub fn warmed(meta: &AgentMeta, w: &Warming, context: u64, at: i64) -> CacheHeat {
        let (warm, cold) = next_message(&w.model, context, w.ttl);
        let day = day_of(at);
        // What the day had saved before the turn, and what the turn saved so far (told again at each of its requests).
        let today = w
            .base
            .as_ref()
            .filter(|(d, _)| *d == day)
            .map_or(0.0, |(_, s)| *s)
            + w.served.saved.unwrap_or(0.0);
        CacheHeat {
            at,
            ttl: w.ttl,
            broken: None,
            read: w.served.read,
            written: w.served.written,
            cached: w.served.cached,
            fresh: w.served.fresh,
            context,
            saved: w.served.saved,
            warm,
            cold,
            model: w.model.clone(),
            set_model: meta.model.clone(),
            account: meta.account.clone(),
            prompt: prompt_hash(meta.append_prompt.as_deref()),
            day,
            saved_today: today,
        }
    }

    /// As the window shows it: cold when what the cache was warmed under changed since (its model, its account, its added
    /// instructions).
    pub fn seen(&self, meta: &AgentMeta) -> CacheHeat {
        let mut h = self.clone();
        if h.broken.is_none() {
            h.broken = if meta.model != h.set_model {
                Some(broken::MODEL.into())
            } else if meta.account != h.account {
                Some(broken::ACCOUNT.into())
            } else if prompt_hash(meta.append_prompt.as_deref()) != h.prompt {
                Some(broken::PROMPT.into())
            } else {
                None
            };
        }
        h
    }
}

#[cfg(test)]
#[path = "heat_tests.rs"]
mod tests;
