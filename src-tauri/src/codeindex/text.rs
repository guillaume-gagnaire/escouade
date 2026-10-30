//! Ported from tokensave `src/text.rs` (github.com/aovestdipaperino/tokensave, commit 580936b,
//! MIT, © 2025 Enzo Lombardi; see THIRD-PARTY-NOTICES.md): the words of a compound name and the
//! `search_terms` column of the full-text index. Only the parts the index uses are ported.

/// Split a compound name into individual words.
///
/// Handles camelCase, `PascalCase`, and `snake_case`:
/// - `getUserName` → `["get", "User", "Name"]`
/// - `process_request` → `["process", "request"]`
/// - `MAX_RETRIES` → `["MAX", "RETRIES"]`
pub fn split_compound(name: &str) -> Vec<&str> {
    if name.contains('_') {
        return name.split('_').filter(|s| !s.is_empty()).collect();
    }

    // camelCase / PascalCase splitting
    let bytes = name.as_bytes();
    let mut parts = Vec::new();
    let mut start = 0;

    for i in 1..bytes.len() {
        let cur = bytes[i] as char;
        let prev = bytes[i - 1] as char;

        // Split at lowercase→uppercase boundary (e.g. getUser → get|User)
        let boundary = prev.is_ascii_lowercase() && cur.is_ascii_uppercase();
        // Split at uppercase→uppercase+lowercase (e.g. XMLParser → XML|Parser)
        let acronym_end = i + 1 < bytes.len()
            && prev.is_ascii_uppercase()
            && cur.is_ascii_uppercase()
            && (bytes[i + 1] as char).is_ascii_lowercase();

        if boundary || acronym_end {
            if i > start {
                parts.push(&name[start..i]);
            }
            start = i;
        }
    }
    if start < name.len() {
        parts.push(&name[start..]);
    }
    parts
}

/// Returns `true` if `word` looks like CamelCase.
///
/// The word must contain at least one uppercase letter after the first character
/// and consist only of ASCII alphanumeric characters.
pub fn is_camel_case(word: &str) -> bool {
    if word.len() < 2 {
        return false;
    }
    // Must be all alphanumeric
    if !word.chars().all(|c| c.is_ascii_alphanumeric()) {
        return false;
    }
    // Must have at least one uppercase letter after the first char
    word[1..].chars().any(|c| c.is_ascii_uppercase())
}

/// Space-joined camelCase word segments for the FTS `search_terms` column.
///
/// The unicode61 tokenizer already splits on every non-alphanumeric byte
/// (`_`, `::`, `/`, `.`), so `snake_case` names and path segments are indexed
/// as separate tokens on their own. What it cannot derive are the words
/// inside a camelCase identifier — `updateCloudClient` stays one token.
/// Emit only those inner words, deduplicated case-insensitively, so the
/// column adds signal without repeating what the tokenizer already produces.
#[must_use]
pub fn search_terms(name: &str, qualified_name: &str) -> String {
    let mut seen = std::collections::HashSet::new();
    let mut out: Vec<&str> = Vec::new();
    for word in name
        .split(|c: char| !c.is_ascii_alphanumeric())
        .chain(qualified_name.split(|c: char| !c.is_ascii_alphanumeric()))
    {
        if !is_camel_case(word) {
            continue;
        }
        for part in split_compound(word) {
            if part.len() >= 2 && seen.insert(part.to_ascii_lowercase()) {
                out.push(part);
            }
        }
    }
    out.join(" ")
}

#[cfg(test)]
mod tests {
    use super::{is_camel_case, search_terms, split_compound};

    #[test]
    fn split_compound_handles_camel_snake_screaming() {
        assert_eq!(split_compound("getUserName"), vec!["get", "User", "Name"]);
        assert_eq!(
            split_compound("process_request"),
            vec!["process", "request"]
        );
        assert_eq!(split_compound("MAX_RETRIES"), vec!["MAX", "RETRIES"]);
        assert_eq!(split_compound("XMLParser"), vec!["XML", "Parser"]);
    }

    #[test]
    fn is_camel_case_classifies() {
        assert!(is_camel_case("UserService"));
        assert!(is_camel_case("processRequest"));
        assert!(!is_camel_case("user"));
        assert!(!is_camel_case("U"));
        assert!(!is_camel_case("process_request"));
    }

    #[test]
    fn search_terms_emits_camel_parts_only() {
        // Snake_case name: unicode61 already splits it — nothing to add.
        assert_eq!(
            search_terms(
                "rerank_candidates",
                "src/context/ranking.rs::rerank_candidates"
            ),
            ""
        );
        // CamelCase name and path segment produce inner words.
        let terms = search_terms(
            "updateCloudClient",
            "src/api/CloudSync.ts::updateCloudClient",
        );
        assert_eq!(terms, "update Cloud Client Sync");
    }

    #[test]
    fn search_terms_dedupes_case_insensitively() {
        let terms = search_terms("ParseJson", "parser/ParseJson.kt::ParseJson");
        assert_eq!(terms, "Parse Json");
    }
}
