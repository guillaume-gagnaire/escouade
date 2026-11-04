//! The usage limits Google's harnesses tell in their words (Antigravity CLI's, read in its program, ESC-269, and Gemini
//! CLI's, which says the same of its model's capacity).

/// How long after a limit of the model's capacity (not the account's) its agent tries again.
pub const CAPACITY_RESUME_MS: i64 = 60_000;

/// How long after its quota was met its agent tries again when `agy` tells no reset: its plans' quotas are weekly and
/// refreshed by parts (`/usage`: « Refreshes in 167h 49m »), a turn tried for nothing costs none.
pub const QUOTA_RESUME_MS: i64 = 3_600_000;

/// The usage limit an error of `agy` tells (its words read in its program, ESC-269: « You have exhausted your quota on
/// this model. », « individual quota reached », `RESOURCE_EXHAUSTED`, `MODEL_CAPACITY_EXHAUSTED`, an HTTP 429): in how many
/// ms to try again, the reset it tells (« in 2h 5m », « after 30s ») else a capacity's minute or a quota's hour. None for
/// another error.
pub fn limit_of(error: &str) -> Option<i64> {
    let lower = error.to_lowercase();
    let capacity = lower.contains("model_capacity_exhausted") || lower.contains("capacity");
    let quota = [
        "exhausted your quota",
        "quota reached",
        "quota exceeded",
        "resource_exhausted",
        "rate limit",
        "too many requests",
        " 429",
    ]
    .iter()
    .any(|w| lower.contains(w))
        || lower.starts_with("429");
    if !capacity && !quota {
        return None;
    }
    Some(
        told_wait(&lower).unwrap_or(if quota && !lower.contains("capacity") {
            QUOTA_RESUME_MS
        } else {
            CAPACITY_RESUME_MS
        }),
    )
}

/// The wait a text tells after « in », « after » or « retry »: `2h 5m`, `167h 49m`, `30s`, `1d`.
fn told_wait(lower: &str) -> Option<i64> {
    [
        "refreshes in",
        "resets in",
        "reset in",
        "retry in",
        "retry after",
        "try again in",
        "after",
    ]
    .iter()
    .flat_map(|w| lower.match_indices(w).map(move |(i, _)| i + w.len()))
    .find_map(|at| wait_at(&lower[at..]))
}

/// The wait `text` starts with (` 2h 5m…`), none when it starts with none.
fn wait_at(text: &str) -> Option<i64> {
    let mut total = 0i64;
    let mut num = String::new();
    for c in text.chars() {
        match c {
            '0'..='9' => num.push(c),
            ' ' | ',' if num.is_empty() => {}
            _ if num.is_empty() => break,
            _ => {
                let unit = match c {
                    'd' => 86_400_000,
                    'h' => 3_600_000,
                    'm' => 60_000,
                    's' => 1_000,
                    _ => break,
                };
                total += num.parse::<i64>().ok()? * unit;
                num.clear();
            }
        }
    }
    (total > 0).then_some(total)
}
