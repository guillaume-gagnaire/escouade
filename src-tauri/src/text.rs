//! Small helpers on text.

/// `s` cut to `max` bytes at most, on a character's boundary, with `…` when it was cut.
pub fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut end = max;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &s[..end])
}

/// The JSON value of a harness's line. A harness written in JavaScript escapes a lone half of a
/// UTF-16 pair (`\ud83c`), which JSON allows and serde refuses: the output of a tool that read a
/// binary file was lost whole (ESC-290). Such a half is read as `�` (�).
pub fn json_line(line: &str) -> serde_json::Result<serde_json::Value> {
    match serde_json::from_str(line) {
        Err(e) if line.contains("\\ud") || line.contains("\\uD") => {
            serde_json::from_str(&lone_halves_replaced(line)).map_err(|_| e)
        }
        other => other,
    }
}

/// `json` with each escaped half of a UTF-16 pair that has no other half written `�`.
fn lone_halves_replaced(json: &str) -> String {
    let half = |s: &str| -> Option<u16> {
        let hex = s.strip_prefix("\\u").or_else(|| s.strip_prefix("\\U"))?;
        u16::from_str_radix(hex.get(..4)?, 16)
            .ok()
            .filter(|c| (0xd800..=0xdfff).contains(c))
    };
    let mut out = String::with_capacity(json.len());
    let mut i = 0;
    while i < json.len() {
        let rest = &json[i..];
        if !rest.starts_with('\\') {
            let c = rest.chars().next().unwrap_or_default();
            out.push(c);
            i += c.len_utf8().max(1);
            continue;
        }
        match half(rest) {
            Some(0xd800..=0xdbff)
                if half(&rest[6..]).is_some_and(|low| (0xdc00..=0xdfff).contains(&low)) =>
            {
                out.push_str(&rest[..12]);
                i += 12;
            }
            Some(_) => {
                out.push_str("\\ufffd");
                i += 6;
            }
            // Any other escape, kept whole: `\\u…` is a backslash, then text.
            None => {
                let next = rest[1..].chars().next().map_or(0, char::len_utf8);
                out.push_str(&rest[..1 + next]);
                i += 1 + next;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lone_half_of_a_utf16_pair_is_read_as_a_replacement_character() {
        let v = json_line(r#"{"t":"a\ud83cb","u":"\udc00","ok":"🎧","k":"\\ud83c"}"#).unwrap();
        assert_eq!(v["t"], "a\u{fffd}b");
        assert_eq!(v["u"], "\u{fffd}");
        assert_eq!(v["ok"], "🎧");
        assert_eq!(v["k"], "\\ud83c");
        // A line broken otherwise is still refused.
        assert!(json_line(r#"{"t":"\ud83c"#).is_err());
        assert_eq!(json_line(r#"{"a":1}"#).unwrap()["a"], 1);
    }

    #[test]
    fn truncate_respects_char_boundaries() {
        assert_eq!(truncate("héllo", 2), "h…");
        assert_eq!(truncate("abc", 5), "abc");
    }
}
