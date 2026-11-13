//! What a line of text shows: the characters that are drawn, on one line; a text cut to a length; a slug.

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

/// Kebab-case ASCII slug from free text (accents folded), of at most 40 characters.
pub fn slugify(s: &str) -> String {
    slug_within(s, 40)
}

/// Kebab-case ASCII slug from free text (accents folded), of at most `max` characters: whole
/// words while they fit, the first one cut when it alone is longer.
pub fn slug_within(s: &str, max: usize) -> String {
    let folded: String = s
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ä' | 'ã' => 'a',
            'ç' => 'c',
            'è' | 'é' | 'ê' | 'ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' => 'i',
            'ò' | 'ó' | 'ô' | 'ö' | 'õ' => 'o',
            'ù' | 'ú' | 'û' | 'ü' => 'u',
            'ÿ' => 'y',
            'ñ' => 'n',
            c if c.is_ascii_alphanumeric() => c,
            _ => '-',
        })
        .collect();
    let mut out = String::new();
    for part in folded.split('-').filter(|p| !p.is_empty()) {
        if out.len() + part.len() + 1 > max {
            if out.is_empty() {
                // ASCII only by now: any byte ends a character.
                out.push_str(&part[..max.min(part.len())]);
            }
            break;
        }
        if !out.is_empty() {
            out.push('-');
        }
        out.push_str(part);
    }
    out
}

/// Percent-encoding of everything but the unreserved characters (and `/` when `keep_slash`).
pub fn percent_encode(s: &str, keep_slash: bool) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) || (keep_slash && b == b'/') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// A character that is not drawn, or drawn as something else than itself: the controls, the
/// format characters (zero-width marks, direction marks and overrides…), the line and paragraph
/// separators and the ones ignored when drawn (variation selectors, tag characters…). Written as
/// ranges as the window's `revealHidden` has them (it decides by Unicode property, which `std`
/// does not know).
fn is_hidden(c: char) -> bool {
    const RANGES: &[(u32, u32)] = &[
        (0x00AD, 0x00AD),   // soft hyphen
        (0x034F, 0x034F),   // combining grapheme joiner
        (0x0600, 0x0605),   // Arabic number signs
        (0x061C, 0x061C),   // Arabic letter mark
        (0x06DD, 0x06DD),   // Arabic end of ayah
        (0x070F, 0x070F),   // Syriac abbreviation mark
        (0x0890, 0x0891),   // Arabic pound and piastre marks
        (0x08E2, 0x08E2),   // Arabic disputed end of ayah
        (0x115F, 0x1160),   // hangul fillers
        (0x17B4, 0x17B5),   // Khmer inherent vowels
        (0x180B, 0x180F),   // Mongolian free variation selectors
        (0x200B, 0x200F),   // zero-width marks, direction marks
        (0x2028, 0x202E),   // separators, direction embeddings and overrides
        (0x2060, 0x206F),   // word joiner, invisible operators, deprecated format characters
        (0x2800, 0x2800),   // braille blank
        (0x3164, 0x3164),   // hangul filler
        (0xFE00, 0xFE0F),   // variation selectors
        (0xFEFF, 0xFEFF),   // zero-width no-break space
        (0xFFA0, 0xFFA0),   // halfwidth hangul filler
        (0xFFF0, 0xFFFB),   // unassigned, interlinear annotation
        (0x110BD, 0x110BD), // Kaithi number sign
        (0x110CD, 0x110CD), // Kaithi number sign above
        (0x13430, 0x1343F), // Egyptian hieroglyph format controls
        (0x1BCA0, 0x1BCA3), // shorthand format controls
        (0x1D173, 0x1D17A), // musical format controls
        (0xE0000, 0xE0FFF), // tag characters, variation selectors supplement
    ];
    c.is_control()
        || RANGES
            .iter()
            .any(|&(from, to)| (from..=to).contains(&(c as u32)))
}

/// `text` as a status line is kept: on one line (breaks, tabs and runs of spaces are single
/// spaces) and with none of the characters that are not drawn (`is_hidden`), which could hide
/// part of it or reverse its reading (U+202E).
pub fn visible_line(text: &str) -> String {
    let spaced: String = text
        .chars()
        .filter_map(|c| match c {
            c if c.is_whitespace() || c.is_control() => Some(' '),
            c if is_hidden(c) => None,
            c => Some(c),
        })
        .collect();
    spaced.split_whitespace().collect::<Vec<_>>().join(" ")
}
