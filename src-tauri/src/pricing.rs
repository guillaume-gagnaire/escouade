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

/// Claude Haiku 5.5's prompts over this many tokens (input, cache reads and writes) pay its higher prices.
const HAIKU_5_5_LONG: u64 = 100_000;

/// List price of a model id (`claude-opus-5-5`, `claude-haiku-4-5-20251001`, `gpt-6-luna`…), if
/// known; for a model priced by the length of its prompt (Claude Haiku 5.5), its prices for a short one.
pub fn price(model: &str) -> Option<Price> {
    price_for(model, 0)
}

/// As `price`, for a request whose prompt is `prompt` tokens long (input, cache reads and writes).
/// platform.claude.com/docs/en/about-claude/pricing, read 2026-11-11.
pub fn price_for(model: &str, prompt: u64) -> Option<Price> {
    let lower = model.trim().to_ascii_lowercase();
    if lower.starts_with("gpt-") || lower.starts_with("o4-") {
        return openai(&lower);
    }
    let m = lower.replace('.', "-");
    // Claude's go by their aliases too (`opus`, `sonnet[1m]`): Claude Code runs its family's latest model for one.
    let bare = m.trim_start_matches("claude-");
    let bare = bare.split('[').next().unwrap_or(bare);
    let m = match bare {
        "fable" => "fable-5-2",
        "opus" => "opus-5-5",
        "sonnet" => "sonnet-5-5",
        "haiku" => "haiku-5-5",
        _ => m.as_str(),
    };
    let has = |s: &str| m.contains(s);
    Some(if has("fable-5-2") || has("mythos-5-2") {
        p(8.0, 40.0, 0.025)
    } else if has("fable-5-1") || has("mythos-5-1") {
        p(10.0, 50.0, 0.025)
    } else if has("fable") || has("mythos") {
        p(10.0, 50.0, 0.1)
    } else if has("opus-5-5") {
        p(4.0, 20.0, 0.05)
    } else if has("opus-4-1") || has("opus-4-2025") || m.ends_with("opus-4") {
        p(15.0, 75.0, 0.1)
    } else if has("opus") {
        p(5.0, 25.0, 0.1)
    } else if has("sonnet-5-5") {
        p(2.0, 10.0, 0.05)
    } else if has("sonnet-5") {
        p(2.0, 10.0, 0.1)
    } else if has("sonnet") {
        p(3.0, 15.0, 0.1)
    } else if has("haiku-5-5") && prompt > HAIKU_5_5_LONG {
        p(0.5, 2.5, 0.1)
    } else if has("haiku-5-5") {
        p(0.1, 0.5, 0.1)
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
    let prompt = usage.input + usage.cache_read + usage.write_5m + usage.write_1h;
    Some(cost(price_for(model, prompt)?, usage))
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

    /// One million tokens of each kind: what a model's five prices add up to.
    fn million() -> Usage {
        Usage {
            input: 1_000_000,
            output: 1_000_000,
            cache_read: 1_000_000,
            write_5m: 1_000_000,
            write_1h: 1_000_000,
            fast: false,
        }
    }

    // platform.claude.com/docs/en/about-claude/pricing, read 2026-11-11: input, 5-minute and 1-hour writes, cache hits
    // (0.025× the input on Fable and Mythos 5.1 and 5.2, 0.05× on Opus and Sonnet 5.5), output.
    #[test]
    fn prices_each_current_model_as_the_price_list_does() {
        let u = million();
        for (model, cost) in [
            ("claude-fable-5-2", 8.0 + 10.0 + 16.0 + 0.20 + 40.0),
            ("claude-mythos-5-2", 8.0 + 10.0 + 16.0 + 0.20 + 40.0),
            ("claude-fable-5-1", 10.0 + 12.5 + 20.0 + 0.25 + 50.0),
            ("claude-mythos-5-1", 10.0 + 12.5 + 20.0 + 0.25 + 50.0),
            ("claude-fable-5", 10.0 + 12.5 + 20.0 + 1.0 + 50.0),
            ("claude-opus-5-5", 4.0 + 5.0 + 8.0 + 0.20 + 20.0),
            ("claude-opus-5", 5.0 + 6.25 + 10.0 + 0.50 + 25.0),
            ("claude-opus-4-8", 5.0 + 6.25 + 10.0 + 0.50 + 25.0),
            ("claude-sonnet-5-5", 2.0 + 2.5 + 4.0 + 0.10 + 10.0),
            ("claude-sonnet-5", 2.0 + 2.5 + 4.0 + 0.20 + 10.0),
            ("claude-sonnet-4-6", 3.0 + 3.75 + 6.0 + 0.30 + 15.0),
            // Its prompt is 4 million tokens here: the prices for prompts over 100,000 tokens.
            ("claude-haiku-5-5", 0.50 + 0.625 + 1.0 + 0.05 + 2.50),
            ("claude-haiku-4-5-20251001", 1.0 + 1.25 + 2.0 + 0.10 + 5.0),
        ] {
            let got = estimate(model, &u).unwrap_or(f64::NAN);
            assert!(close(got, cost), "{model}: {got} instead of {cost}");
        }
    }

    // A request's prompt length counts all its input tokens, cache reads and writes included; over 100,000 tokens,
    // the whole request pays the higher prices.
    #[test]
    fn haiku_5_5_is_priced_by_the_length_of_its_prompt() {
        let short = Usage {
            input: 40_000,
            cache_read: 60_000,
            output: 1_000_000,
            ..Default::default()
        };
        let small = 40_000.0 * 0.10 + 60_000.0 * 0.01 + 1_000_000.0 * 0.50;
        assert!(close(
            estimate("claude-haiku-5-5", &short).unwrap(),
            small / 1e6
        ));
        let long = Usage {
            cache_read: 60_001,
            ..short.clone()
        };
        let large = 40_000.0 * 0.50 + 60_001.0 * 0.05 + 1_000_000.0 * 2.50;
        assert!(close(
            estimate("claude-haiku-5-5", &long).unwrap(),
            large / 1e6
        ));
        // Its price alone, the length unknown: the prices up to 100,000 tokens.
        assert_eq!(price("claude-haiku-5-5").map(|p| p.input), Some(0.10));
        assert_eq!(
            price_for("claude-haiku-5-5", 100_001).map(|p| p.input),
            Some(0.50)
        );
    }

    // Claude Code's aliases run the latest model of their family (2.1.321: `fable` is Claude Fable 5.2).
    #[test]
    fn an_alias_without_a_version_is_its_family_s_latest_model() {
        for (alias, model) in [
            ("fable", "claude-fable-5-2"),
            ("fable[1m]", "claude-fable-5-2"),
            ("opus", "claude-opus-5-5"),
            ("opus[1m]", "claude-opus-5-5"),
            ("sonnet", "claude-sonnet-5-5"),
            ("sonnet[1m]", "claude-sonnet-5-5"),
            ("haiku", "claude-haiku-5-5"),
        ] {
            assert_eq!(price(alias), price(model), "{alias}");
        }
    }

    #[test]
    fn fast_mode_costs_twice_as_much() {
        let fable = Usage {
            output: 1_000_000,
            fast: true,
            ..Default::default()
        };
        assert!(close(estimate("claude-fable-5-2", &fable).unwrap(), 80.0));
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
