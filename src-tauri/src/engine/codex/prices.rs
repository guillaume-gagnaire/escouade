//! The public list prices of OpenAI's models, to estimate what a turn of Codex costs (it tells none).

/// USD per million tokens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Price {
    pub input: f64,
    pub cached: f64,
    pub output: f64,
}

/// OpenAI's models: id, then USD per million tokens of input, cached input and output, at the
/// standard tier and short context (developers.openai.com/api/docs/pricing, read on 2026-10-17).
/// Codex's own variants (`gpt-5-codex`) are their model's but where the grid names them.
const GRID: [(&str, f64, f64, f64); 24] = [
    ("gpt-6-astra", 10.0, 1.0, 50.0),
    ("gpt-6.1-sol", 2.0, 0.1, 10.0),
    ("gpt-6-sol", 2.0, 0.2, 10.0),
    ("gpt-6-luna", 0.1, 0.01, 0.5),
    ("gpt-5.6-sol", 4.0, 0.4, 20.0),
    ("gpt-5.6-terra", 2.0, 0.2, 12.0),
    ("gpt-5.6-luna", 0.2, 0.02, 1.2),
    ("gpt-5.5", 5.0, 0.5, 30.0),
    ("gpt-5.4-mini", 0.75, 0.075, 4.5),
    ("gpt-5.4-nano", 0.2, 0.02, 1.25),
    ("gpt-5.4", 2.5, 0.25, 15.0),
    ("gpt-5.3-codex", 1.75, 0.175, 14.0),
    ("gpt-5.2", 1.75, 0.175, 14.0),
    ("gpt-5.1-codex-mini", 0.25, 0.025, 2.0),
    ("gpt-5.1", 1.25, 0.125, 10.0),
    ("gpt-5-mini", 0.25, 0.025, 2.0),
    ("gpt-5-nano", 0.05, 0.005, 0.4),
    ("gpt-5", 1.25, 0.125, 10.0),
    ("gpt-4.1-mini", 0.4, 0.1, 1.6),
    ("gpt-4.1-nano", 0.1, 0.025, 0.4),
    ("gpt-4.1", 2.0, 0.5, 8.0),
    ("gpt-4o-mini", 0.15, 0.075, 0.6),
    ("gpt-4o", 2.5, 1.25, 10.0),
    ("o4-mini", 1.1, 0.275, 4.4),
];

/// The price of OpenAI's model `model`: the longest id of the grid it is, or is a variant of
/// (`gpt-5-2025-08-07`, `gpt-5-codex`).
pub fn price(model: &str) -> Option<Price> {
    let model = model.trim().to_ascii_lowercase();
    let mut grid = GRID;
    grid.sort_by_key(|(id, ..)| std::cmp::Reverse(id.len()));
    grid.iter()
        .find(|(id, ..)| model == *id || model.starts_with(&format!("{id}-")))
        .map(|&(_, input, cached, output)| Price {
            input,
            cached,
            output,
        })
}

#[cfg(test)]
/// What `input` fresh tokens, `cached` ones and `output` cost on `model`, if its price is known.
pub fn estimate(model: &str, input: u64, cached: u64, output: u64) -> Option<f64> {
    let p = price(model)?;
    let micro = input as f64 * p.input + cached as f64 * p.cached + output as f64 * p.output;
    Some(micro / 1e6)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn openai_models_are_priced_by_their_id_the_longest_first() {
        // developers.openai.com/api/docs/pricing, 2026-10-17: input, cached input, output.
        for (model, sum) in [
            ("gpt-6-luna", 0.10 + 0.01 + 0.50),
            ("gpt-5.6-terra", 2.0 + 0.2 + 12.0),
            ("gpt-5.6-sol", 4.0 + 0.4 + 20.0),
            ("gpt-5.4-mini", 0.75 + 0.075 + 4.5),
            ("gpt-5.4", 2.5 + 0.25 + 15.0),
            ("gpt-5-2025-08-07", 1.25 + 0.125 + 10.0),
            ("gpt-5-codex", 1.25 + 0.125 + 10.0),
            ("gpt-5.1-codex-mini", 0.25 + 0.025 + 2.0),
        ] {
            let p = price(model).unwrap_or_else(|| panic!("{model}"));
            assert!(close(p.input + p.cached + p.output, sum), "{model}");
        }
        assert_eq!(price("gpt-5.65"), None);
        assert_eq!(price("gpt-6-lunar"), None);
        assert_eq!(price("gpt-something"), None);
    }

    #[test]
    fn a_turn_is_estimated_from_its_tokens_and_an_unknown_model_is_not() {
        let cost = estimate("gpt-6-luna", 1_000_000, 1_000_000, 1_000_000).unwrap();
        assert!(close(cost, 0.61));
        assert_eq!(estimate("claude-opus-5-5", 1, 1, 1), None);
    }
}
