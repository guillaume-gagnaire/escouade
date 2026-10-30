//! Steering the agents to the index (ESC-253): where a harness has hooks (the views of Claude Code
//! and Codex), a search that aims at a symbol (`grep`, `rg`, the Grep tool) gets, before it runs, a
//! line that says the index answers it in one call. **Never a refusal**: the hook adds context and
//! decides nothing, and a search for a text (a message, a phrase, a word of prose, a regex) goes by
//! with no word. Elsewhere the same advice is a rule in the agent's instructions (`rule`).
//!
//! What counts as aiming at a symbol, on purpose strict (a doubt is a text):
//! - a pattern that is one identifier with the mark of one (`parse_config`, `parseConfig`,
//!   `ParseConfig`: an underscore, or an upper case letter beyond the first), or `Name(`;
//! - or a declaration being looked for (`fn parse`, `class Shop`, `def run`, `struct Cart`…).
//!
//! A lower case word alone (`config`, `error`) is as likely prose: it is left alone.

use crate::i18n::Lang;
use crate::model::EngineId;
use serde_json::{json, Value};

/// The keywords a declaration begins with.
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
    "impl",
    "const",
    "let",
    "var",
    "pub fn",
    "async fn",
    "pub struct",
    "pub enum",
    "pub trait",
    "export function",
    "export class",
    "export const",
    "abstract class",
];

/// The programs that search the files for a pattern.
const SEARCHERS: &[&str] = &["grep", "egrep", "rg", "ag", "ack"];

/// The options of a search that take a value (the one after them is no pattern).
const WITH_VALUE: &[&str] = &[
    "-g",
    "--glob",
    "-t",
    "--type",
    "-T",
    "--type-not",
    "-A",
    "-B",
    "-C",
    "--context",
    "-m",
    "--max-count",
    "-f",
    "--file",
    "--include",
    "--exclude",
    "--exclude-dir",
    "-j",
    "--threads",
    "--type-add",
    "--max-depth",
    "-d",
    "--iglob",
];

/// The pattern a tool call of an agent searches for with: the Grep tool's `pattern`, or the first
/// pattern of a `grep`/`rg` in a shell command.
pub fn pattern_of(hook: &Value) -> Option<String> {
    let input = &hook["tool_input"];
    if let Some(p) = input["pattern"].as_str() {
        return Some(p.to_string());
    }
    let command = input["command"]
        .as_str()
        .or_else(|| input["cmd"].as_str())?;
    shell_pattern(command)
}

/// The pattern of the first search program a shell command runs.
fn shell_pattern(command: &str) -> Option<String> {
    let words = split_words(command);
    let mut i = 0;
    while i < words.len() {
        let w = words[i].rsplit(['/', '\\']).next().unwrap_or("");
        let w = w.strip_suffix(".exe").unwrap_or(w);
        let git_grep = w == "git" && words.get(i + 1).is_some_and(|n| n == "grep");
        if SEARCHERS.contains(&w) || git_grep {
            let mut j = i + if git_grep { 2 } else { 1 };
            while j < words.len() {
                let a = words[j].as_str();
                if matches!(a, "|" | "||" | "&&" | ";") {
                    return None;
                }
                if a == "-e" || a == "--regexp" {
                    return words.get(j + 1).cloned();
                }
                if let Some(rest) = a.strip_prefix("--regexp=") {
                    return Some(rest.to_string());
                }
                if WITH_VALUE.contains(&a) {
                    j += 2;
                    continue;
                }
                if a.starts_with('-') && a != "-" {
                    j += 1;
                    continue;
                }
                return Some(a.to_string());
            }
            return None;
        }
        i += 1;
    }
    None
}

/// The words of a command line, the quotes taken off; a separator is a word of its own.
fn split_words(s: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    let mut started = false;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => cur.push(c),
            None => match c {
                '\'' | '"' => {
                    quote = Some(c);
                    started = true;
                }
                c if c.is_whitespace() => {
                    if started || !cur.is_empty() {
                        out.push(std::mem::take(&mut cur));
                        started = false;
                    }
                }
                '|' | ';' | '&' => {
                    if started || !cur.is_empty() {
                        out.push(std::mem::take(&mut cur));
                        started = false;
                    }
                    let mut sep = c.to_string();
                    if chars.peek() == Some(&c) && c != ';' {
                        sep.push(c);
                        chars.next();
                    }
                    out.push(sep);
                }
                c => cur.push(c),
            },
        }
    }
    if started || !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Whether a search pattern aims at a symbol (see the module's doc).
pub fn aims_at_symbol(pattern: &str) -> bool {
    let p = pattern.trim();
    if p.len() < 3 || p.len() > 80 {
        return false;
    }
    // A declaration being looked for: `fn parse`, `class\s+Shop`.
    let flat = p
        .replace("\\s+", " ")
        .replace("\\s", " ")
        .replace("\\b", "");
    let flat = flat.trim();
    for kw in DECLARATIONS {
        if let Some(rest) = flat.strip_prefix(kw) {
            let rest = rest.trim_start_matches(' ');
            if rest.len() < flat.len() - kw.len()
                && is_identifier(rest.trim_end_matches(['(', '<', ' ']))
            {
                return true;
            }
        }
    }
    // `Name(` or `\bName\b`: one identifier with the mark of one.
    let bare = flat.trim_end_matches('(').trim_end_matches("\\(");
    is_identifier(bare) && has_identifier_mark(bare)
}

fn is_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    chars.next().is_some_and(|c| c.is_alphabetic() || c == '_')
        && s.chars().count() >= 3
        && s.chars().all(|c| c.is_alphanumeric() || c == '_')
}

/// An underscore, or a capital beyond the first letter in a word that has lower case ones (not
/// `TODO`): what prose does not have. `Shop` alone is a word of prose too, but a class's name more often: it counts
/// only after a keyword.
fn has_identifier_mark(s: &str) -> bool {
    s.contains('_')
        || (s.chars().skip(1).any(|c| c.is_uppercase()) && s.chars().any(|c| c.is_lowercase()))
}

/// What an agent of `engine` is told about a search for a symbol, as the hook's answer: only
/// harnesses that read it (Claude Code: an HTTP hook's answer; Codex: its command's output).
pub fn reply(engine: EngineId, lang: Lang, hook: &Value) -> Option<Value> {
    if !matches!(engine, EngineId::Claude | EngineId::Codex) {
        return None;
    }
    let event = hook["hook_event_name"].as_str().unwrap_or("");
    if event != "PreToolUse" {
        return None;
    }
    let pattern = pattern_of(hook)?;
    if !aims_at_symbol(&pattern) {
        return None;
    }
    Some(json!({ "hookSpecificOutput": {
        "hookEventName": "PreToolUse",
        "additionalContext": advice(lang),
    }}))
}

/// The line the hook adds to the context.
fn advice(lang: Lang) -> String {
    tr_in!(
        lang,
        "Escouade a un index du code de ce projet : l’outil MCP index_search (ou index_callers, index_context, index_body, index_impact) trouve en un appel un symbole, ses appelants (justes même quand le nom a des homonymes) et ce qu’un changement touche ; un texte exact se cherche avec index_search et literal. Garde grep pour une regex ou un texte.",
        "Escouade has a code index of this project: the MCP tool index_search (or index_callers, index_context, index_body, index_impact) finds in one call a symbol, its callers (right even when the name has namesakes) and what a change touches; an exact text is found by index_search with literal. Keep grep for a regex or a text."
    )
}

/// The rule added to the instructions of an agent whose harness has no hook for it (or whose view
/// has none): short, one line.
pub fn rule(lang: Lang) -> String {
    tr_in!(
        lang,
        "Index du code : pour trouver un symbole, ses appelants/appelés, le contexte d’une tâche, le corps d’une fonction ou l’impact d’un changement, utilise d’abord les outils MCP d’Escouade index_search, index_context, index_body, index_callers, index_callees et index_impact (un appel donne les appelants, justes même quand le nom a des homonymes), avec les mots du code ; garde grep pour une regex ou un texte. Si l’index est coupé, l’outil le dit.",
        "Code index: to find a symbol, its callers/callees, the context of a task, the body of a function or the impact of a change, first use Escouade's MCP tools index_search, index_context, index_body, index_callers, index_callees and index_impact (one call gives the callers, right even when the name has namesakes), with the words of the code; keep grep for a regex or a text. If the index is off, the tool says so."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bash(cmd: &str) -> Value {
        json!({ "hook_event_name": "PreToolUse", "tool_name": "Bash", "tool_input": { "command": cmd } })
    }

    #[test]
    fn a_symbol_is_an_identifier_with_its_mark_or_a_declaration() {
        for yes in [
            "parse_config",
            "parseConfig",
            "ParseConfig",
            "parseConfig(",
            "fn parse",
            "fn\\s+parse_all",
            "class Shop",
            "pub struct Cart",
            "def run",
            "export function loadAll",
        ] {
            assert!(aims_at_symbol(yes), "{yes}");
        }
        for no in [
            "config",
            "Shop",
            "error",
            "TODO",
            "failed to connect",
            "Cannot read property",
            "https://example.com",
            "foo.*bar",
            "\"parse_config\" is not",
            "ab",
            "fn ",
            "class of its own kind",
        ] {
            assert!(!aims_at_symbol(no), "{no}");
        }
    }

    #[test]
    fn the_pattern_of_a_shell_search_is_the_first_argument_that_is_no_option() {
        assert_eq!(
            shell_pattern("grep -rn parse_config src").as_deref(),
            Some("parse_config")
        );
        assert_eq!(
            shell_pattern("rg -g '*.rs' -i \"fn parse\" .").as_deref(),
            Some("fn parse")
        );
        assert_eq!(
            shell_pattern("cd src && rg -e parseConfig").as_deref(),
            Some("parseConfig")
        );
        assert_eq!(
            shell_pattern("git grep -n load_all").as_deref(),
            Some("load_all")
        );
        assert_eq!(
            shell_pattern("C:\\tools\\rg.exe load_all").as_deref(),
            Some("load_all")
        );
        assert_eq!(shell_pattern("ls -la | grep foo"), Some("foo".to_string()));
        assert_eq!(shell_pattern("cargo test"), None);
        assert_eq!(shell_pattern("grep"), None);
    }

    #[test]
    fn only_claude_and_codex_are_answered_and_only_for_a_symbol_search() {
        let grep = json!({ "hook_event_name": "PreToolUse", "tool_name": "Grep",
            "tool_input": { "pattern": "parse_config" } });
        for e in [EngineId::Claude, EngineId::Codex] {
            let r = reply(e, Lang::En, &grep).unwrap();
            assert_eq!(r["hookSpecificOutput"]["hookEventName"], "PreToolUse");
            assert!(r["hookSpecificOutput"]["additionalContext"]
                .as_str()
                .unwrap()
                .contains("index_search"));
            // Never a decision.
            assert!(r["hookSpecificOutput"].get("permissionDecision").is_none());
            assert!(r.get("decision").is_none());
        }
        assert!(reply(EngineId::Copilot, Lang::En, &grep).is_none());
        assert!(reply(EngineId::Cursor, Lang::En, &grep).is_none());
        assert!(reply(EngineId::Claude, Lang::En, &bash("rg parse_config src")).is_some());
        // A text search, another event, another tool: no word.
        assert!(reply(
            EngineId::Claude,
            Lang::En,
            &bash("rg 'failed to connect' src")
        )
        .is_none());
        assert!(reply(EngineId::Claude, Lang::En, &bash("npm test")).is_none());
        let mut post = grep.clone();
        post["hook_event_name"] = json!("PostToolUse");
        assert!(reply(EngineId::Claude, Lang::En, &post).is_none());
    }
}
