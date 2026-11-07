//! Public list prices of Claude models, to estimate what a turn costs while it runs (the CLI
//! reports the exact cost only once the turn ends). OpenAI's left with Codex, in its plugin
//! (`engine::codex::prices`, INT-10.4).

use serde_json::Value;

/// USD per million tokens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Price {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub write_5m: f64,
    pub write_1h: f64,
}

/// Token usage of one API message, as reported in its stream events.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Usage {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub write_5m: u64,
    pub write_1h: u64,
    /// Fast mode is billed at twice the standard rates.
    pub fast: bool,
}

impl Usage {
    /// Reads an API `usage` object (message_start / message_delta).
    pub fn from_api(u: &Value) -> Self {
        let n = |v: &Value| v.as_u64().unwrap_or(0);
        let writes = n(&u["cache_creation_input_tokens"]);
        let split = &u["cache_creation"];
        let (write_5m, write_1h) = if split.is_object() {
            (
                n(&split["ephemeral_5m_input_tokens"]),
                n(&split["ephemeral_1h_input_tokens"]),
            )
        } else {
            (writes, 0)
        };
        Usage {
            input: n(&u["input_tokens"]),
            output: n(&u["output_tokens"]),
            cache_read: n(&u["cache_read_input_tokens"]),
            write_5m,
            write_1h,
            fast: u["speed"] == "fast",
        }
    }

    pub fn tokens(&self) -> u64 {
        self.input + self.output + self.cache_read + self.write_5m + self.write_1h
    }
}

/// Input price, output price and cache-read multiplier; writes cost 1.25× (5 min) and 2× (1 h)
/// the input price (platform.claude.com/docs/en/about-claude/pricing).
const fn p(input: f64, output: f64, read: f64) -> Price {
    Price {
        input,
        output,
        cache_read: input * read,
        write_5m: input * 1.25,
        write_1h: input * 2.0,
    }
}

/// The price of OpenAI's model `model`, from the grid of its plugin. Cache writes cost what input does.
fn openai(model: &str) -> Option<Price> {
    let p = crate::engine::codex::prices::price(model)?;
    Some(Price {
        input: p.input,
        output: p.output,
        cache_read: p.cached,
        write_5m: p.input,
        write_1h: p.input,
    })
}

/// List price of a model id (`claude-opus-5-5`, `claude-haiku-4-5-20251001`, `gpt-6-luna`…), if
/// known.
pub fn price(model: &str) -> Option<Price> {
    let lower = model.trim().to_ascii_lowercase();
    // Claude's go by their aliases too (`opus`, `sonnet[1m]`).
    if lower.starts_with("gpt-") || lower.starts_with("o4-") {
        return openai(&lower);
    }
    let m = lower.replace('.', "-");
    let has = |s: &str| m.contains(s);
    Some(if has("fable-5-1") || has("mythos-5-1") {
        p(10.0, 50.0, 0.025)
    } else if has("fable") || has("mythos") {
        p(10.0, 50.0, 0.1)
    } else if has("opus-5-5") {
        p(4.0, 20.0, 0.05)
    } else if has("opus-4-1") || has("opus-4-2025") || m.ends_with("opus-4") {
        p(15.0, 75.0, 0.1)
    } else if has("opus") {
        p(5.0, 25.0, 0.1)
    } else if has("sonnet-5") {
        p(2.0, 10.0, 0.1)
    } else if has("sonnet") {
        p(3.0, 15.0, 0.1)
    } else if has("haiku-3") {
        p(0.8, 4.0, 0.1)
    } else if has("haiku") {
        p(1.0, 5.0, 0.1)
    } else {
        return None;
    })
}

/// Estimated cost in USD of `usage` on `model`, if its price is known.
pub fn estimate(model: &str, usage: &Usage) -> Option<f64> {
    Some(cost(price(model)?, usage))
}

fn cost(p: Price, usage: &Usage) -> f64 {
    let micro = usage.input as f64 * p.input
        + usage.output as f64 * p.output
        + usage.cache_read as f64 * p.cache_read
        + usage.write_5m as f64 * p.write_5m
        + usage.write_1h as f64 * p.write_1h;
    micro / 1e6 * if usage.fast { 2.0 } else { 1.0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn usage(input: u64, output: u64, cache_read: u64, write_1h: u64) -> Usage {
        Usage {
            input,
            output,
            cache_read,
            write_1h,
            ..Default::default()
        }
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    // Turns measured with the real CLI: its `costUSD` must be matched exactly.
    #[test]
    fn matches_the_cost_the_cli_reports_for_current_models() {
        let cases = [
            (
                "claude-haiku-4-5-20251001",
                usage(18, 322, 45071, 10534),
                0.0272031,
            ),
            ("claude-opus-5-5", usage(2, 4, 15163, 13093), 0.1078646),
            ("claude-fable-5-1", usage(2, 4, 15163, 14911), 0.30223075),
            ("claude-sonnet-5", usage(2, 4, 23837, 17135), 0.0733514),
        ];
        for (model, u, cost) in cases {
            let got = estimate(model, &u).unwrap_or(f64::NAN);
            assert!(close(got, cost), "{model}: {got} instead of {cost}");
        }
    }

    #[test]
    fn prices_earlier_model_generations_and_5_minute_cache_writes() {
        let u = Usage {
            input: 1_000_000,
            write_5m: 1_000_000,
            ..Default::default()
        };
        assert!(close(
            estimate("claude-opus-4-5-20251101", &u).unwrap(),
            5.0 + 6.25
        ));
        assert!(close(
            estimate("claude-sonnet-4-6", &u).unwrap(),
            3.0 + 3.75
        ));
        assert!(close(
            estimate("claude-opus-4-1-20250805", &u).unwrap(),
            15.0 + 18.75
        ));
        assert!(close(estimate("claude-fable-5", &u).unwrap(), 10.0 + 12.5));
    }

    #[test]
    fn fast_mode_costs_twice_as_much() {
        let u = Usage {
            output: 1_000_000,
            fast: true,
            ..Default::default()
        };
        assert!(close(estimate("claude-opus-5-5", &u).unwrap(), 40.0));
    }

    #[test]
    fn openai_models_are_priced_by_their_id_the_longest_first() {
        let u = Usage {
            input: 1_000_000,
            cache_read: 1_000_000,
            output: 1_000_000,
            ..Default::default()
        };
        // developers.openai.com/api/docs/pricing, 2026-10-17: input, cached input, output.
        for (model, cost) in [
            ("gpt-6-luna", 0.10 + 0.01 + 0.50),
            ("GPT-6-Luna", 0.10 + 0.01 + 0.50),
            ("gpt-5.6-terra", 2.0 + 0.2 + 12.0),
            ("gpt-5.6-sol", 4.0 + 0.4 + 20.0),
            ("gpt-5.4-mini", 0.75 + 0.075 + 4.5),
            ("gpt-5.4", 2.5 + 0.25 + 15.0),
            // A dated or Codex variant is its model's.
            ("gpt-5-2025-08-07", 1.25 + 0.125 + 10.0),
            ("gpt-5-codex", 1.25 + 0.125 + 10.0),
            ("gpt-5.1-codex-mini", 0.25 + 0.025 + 2.0),
        ] {
            let got = estimate(model, &u).unwrap_or(f64::NAN);
            assert!(close(got, cost), "{model}: {got} instead of {cost}");
        }
        // A name that only starts like one is not it.
        assert_eq!(price("gpt-5.65"), None);
        assert_eq!(price("gpt-6-lunar"), None);
    }

    #[test]
    fn unknown_models_have_no_estimate() {
        assert_eq!(price("gpt-something"), None);
        assert_eq!(estimate("claude-unknown-9", &Usage::default()), None);
    }

    #[test]
    fn reads_api_usage_with_its_cache_write_durations() {
        let u = Usage::from_api(&json!({
            "input_tokens": 10, "output_tokens": 257, "cache_read_input_tokens": 17513,
            "cache_creation_input_tokens": 10045,
            "cache_creation": {"ephemeral_5m_input_tokens": 45, "ephemeral_1h_input_tokens": 10000},
            "speed": "fast"
        }));
        assert_eq!(
            u,
            Usage {
                input: 10,
                output: 257,
                cache_read: 17513,
                write_5m: 45,
                write_1h: 10000,
                fast: true
            }
        );
        assert_eq!(u.tokens(), 10 + 257 + 17513 + 10045);
        // Without the split, cache writes are counted as 5-minute ones.
        let old = Usage::from_api(&json!({"input_tokens": 1, "cache_creation_input_tokens": 7}));
        assert_eq!((old.write_5m, old.write_1h), (7, 0));
    }
}
