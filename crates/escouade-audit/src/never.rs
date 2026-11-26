//! What never goes out (`docs/TEAM-AUDIT.md` §2, A3-B, A8-A), written here and nowhere else: neither a team nor a
//! setting changes it.
//!
//! How it is held:
//!
//! - every event is a closed structure (`deny_unknown_fields`), whose strings are typed words of a fixed grammar
//!   (`words.rs`: ids, ticket keys, setting keys, hashes…) that refuse a space, a path separator or a token;
//! - two kinds of strings only are free: the names (of members, devices, projects, budgets: A3-B) and the reasons an
//!   administrator writes (a kill switch, a setting, a revocation). They are scrubbed here ([`scrub`]): a token, a path
//!   or a variable given a value (`TOKEN=…`) is replaced by a marker the emitter counts ([`Sealed::redacted`]);
//! - a command an agent is allowed to run goes out as its program and its fingerprint only ([`crate::CommandPrint`]);
//! - the test of the trapped data (`tests` of `event.rs`) puts secrets, titles, texts, paths and a whole command in every
//!   slot of every event and finds none in what goes out, in the three formats.
//!
//! [`Sealed::redacted`]: crate::Sealed::redacted

/// What never goes out, as the crate's README and `docs/TEAM-AUDIT.md` §2 say it: the list is fixed in the code.
pub const NEVER: &[&str] = &[
    "the encrypted content: a ticket's title, description and criteria, spec pages, decisions, documents and their names, a memory's text, a project's name and logo beyond its short name",
    "the secrets: session, enrolment and plugin tokens, device, project, team, audit and cost keys, the 24 recovery words, the escrow seed, the secret after the # of an invitation, the short code of a pairing",
    "the work: the text of conversations, prompts, answers, command outputs, diffs and commit messages, file names, code",
    "the harnesses' accounts: neither the account, nor its e-mail, nor its plan",
    "the whole text of a command an agent is allowed to run: its program and its fingerprint only",
];

/// The marker a secret is replaced by in a free string (§2).
pub const SECRET_MARK: &str = "[retiré : ressemble à un secret]";
/// The marker a path is replaced by in a free string.
pub const PATH_MARK: &str = "[retiré : ressemble à un chemin]";

/// The marker an e-mail address is replaced by in a free string.
pub const ADDRESS_MARK: &str = "[retiré : ressemble à une adresse]";

/// What a secret found is, said by the caller in its own language (the team memory's refusal: `src-tauri/src/memory.rs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecretKind {
    /// A GitHub token (`ghp_`, `gho_`, `ghu_`, `ghs_`, `ghr_`, `github_pat_`).
    GitHub,
    /// An OpenAI or Anthropic key (`sk-`).
    Api,
    /// A GitLab token (`glpat-`).
    GitLab,
    /// An AWS access key (`AKIA`).
    Aws,
    /// A Slack token (`xox?-`).
    Slack,
    /// A bearer token.
    Bearer,
    /// A private key in PEM.
    PrivateKey,
    /// The address of a Slack or Discord webhook.
    Webhook,
}

/// Whether `c` may stand inside a token (letters, digits, `_`), so that a known prefix inside a word is not one.
fn wordy(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// How many characters of `rest` from its start are in `ok`.
fn run_of(rest: &str, ok: impl Fn(char) -> bool) -> usize {
    rest.chars().take_while(|c| ok(*c)).count()
}

/// The first secret `text` holds: the motifs of known tokens the team memory refuses (TS-10) and the security audit
/// reads its report for (`docs/security/audit-2026-11/build.mjs`, `leaks`): GitHub, OpenAI and Anthropic keys (`sk-`),
/// Slack, GitLab, AWS, a bearer token, a private key, a webhook of Slack or Discord. None when it holds none.
pub fn secret_kind(text: &str) -> Option<SecretKind> {
    let alnum = |c: char| c.is_ascii_alphanumeric();
    let token = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-';
    // (prefix, the characters after it, how many at least, what it is).
    type Rule = (&'static str, fn(char) -> bool, usize, SecretKind);
    let rules: [Rule; 9] = [
        (
            "ghp_",
            |c| c.is_ascii_alphanumeric(),
            20,
            SecretKind::GitHub,
        ),
        (
            "gho_",
            |c| c.is_ascii_alphanumeric(),
            20,
            SecretKind::GitHub,
        ),
        (
            "ghu_",
            |c| c.is_ascii_alphanumeric(),
            20,
            SecretKind::GitHub,
        ),
        (
            "ghs_",
            |c| c.is_ascii_alphanumeric(),
            20,
            SecretKind::GitHub,
        ),
        (
            "ghr_",
            |c| c.is_ascii_alphanumeric(),
            20,
            SecretKind::GitHub,
        ),
        (
            "github_pat_",
            |c| c.is_ascii_alphanumeric() || c == '_',
            20,
            SecretKind::GitHub,
        ),
        (
            "sk-",
            |c| c.is_ascii_alphanumeric() || c == '_' || c == '-',
            20,
            SecretKind::Api,
        ),
        (
            "glpat-",
            |c| c.is_ascii_alphanumeric() || c == '_' || c == '-',
            20,
            SecretKind::GitLab,
        ),
        (
            "AKIA",
            |c| c.is_ascii_digit() || c.is_ascii_uppercase(),
            16,
            SecretKind::Aws,
        ),
    ];
    for (at, _) in text.char_indices() {
        let rest = &text[at..];
        let starts = text[..at].chars().next_back().is_none_or(|c| !wordy(c));
        if !starts {
            continue;
        }
        for (prefix, ok, min, kind) in &rules {
            if let Some(after) = rest.strip_prefix(prefix) {
                if run_of(after, ok) >= *min {
                    return Some(*kind);
                }
            }
        }
        for slack in ["xoxa-", "xoxb-", "xoxp-", "xoxr-", "xoxs-"] {
            if let Some(after) = rest.strip_prefix(slack) {
                if run_of(after, |c| alnum(c) || c == '-') >= 10 {
                    return Some(SecretKind::Slack);
                }
            }
        }
        if let Some(after) = rest.strip_prefix("Bearer") {
            let spaces = run_of(after, char::is_whitespace);
            if spaces > 0 && run_of(&after[spaces..], |c| token(c) || c == '.' || c == '=') >= 24 {
                return Some(SecretKind::Bearer);
            }
        }
    }
    if let Some(i) = text.find("-----BEGIN ") {
        let rest = &text[i + "-----BEGIN ".len()..];
        let label = run_of(rest, |c| c.is_ascii_uppercase() || c == ' ');
        if rest[..label].ends_with("PRIVATE KEY") && rest[label..].starts_with("-----") {
            return Some(SecretKind::PrivateKey);
        }
    }
    for hook in [
        "https://hooks.slack.com/services/",
        "https://discord.com/api/webhooks/",
        "https://discordapp.com/api/webhooks/",
        "https://ptb.discord.com/api/webhooks/",
        "https://canary.discord.com/api/webhooks/",
    ] {
        if let Some(i) = text.find(hook) {
            if text[i + hook.len()..]
                .chars()
                .next()
                .is_some_and(|c| !c.is_whitespace())
            {
                return Some(SecretKind::Webhook);
            }
        }
    }
    None
}

/// Whether a word of a free string looks like a path: a separator `\`, a start in `/`, `~/`, `./`, `../` or a drive
/// (`C:`), or two `/` (an address, a relative path).
fn pathy(word: &str) -> bool {
    let w = word
        .trim_matches(|c: char| matches!(c, '"' | '\'' | '`' | '(' | ')' | '[' | ']' | ',' | ';'));
    let drive = {
        let mut c = w.chars();
        matches!((c.next(), c.next()), (Some(l), Some(':')) if l.is_ascii_alphabetic())
            && w.len() > 2
    };
    w.contains('\\')
        || w.starts_with('/')
        || w.starts_with("~/")
        || w.starts_with("./")
        || w.starts_with("../")
        || drive
        || w.matches('/').count() >= 2
}

/// Whether a word looks like an e-mail address (`claire@acme.fr`): a harness's account never goes out (§2).
pub fn email_like(word: &str) -> bool {
    let w =
        word.trim_matches(|c: char| matches!(c, '<' | '>' | '"' | '\'' | '(' | ')' | ',' | ';'));
    match w.split_once('@') {
        Some((local, domain)) => {
            !local.is_empty()
                && domain.rsplit_once('.').is_some_and(|(host, tld)| {
                    !host.is_empty()
                        && tld.len() >= 2
                        && tld.chars().all(|c| c.is_ascii_alphabetic())
                })
        }
        None => false,
    }
}

/// Whether a word gives a variable a value (`TOKEN=…`, `export KEY=…`): the way a secret rides in a command.
fn assignment(word: &str) -> bool {
    match word.split_once('=') {
        Some((name, _)) => {
            !name.is_empty()
                && name
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        }
        None => false,
    }
}

/// Whether a word looks like a key nobody knows the motif of: 32 characters or more of a token's alphabet, with
/// letters and digits mixed.
fn random_looking(word: &str) -> bool {
    let w = word.trim_matches(|c: char| !c.is_ascii_alphanumeric());
    w.chars().count() >= 32
        && w.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '+' | '/' | '=' | '.'))
        && w.chars().any(|c| c.is_ascii_digit())
        && w.chars().any(|c| c.is_ascii_alphabetic())
}

/// A free string (a name, a reason) as it may go out: each word that is a token, a path or a variable's value replaced
/// by its marker; the whole string replaced when a secret spans words (a bearer token, a private key). Control
/// characters become spaces.
pub fn scrub(text: &str) -> String {
    let flat: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let mut out = String::with_capacity(flat.len());
    for (i, word) in flat.split(' ').enumerate() {
        if i > 0 {
            out.push(' ');
        }
        if word.is_empty() {
            out.push_str(word);
        } else if secret_kind(word).is_some() || assignment(word) || random_looking(word) {
            out.push_str(SECRET_MARK);
        } else if email_like(word) {
            out.push_str(ADDRESS_MARK);
        } else if pathy(word) {
            out.push_str(PATH_MARK);
        } else {
            out.push_str(word);
        }
    }
    if secret_kind(&out).is_some() {
        return SECRET_MARK.to_owned();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_motifs_of_the_team_memory_find_each_kind_of_token() {
        let found = [
            ("ghp_abcdefghijklmnopqrstuvwxyz0123", SecretKind::GitHub),
            ("github_pat_11ABCDEFG0123456789_abcdef", SecretKind::GitHub),
            ("sk-ant-api03-abcdefghijklmnopqrstuv", SecretKind::Api),
            ("glpat-abcdefghijklmnopqrst", SecretKind::GitLab),
            ("AKIAABCDEFGHIJKLMNOP", SecretKind::Aws),
            ("xoxb-1234567890-abcdef", SecretKind::Slack),
            (
                "Authorization: Bearer abcdefghijklmnopqrstuvwxyz0123",
                SecretKind::Bearer,
            ),
            (
                "-----BEGIN OPENSSH PRIVATE KEY-----",
                SecretKind::PrivateKey,
            ),
            (
                "https://hooks.slack.com/services/T000/B000/XXX",
                SecretKind::Webhook,
            ),
        ];
        for (text, kind) in found {
            assert_eq!(secret_kind(text), Some(kind), "{text}");
        }
        for text in [
            "task-1234",
            "ask-me-anything-about-it-now",
            "the ghp_ prefix",
            "AKIA is AWS",
        ] {
            assert_eq!(secret_kind(text), None, "{text}");
        }
    }

    #[test]
    fn a_free_string_loses_its_tokens_paths_and_values_and_keeps_its_words() {
        assert_eq!(scrub("Arrêt du fournisseur X"), "Arrêt du fournisseur X");
        assert_eq!(
            scrub("fuite de ghp_abcdefghijklmnopqrstuvwxyz0123 hier"),
            format!("fuite de {SECRET_MARK} hier")
        );
        assert_eq!(
            scrub("voir C:\\Users\\claire\\plan.docx et /home/claire/x.md"),
            format!("voir {PATH_MARK} et {PATH_MARK}")
        );
        assert_eq!(
            scrub("TOKEN=abc npm publish"),
            format!("{SECRET_MARK} npm publish")
        );
        assert_eq!(scrub("Bearer abcdefghijklmnopqrstuvwxyz0123"), SECRET_MARK);
        assert_eq!(scrub("une\nligne"), "une ligne");
        assert_eq!(scrub(&scrub("TOKEN=abc /etc/x")), scrub("TOKEN=abc /etc/x"));
        assert_eq!(
            scrub("cle 9f8e7d6c5b4a39281706f5e4d3c2b1a0ffeeddcc"),
            format!("cle {SECRET_MARK}")
        );
    }
}
