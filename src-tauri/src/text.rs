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

pub use escouade_plugin_protocol::json_line;

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
