//! The code a text talks about, found by its words (ESC-310): what the comparison of the spec to the
//! code (`docscompare`) and the planner (`docsplan`) name to their agent, and the names a ticket's
//! files declare. The words of the code a text names (its code spans, its identifiers) are looked
//! for by `git grep` in the files git follows, so what `.gitignore` leaves out is never read; the
//! files the fence keeps out (those of a private project) are left out too. Each harness finds the
//! rest of the code by its own tools.

use std::collections::HashMap;
use std::path::Path;

/// The most words of a text looked for.
const MAX_TERMS: usize = 24;
/// The shortest word looked for.
const MIN_TERM: usize = 4;
/// A file read for the names it declares is no bigger than this.
const MAX_FILE: u64 = 1 << 20;

/// The keywords a declaration begins with: the name after one is declared.
const DECLARATIONS: &[&str] = &[
    "fn",
    "func",
    "function",
    "def",
    "class",
    "struct",
    "enum",
    "trait",
    "interface",
    "type",
    "const",
];

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// An identifier with its mark: an underscore, or a capital past the first letter in a word that
/// has lower case ones (`parse_config`, `parseConfig`, `ParseConfig`), what prose does not have.
fn has_identifier_mark(w: &str) -> bool {
    w.contains('_')
        || (w.chars().skip(1).any(char::is_uppercase) && w.chars().any(char::is_lowercase))
}

/// The words of `text` worth looking for in the code, in order and once each: its code spans
/// (`` `like_this` ``, a path, a name, a command: one word without spaces), then the words that
/// have the mark of an identifier. At most `MAX_TERMS`.
pub fn terms(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut add = |t: &str| {
        let t = t.trim_matches(|c: char| !is_word_char(c) && c != '/' && c != '.');
        let t = t.trim_end_matches(['.', '/']);
        if t.chars().count() >= MIN_TERM
            && t.chars().any(char::is_alphabetic)
            && !out.iter().any(|x| x == t)
            && out.len() < MAX_TERMS
        {
            out.push(t.to_string());
        }
    };
    let mut prose = String::new();
    for (i, part) in text.split('`').enumerate() {
        if i % 2 == 1 && !part.contains(char::is_whitespace) {
            add(part.trim_end_matches("()"));
        } else {
            prose.push_str(part);
            prose.push(' ');
        }
    }
    for w in prose.split(|c: char| !is_word_char(c)) {
        if has_identifier_mark(w) {
            add(w);
        }
    }
    out
}

/// Whether the file at `rel` is code to name: not the docs of Escouade (the spec itself holds the
/// words), nor a Markdown text.
fn is_code(rel: &str) -> bool {
    !rel.starts_with(".escouade/") && !rel.to_ascii_lowercase().ends_with(".md")
}

/// The files of `folder` that git follows which hold the most of `terms` (any case), the most
/// first, then by path; a term that is the path of one of them counts for more. None that `fenced`
/// covers (given the file's full path). At most `max`. Empty when `folder` is no git repository.
pub async fn related_files(
    folder: &Path,
    terms: &[String],
    fenced: impl Fn(&str) -> bool,
    max: usize,
) -> Vec<String> {
    let cwd = folder.to_string_lossy().to_string();
    let mut score: HashMap<String, usize> = HashMap::new();
    for t in terms {
        if t.contains('/') && folder.join(t).is_file() {
            *score.entry(t.clone()).or_default() += 100;
        }
        let args = [
            "-c",
            "core.quotepath=off",
            "grep",
            "-l",
            "-I",
            "-i",
            "-F",
            "-z",
            "-e",
            t.as_str(),
        ];
        // No match: git grep ends with 1, an error here.
        let Ok(found) = crate::git::text(&cwd, &args).await else {
            continue;
        };
        for f in found.split('\0').filter(|f| !f.is_empty()) {
            *score.entry(f.to_string()).or_default() += 1;
        }
    }
    let mut files: Vec<(String, usize)> = score
        .into_iter()
        .filter(|(f, _)| is_code(f) && !fenced(&folder.join(f).to_string_lossy()))
        .collect();
    files.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    files.into_iter().take(max).map(|(f, _)| f).collect()
}

/// The names `source` declares: the word after a keyword of a declaration (`fn send_receipt`,
/// `class Shop`, `export function loadAll`, `pub(crate) struct Cart`), in order and once each.
pub fn declared_names(source: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let words: Vec<&str> = source
        .split(|c: char| !is_word_char(c))
        .filter(|w| !w.is_empty())
        .collect();
    for pair in words.windows(2) {
        let (kw, name) = (pair[0], pair[1]);
        if DECLARATIONS.contains(&kw)
            && name
                .chars()
                .next()
                .is_some_and(|c| c.is_alphabetic() || c == '_')
            && !DECLARATIONS.contains(&name)
            && !out.iter().any(|x| x == name)
        {
            out.push(name.to_string());
        }
    }
    out
}

/// The names the files `changed` of `folder` declare (`declared_names`), those `fenced` covers and
/// the docs of Escouade left out, the files too big or not text skipped. At most `max`.
pub fn names_of_files(
    folder: &Path,
    changed: &[String],
    fenced: impl Fn(&str) -> bool,
    max: usize,
) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for rel in changed.iter().filter(|f| is_code(f)) {
        let path = folder.join(rel);
        if fenced(&path.to_string_lossy())
            || std::fs::metadata(&path).map_or(true, |m| !m.is_file() || m.len() > MAX_FILE)
        {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for n in declared_names(&text) {
            if out.len() >= max {
                return out;
            }
            if !out.contains(&n) {
                out.push(n);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn git(dir: &Path, args: &[&str]) {
        let ok = Command::new("git")
            .args([
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .current_dir(dir)
            .output()
            .unwrap()
            .status
            .success();
        assert!(ok, "git {args:?}");
    }

    fn write(dir: &Path, rel: &str, text: &str) {
        let p = dir.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }

    #[test]
    fn the_terms_are_the_code_spans_then_the_identifiers_of_the_prose() {
        let t = terms(
            "# Reçu\nLe reçu part par `send_receipt()` dans `src/pay.rs`, et `MailQueue` le garde. \
             Le `mode d'envoi` est libre ; parseConfig lit le_fichier, une Phrase TODO ordinaire, `ab`.",
        );
        assert_eq!(
            t,
            [
                "send_receipt",
                "src/pay.rs",
                "MailQueue",
                "parseConfig",
                "le_fichier"
            ]
        );
        assert!(terms("Une page sans un mot de code.").is_empty());
    }

    #[test]
    fn the_names_a_file_declares_follow_their_keyword() {
        let src = "pub(crate) fn send_receipt(x: u32) {}\nexport function loadAll() {}\n\
                   class Shop:\n  def run(self): pass\nstruct Cart;\nlet x = fn_like;\nfn send_receipt() {}";
        assert_eq!(
            declared_names(src),
            ["send_receipt", "loadAll", "Shop", "run", "Cart"]
        );
    }

    #[tokio::test]
    async fn the_files_git_follows_with_the_most_terms_come_first_and_never_a_fenced_or_ignored_one(
    ) {
        let dir = crate::paths::test_dir("codesearch-related");
        git(&dir, &["init", "-q"]);
        write(&dir, ".gitignore", "build/\n");
        write(&dir, "src/pay.rs", "pub fn send_receipt() {}\n");
        write(
            &dir,
            "src/mail.rs",
            "struct MailQueue;\nfn f() { send_receipt(); }\n",
        );
        write(&dir, "src/other.rs", "fn nothing() {}\n");
        write(&dir, "build/out.rs", "send_receipt MailQueue\n");
        write(&dir, "private/secret.rs", "send_receipt MailQueue\n");
        write(
            &dir,
            ".escouade/spec/pay/receipt.md",
            "`send_receipt` `MailQueue`\n",
        );
        write(&dir, "docs/notes.md", "send_receipt MailQueue\n");
        git(&dir, &["add", "-A"]);
        git(&dir, &["commit", "-qm", "init"]);
        write(&dir, "src/new.rs", "send_receipt MailQueue\n");

        let private = dir.join("private");
        let fence = crate::fence::Fence::new(vec![private]);
        let t = vec!["send_receipt".to_string(), "mailqueue".to_string()];
        let got = related_files(&dir, &t, |p| fence.covers(p), 10).await;
        assert_eq!(got, ["src/mail.rs", "src/pay.rs"]);
        // A path named first, the most kept.
        let t = vec!["src/other.rs".to_string(), "send_receipt".to_string()];
        assert_eq!(
            related_files(&dir, &t, |p| fence.covers(p), 2).await,
            ["src/other.rs", "src/mail.rs"]
        );
        // No repository: nothing.
        let bare = crate::paths::test_dir("codesearch-none");
        write(&bare, "a.rs", "send_receipt");
        assert!(related_files(&bare, &t, |_| false, 10).await.is_empty());
    }

    #[test]
    fn the_names_of_the_files_changed_leave_out_the_fenced_and_the_docs() {
        let dir = crate::paths::test_dir("codesearch-names");
        write(
            &dir,
            "src/pay.rs",
            "pub fn send_receipt() {}\nstruct Receipt;\n",
        );
        write(&dir, "private/x.rs", "fn hidden_name() {}\n");
        write(&dir, ".escouade/spec/a.md", "fn not_code() {}\n");
        let fence = crate::fence::Fence::new(vec![dir.join("private")]);
        let changed: Vec<String> = [
            "src/pay.rs",
            "private/x.rs",
            ".escouade/spec/a.md",
            "gone.rs",
        ]
        .map(String::from)
        .to_vec();
        assert_eq!(
            names_of_files(&dir, &changed, |p| fence.covers(p), 40),
            ["send_receipt", "Receipt"]
        );
    }
}
