//! The secrets a command typed in the clear may hold, masked before it is kept or shown (ESC-689): the value of a name
//! that says it is one (`token=`, `GITHUB_TOKEN=`, `--password`, `Authorization:`, `"apiKey": …`), the password of an
//! address (`https://user:pass@host`) or of `curl -u user:pass`, a key or a token of a known form (GitHub, GitLab,
//! OpenAI and Anthropic, Slack, AWS, Google, npm, Hugging Face, a JSON Web Token, a `Bearer`'s), a private key's block.
//! Masking too much is no harm: what is kept is only shown.

/// What stands in the place of a secret.
pub const MASK: &str = "***";

/// `text` with every secret it holds replaced by [`MASK`].
pub fn mask_secrets(text: &str) -> String {
    let mut spans = Vec::new();
    private_keys(text, &mut spans);
    addresses(text, &mut spans);
    named(text, &mut spans);
    tokens(text, &mut spans);
    if spans.is_empty() {
        return text.to_string();
    }
    spans.sort_unstable();
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    for (s, e) in spans {
        // Within, or across, the one before: one mask for both.
        if s < at {
            at = at.max(e);
            continue;
        }
        out.push_str(&text[at..s]);
        out.push_str(MASK);
        at = e;
    }
    out.push_str(&text[at..]);
    out
}

/// A character of a name or a word: letters, digits, `_` and `-`.
fn word_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'-'
}

/// A private key's block, whole.
fn private_keys(text: &str, spans: &mut Vec<(usize, usize)>) {
    let mut from = 0;
    while let Some(at) = text[from..].find("-----BEGIN").map(|i| from + i) {
        let head = &text[at..];
        let end = head
            .find("-----END")
            .and_then(|e| head[e + 8..].find("-----").map(|f| e + 8 + f + 5));
        match end {
            Some(end) if head[..end].contains("PRIVATE KEY") => {
                spans.push((at, at + end));
                from = at + end;
            }
            _ => from = at + 10,
        }
    }
}

/// The password of an address: `scheme://user:password@host`.
fn addresses(text: &str, spans: &mut Vec<(usize, usize)>) {
    for (at, _) in text.match_indices("://") {
        let start = at + 3;
        let rest = &text[start..];
        let end = rest
            .find(|c: char| c.is_whitespace() || "/\"'?#".contains(c))
            .unwrap_or(rest.len());
        let authority = &rest[..end];
        let Some(user_end) = authority.rfind('@') else {
            continue;
        };
        if let Some(colon) = authority[..user_end].find(':') {
            if colon + 1 < user_end {
                spans.push((start + colon + 1, start + user_end));
            }
        }
    }
}

/// A name that says its value is a secret: one of its parts (split at `-` and `_`) is a word of a secret, or it says
/// it is a key of an API, of an access, a private one.
fn secret_name(name: &str) -> bool {
    const PARTS: [&str; 16] = [
        "token",
        "tokens",
        "secret",
        "secrets",
        "password",
        "passwd",
        "pwd",
        "pass",
        "passphrase",
        "apikey",
        "auth",
        "authorization",
        "credential",
        "credentials",
        "cookie",
        "pat",
    ];
    let lower = name.trim_matches('-').to_ascii_lowercase();
    let parts: Vec<&str> = lower.split(['-', '_']).filter(|p| !p.is_empty()).collect();
    if parts.iter().any(|p| PARTS.contains(p)) {
        return true;
    }
    let joined = parts.join("_");
    [
        "api_key",
        "access_key",
        "private_key",
        "secret_key",
        "client_key",
        "session_key",
    ]
    .iter()
    .any(|k| joined.contains(k))
        || joined.ends_with("apikey")
        || joined.ends_with("accesskey")
}

/// The value that starts at `at`: inside its quotes when it is quoted, else up to a blank, a quote or what joins
/// another command or parameter (`&`, `;`, `|`, `,`, `)`).
fn value_at(text: &str, at: usize) -> Option<(usize, usize)> {
    let b = text.as_bytes();
    let first = *b.get(at)?;
    if first == b'"' || first == b'\'' {
        let inner = at + 1;
        let end = text[inner..]
            .find(first as char)
            .map_or(text.len(), |e| inner + e);
        return (end > inner).then_some((inner, end));
    }
    let end = text[at..]
        .find(|c: char| c.is_whitespace() || "\"'&;|,)".contains(c))
        .map_or(text.len(), |e| at + e);
    (end > at).then_some((at, end))
}

/// The first character after `at` that is no blank.
fn skip_blanks(text: &str, mut at: usize) -> usize {
    let b = text.as_bytes();
    while at < b.len() && (b[at] == b' ' || b[at] == b'\t') {
        at += 1;
    }
    at
}

/// The values of the names that say they are secrets: `name=value`, `name: value` (a header, with its scheme kept),
/// `"name": "value"`, `--name value` and `--name=value`; and the password of `-u user:password`.
fn named(text: &str, spans: &mut Vec<(usize, usize)>) {
    let b = text.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if !word_char(b[i]) || (i > 0 && word_char(b[i - 1])) {
            i += 1;
            continue;
        }
        let start = i;
        while i < b.len() && word_char(b[i]) {
            i += 1;
        }
        let word = &text[start..i];
        let flag = word.starts_with('-');
        // `-u user:password` and `--user user:password` (curl): the user stays.
        if word == "-u" || word == "--user" {
            let at = if b.get(i) == Some(&b'=') {
                i + 1
            } else {
                skip_blanks(text, i)
            };
            if let Some((s, e)) = value_at(text, at) {
                if let Some(colon) = text[s..e].find(':') {
                    if s + colon + 1 < e {
                        spans.push((s + colon + 1, e));
                    }
                }
            }
            continue;
        }
        if !secret_name(word) {
            continue;
        }
        if flag {
            let at = match b.get(i) {
                Some(b'=') => i + 1,
                Some(b' ') | Some(b'\t') => {
                    let at = skip_blanks(text, i);
                    // A flag after it: this one takes no value.
                    if b.get(at) == Some(&b'-') {
                        continue;
                    }
                    at
                }
                _ => continue,
            };
            if let Some(span) = value_at(text, at) {
                spans.push(span);
            }
            continue;
        }
        // The quote that closes a quoted name, then `=` or `:`.
        let mut at = i;
        if matches!(b.get(at), Some(b'"') | Some(b'\'')) {
            at += 1;
        }
        let sep = b.get(at).copied();
        if sep != Some(b'=') && sep != Some(b':') {
            continue;
        }
        // An address's scheme (`token://`) or a path's drive is no value.
        if sep == Some(b':') && b.get(at + 1) == Some(&b'/') {
            continue;
        }
        at = skip_blanks(text, at + 1);
        // A header's scheme is kept: `Authorization: Bearer <masked>`.
        let scheme_end = text[at..]
            .find(|c: char| !c.is_ascii_alphabetic())
            .map_or(text.len(), |e| at + e);
        let scheme = text[at..scheme_end].to_ascii_lowercase();
        if ["bearer", "basic", "token", "digest", "apikey"].contains(&scheme.as_str())
            && matches!(b.get(scheme_end), Some(b' ') | Some(b'\t'))
        {
            at = skip_blanks(text, scheme_end);
        }
        if let Some(span) = value_at(text, at) {
            spans.push(span);
        }
    }
}

/// The keys and tokens of a known form, wherever they are; and the word after `Bearer`.
fn tokens(text: &str, spans: &mut Vec<(usize, usize)>) {
    let b = text.as_bytes();
    let mut i = 0;
    let mut bearer = false;
    while i < b.len() {
        if !word_char(b[i]) {
            i += 1;
            continue;
        }
        let start = i;
        let jwt = text[start..].starts_with("eyJ");
        while i < b.len() && (word_char(b[i]) || (jwt && b[i] == b'.')) {
            i += 1;
        }
        let w = text[start..i].trim_end_matches('.');
        if known_token(w) || (bearer && w.len() >= 16) {
            spans.push((start, start + w.len()));
        }
        bearer = w.eq_ignore_ascii_case("bearer");
    }
}

/// A key or a token of a known form.
fn known_token(w: &str) -> bool {
    let body = |prefix: &str, min: usize| {
        w.strip_prefix(prefix)
            .is_some_and(|rest| rest.len() >= min && rest.bytes().all(word_char))
    };
    let parts: Vec<&str> = w.split('.').collect();
    if parts.len() == 3 && w.starts_with("eyJ") && parts.iter().all(|p| p.len() >= 8) {
        return true;
    }
    body("ghp_", 30)
        || body("gho_", 30)
        || body("ghs_", 30)
        || body("ghu_", 30)
        || body("ghr_", 30)
        || body("github_pat_", 30)
        || body("glpat-", 18)
        || body("sk-", 20)
        || body("xoxb-", 10)
        || body("xoxp-", 10)
        || body("xoxa-", 10)
        || body("xoxs-", 10)
        || body("xapp-", 10)
        || body("npm_", 30)
        || body("hf_", 30)
        || body("AIza", 30)
        || ((w.starts_with("AKIA") || w.starts_with("ASIA"))
            && w.len() == 20
            && w.bytes()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn masks(text: &str, want: &str) {
        assert_eq!(mask_secrets(text), want, "{text}");
    }

    #[test]
    fn masks_the_values_of_names_that_say_they_are_secrets() {
        masks(
            "curl https://x/?token=abc123&page=2",
            "curl https://x/?token=***&page=2",
        );
        masks(
            "GITHUB_TOKEN=ghx123 gh pr list",
            "GITHUB_TOKEN=*** gh pr list",
        );
        masks(
            "export OPENAI_API_KEY='a b c'",
            "export OPENAI_API_KEY='***'",
        );
        masks("psql --password hunter2 -d db", "psql --password *** -d db");
        masks(
            "tool --api-key=abc --verbose",
            "tool --api-key=*** --verbose",
        );
        masks("login --token --force", "login --token --force");
        masks(
            r#"echo '{"password": "hunter2"}'"#,
            r#"echo '{"password": "***"}'"#,
        );
        masks(
            r#"curl -H "Authorization: Bearer abc.def" -H 'X-Api-Key: k1' x"#,
            r#"curl -H "Authorization: Bearer ***" -H 'X-Api-Key: ***' x"#,
        );
        masks(
            "curl -H \"Private-Token: glx\" x",
            "curl -H \"Private-Token: ***\" x",
        );
    }

    #[test]
    fn masks_the_password_of_an_address_or_of_curl_u() {
        masks(
            "git clone https://oauth2:s3cr3t@gitlab.com/a/b.git",
            "git clone https://oauth2:***@gitlab.com/a/b.git",
        );
        masks(
            "curl -u admin:hunter2 https://x",
            "curl -u admin:*** https://x",
        );
        masks("curl -u admin https://x", "curl -u admin https://x");
    }

    #[test]
    fn masks_keys_and_tokens_of_a_known_form() {
        let gh = format!("ghp_{}", "a".repeat(36));
        masks(
            &format!("echo {gh} | gh auth login --with-token"),
            "echo *** | gh auth login --with-token",
        );
        let ant = format!("sk-ant-api03-{}", "x".repeat(40));
        masks(&format!("claude --key {ant}"), "claude --key ***");
        masks("aws s3 ls # AKIAIOSFODNN7EXAMPLE", "aws s3 ls # ***");
        let jwt = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N";
        masks(&format!("http GET x 'Auth:{jwt}'"), "http GET x 'Auth:***'");
        masks(&format!("echo {jwt}."), "echo ***.");
        masks(
            "wget --header=x Bearer abcdefghijklmnopqrstu",
            "wget --header=x Bearer ***",
        );
        masks(
            "cat <<EOF\n-----BEGIN RSA PRIVATE KEY-----\nMIIE\n-----END RSA PRIVATE KEY-----\nEOF",
            "cat <<EOF\n***\nEOF",
        );
    }

    #[test]
    fn leaves_an_ordinary_command_as_it_is() {
        for text in [
            "cargo test -p escouade-plan",
            "git log --author=guillaume --oneline",
            "npm run test:watch",
            "git -c credential.helper= fetch",
            "ls C:\\Users\\é",
            "curl https://example.com/path?page=2",
            "echo ghp_short",
        ] {
            masks(text, text);
        }
    }
}
