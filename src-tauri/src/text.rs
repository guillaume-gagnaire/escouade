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

/// `s` without its ANSI codes: CSI sequences (colours, cursor) and OSC ones (a link, a title), as a terminal prints them.
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            match chars.peek() {
                Some(&'[') => {
                    chars.next();
                    for n in chars.by_ref() {
                        if n.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
                // An OSC (a link, a title): to its BEL or its ST (ESC \).
                Some(&']') => {
                    while let Some(n) = chars.next() {
                        if n == '\u{7}' {
                            break;
                        }
                        if n == '\u{1b}' && chars.peek() == Some(&'\\') {
                            chars.next();
                            break;
                        }
                    }
                }
                _ => {}
            }
        } else {
            out.push(c);
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
