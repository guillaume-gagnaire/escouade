//! Search through the agents' conversations (Ctrl+K): their logs read back from disk, case and
//! accents ignored, at most `MAX_HITS` results and `TIME_LIMIT` of work.

use crate::{conv, paths};
use serde::Serialize;
use serde_json::Value;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// The results a search gives at most.
pub const MAX_HITS: usize = 200;
/// How long a search reads before it gives what it found.
pub const TIME_LIMIT: Duration = Duration::from_secs(5);
/// Characters of the snippet before the match, a tool's label and the `…` included: the palette
/// shows some 85 characters of a snippet on its line, the match must be among them.
const LEAD: usize = 28;
/// Characters of a tool's label shown (an MCP tool's can be some 50).
const LABEL_MAX: usize = 16;
/// Characters of the snippet in all (the last `…` aside): the line cuts what it cannot show.
const SNIPPET: usize = 120;
/// How much of each value of an unknown tool's input is searched.
const INPUT_MAX: usize = 500;

/// An agent whose conversation may be searched.
pub struct AgentRef {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub archived: bool,
    /// Its folder: the files its tools name are shown from there.
    pub cwd: String,
    pub last_activity: i64,
    pub created_at: i64,
}

pub struct Query<'a> {
    pub text: &'a str,
    /// One project's agents only.
    pub project_id: Option<&'a str>,
    /// Archived agents too.
    pub archived: bool,
}

/// A message of a conversation that matches.
#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Hit {
    pub agent_id: String,
    pub project_id: String,
    pub agent_name: String,
    pub archived: bool,
    /// Its rank among the conversation's items.
    pub event_index: usize,
    pub item_id: String,
    /// The text around the match, on one line.
    pub snippet: String,
    /// Where the match is in `snippet`, in UTF-16 units (the window's string indices).
    pub mark: [usize; 2],
    /// When it was written (epoch ms).
    pub at: i64,
}

#[derive(Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Found {
    /// Agent by agent, the most recently active first; in each, the newest messages first.
    pub hits: Vec<Hit>,
    /// More messages match than `MAX_HITS`.
    pub capped: bool,
    /// Stopped at `TIME_LIMIT`, before the end of the conversations.
    pub timed_out: bool,
}

/// The searches the window started: it only reads the answer to its last one.
#[derive(Default)]
pub struct Searches(AtomicU64);

impl Searches {
    /// Starts a search. The next one makes it pointless, which the returned function tells.
    pub fn start(&self) -> impl Fn() -> bool + '_ {
        let me = self.0.fetch_add(1, Ordering::Relaxed) + 1;
        move || self.0.load(Ordering::Relaxed) != me
    }
}

/// Searches the conversations of `agents` (their logs in `dir`) for `q`, until `deadline` or
/// until `stale` says nobody waits for the answer any more.
pub fn search(
    dir: &Path,
    agents: &[AgentRef],
    q: &Query,
    deadline: Instant,
    stale: &dyn Fn() -> bool,
) -> Found {
    let mut found = Found::default();
    let needle = fold(q.text);
    let needle = needle.trim();
    if needle.is_empty() {
        return found;
    }
    let mut agents: Vec<&AgentRef> = agents
        .iter()
        .filter(|a| q.project_id.is_none_or(|p| p == a.project_id) && (q.archived || !a.archived))
        .collect();
    agents.sort_by_key(|a| std::cmp::Reverse(a.last_activity));
    for a in agents {
        let Some(log) = conv::replay(&conv::log_path(dir, &a.id)) else {
            continue;
        };
        let at = written_at(&log.items, a.created_at);
        for (i, item) in log.items.iter().enumerate().rev() {
            if stale() {
                return found;
            }
            if Instant::now() >= deadline {
                found.timed_out = true;
                return found;
            }
            let Some(text) = searchable(item, &a.cwd) else {
                continue;
            };
            let Some((start, end)) = locate(&text.body, needle) else {
                continue;
            };
            if found.hits.len() == MAX_HITS {
                found.capped = true;
                return found;
            }
            let (snippet, mark) = snippet(&text.label, &text.body, start, end);
            found.hits.push(Hit {
                agent_id: a.id.clone(),
                project_id: a.project_id.clone(),
                agent_name: a.name.clone(),
                archived: a.archived,
                event_index: i,
                item_id: item["id"].as_str().unwrap_or_default().to_string(),
                snippet,
                mark,
                at: at[i],
            });
        }
    }
    found
}

/// When each item was written: its own time, else that of the last item before it that has one
/// (Claude's text has none), else the agent's creation.
fn written_at(items: &[Value], created_at: i64) -> Vec<i64> {
    let mut last = created_at;
    items
        .iter()
        .map(|item| {
            if let Some(ts) = item["ts"].as_i64() {
                last = ts;
            }
            last
        })
        .collect()
}

/// What of an item is searched: `body`, shown after `label` when it has one (the tool).
struct Searchable {
    label: String,
    body: String,
}

/// The text of an item as the conversation shows it: the messages of the user and of Claude, the
/// gist of a tool's call (its command, its file…) and its output, a question and its answer, a
/// plan, a notice, a turn's error. Not the thinking, folded away in the conversation, nor the
/// questions' and plans' own calls, shown by their cards.
fn searchable(item: &Value, cwd: &str) -> Option<Searchable> {
    let s = |v: &Value| v.as_str().unwrap_or_default().to_string();
    let mut label = String::new();
    let mut parts: Vec<String> = Vec::new();
    match item["kind"].as_str()? {
        "user" => {
            parts.push(s(&item["text"]));
            if let Some(files) = item["files"].as_array() {
                parts.extend(files.iter().map(s));
            }
        }
        "text" | "event" | "notice" => parts.push(s(&item["text"])),
        "question" => {
            for q in item["questions"].as_array().into_iter().flatten() {
                parts.push(s(&q["question"]));
                for o in q["options"].as_array().into_iter().flatten() {
                    parts.push(s(&o["label"]));
                    parts.push(s(&o["description"]));
                }
            }
            if let Some(answers) = item["answers"].as_object() {
                parts.extend(answers.values().map(s));
            }
        }
        "permission" => {
            let name = s(&item["toolName"]);
            parts.push(s(&item["title"]));
            parts.push(s(&item["description"]));
            parts.push(tool_gist(&name, &item["input"], cwd));
            parts.push(s(&item["message"]));
        }
        "tool" => {
            let name = s(&item["name"]);
            if name == "AskUserQuestion" || name == "ExitPlanMode" {
                return None;
            }
            label = tool_label(&name);
            parts.push(tool_gist(&name, &item["input"], cwd));
            parts.push(s(&item["result"]["text"]));
        }
        "turn" => parts.push(s(&item["error"])),
        _ => return None,
    }
    parts.retain(|p| !p.trim().is_empty());
    (!parts.is_empty()).then(|| Searchable {
        label,
        body: parts.join("\n"),
    })
}

/// A tool as its row names it.
fn tool_label(name: &str) -> String {
    if let Some(rest) = name.strip_prefix("mcp__") {
        let (server, tool) = rest.split_once("__").unwrap_or((rest, ""));
        return format!("{server}·{tool}");
    }
    match name {
        "Task" | "Agent" => "Agent".to_string(),
        _ => name.to_string(),
    }
}

/// The gist of a tool's call, as its row shows it (lib/tools.ts, `toolArg`): what an edit writes
/// is the file's content, not the call's.
fn tool_gist(name: &str, input: &Value, cwd: &str) -> String {
    let s = |k: &str| input[k].as_str().unwrap_or_default().to_string();
    let file = |k: &str| {
        let p = s(k);
        paths::strip_base(cwd, &p).unwrap_or(p)
    };
    match name {
        "Bash" | "PowerShell" => s("command"),
        "Read" | "Edit" | "Write" | "MultiEdit" => file("file_path"),
        "NotebookEdit" => file("notebook_path"),
        "Grep" | "Glob" => format!("{}  {}", s("pattern"), file("path")),
        "WebFetch" => s("url"),
        "WebSearch" => s("query"),
        "Task" | "Agent" => s("description"),
        "TodoWrite" => input["todos"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|t| t["content"].as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        "Skill" | "SlashCommand" => {
            let skill = s("skill");
            if skill.is_empty() {
                s("command")
            } else {
                skill
            }
        }
        "ExitPlanMode" => s("plan"),
        _ => input
            .as_object()
            .into_iter()
            .flatten()
            .filter_map(|(_, v)| v.as_str())
            .map(|v| v.chars().take(INPUT_MAX).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

/// `text` as it is searched: lowercase, without accents (decomposed ones included), the `œ`, `æ`
/// and `ß` written out, `\` as `/` (a path is found with either), a typographic apostrophe as the
/// one typed, each run of spaces or line breaks as one space. With `from`, the byte of `text` each
/// byte of the result comes from.
fn fold_into(text: &str, out: &mut String, mut from: Option<&mut Vec<usize>>) {
    for (i, c) in text.char_indices() {
        for l in c.to_lowercase() {
            let folded: &str = match l {
                c if c.is_whitespace() => {
                    if out.ends_with(' ') {
                        continue;
                    }
                    " "
                }
                '\u{300}'..='\u{36f}' => continue,
                '\\' => "/",
                '’' | '‘' | 'ʼ' => "'",
                'œ' => "oe",
                'æ' => "ae",
                'ß' => "ss",
                'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' => "a",
                'ç' | 'ć' | 'ĉ' | 'ċ' | 'č' => "c",
                'ď' | 'đ' => "d",
                'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ĕ' | 'ė' | 'ę' | 'ě' => "e",
                'ĝ' | 'ğ' | 'ġ' | 'ģ' => "g",
                'ĥ' | 'ħ' => "h",
                'ì' | 'í' | 'î' | 'ï' | 'ĩ' | 'ī' | 'ĭ' | 'į' | 'ı' => "i",
                'ĵ' => "j",
                'ķ' => "k",
                'ĺ' | 'ļ' | 'ľ' | 'ŀ' | 'ł' => "l",
                'ñ' | 'ń' | 'ņ' | 'ň' => "n",
                'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ŏ' | 'ő' => "o",
                'ŕ' | 'ŗ' | 'ř' => "r",
                'ś' | 'ŝ' | 'ş' | 'š' => "s",
                'ţ' | 'ť' | 'ŧ' => "t",
                'ù' | 'ú' | 'û' | 'ü' | 'ũ' | 'ū' | 'ŭ' | 'ů' | 'ű' | 'ų' => "u",
                'ŵ' => "w",
                'ý' | 'ÿ' | 'ŷ' => "y",
                'ź' | 'ż' | 'ž' => "z",
                c => {
                    let mut buf = [0u8; 4];
                    let one = c.encode_utf8(&mut buf);
                    out.push_str(one);
                    if let Some(from) = from.as_deref_mut() {
                        from.extend(std::iter::repeat_n(i, one.len()));
                    }
                    continue;
                }
            };
            out.push_str(folded);
            if let Some(from) = from.as_deref_mut() {
                from.extend(std::iter::repeat_n(i, folded.len()));
            }
        }
    }
}

fn fold(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    fold_into(text, &mut out, None);
    out
}

/// Where `needle` (folded) is in `text`, in bytes of `text`: from the first character it covers to
/// the end of the last one, with the accents that follow it when they are written apart.
fn locate(text: &str, needle: &str) -> Option<(usize, usize)> {
    // Most items do not match: their text is folded once, without keeping where it comes from.
    if !fold(text).contains(needle) {
        return None;
    }
    let mut folded = String::with_capacity(text.len());
    let mut from = Vec::with_capacity(text.len());
    fold_into(text, &mut folded, Some(&mut from));
    let at = folded.find(needle)?;
    let start = from[at];
    let last = from[at + needle.len() - 1];
    let mut end = last + text[last..].chars().next().map_or(0, char::len_utf8);
    for c in text[end..].chars() {
        if !('\u{300}'..='\u{36f}').contains(&c) {
            break;
        }
        end += c.len_utf8();
    }
    Some((start, end))
}

/// The text around the match (`start..end`, in bytes), on one line, after `label` (" · ", cut
/// at `LABEL_MAX`) when there is one: what `LEAD` leaves of the text before the match, then what
/// the snippet holds after it, `…` where it is cut. With where the match is in it, in UTF-16 units.
fn snippet(label: &str, text: &str, start: usize, end: usize) -> (String, [usize; 2]) {
    let mut out = String::new();
    if !label.is_empty() {
        let mut chars = label.chars();
        out.extend(chars.by_ref().take(LABEL_MAX));
        if chars.next().is_some() {
            out.push('…');
        }
        out.push_str(" · ");
    }
    let room = LEAD.saturating_sub(out.chars().count());
    let before = &text[..start];
    // Cut, the text before the match leaves one of its characters to the `…`.
    let keep = if before.chars().count() <= room {
        room
    } else {
        room.saturating_sub(1)
    };
    let from = match keep {
        0 => start,
        k => before.char_indices().rev().nth(k - 1).map_or(0, |(i, _)| i),
    };
    if from > 0 {
        out.push('…');
    }
    let used = out.chars().count() + text[from..end].chars().count();
    let to = text[end..]
        .char_indices()
        .nth(SNIPPET.saturating_sub(used))
        .map_or(text.len(), |(i, _)| end + i);
    let units = |s: &str| s.encode_utf16().count();
    let mut mark = [0, 0];
    // Spaces at the start are dropped, runs of them become one.
    let mut space = true;
    for (i, c) in text[from..to].char_indices() {
        if from + i == start {
            mark[0] = units(&out);
        }
        if from + i == end {
            mark[1] = units(&out);
        }
        if c.is_whitespace() {
            if !space {
                out.push(' ');
            }
            space = true;
        } else {
            out.push(c);
            space = false;
        }
    }
    if end == to {
        mark[1] = units(&out);
    }
    if out.ends_with(' ') {
        out.pop();
    }
    if to < text.len() {
        out.push('…');
    }
    (out, mark)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::test_dir;
    use serde_json::{json, Value};
    use std::path::PathBuf;

    fn agent(id: &str, project: &str) -> AgentRef {
        AgentRef {
            id: id.into(),
            project_id: project.into(),
            name: format!("agent-{id}"),
            archived: false,
            cwd: r"C:\code\demo".into(),
            last_activity: 0,
            created_at: 7,
        }
    }

    fn append(item: Value) -> String {
        json!({ "op": "append", "item": item }).to_string()
    }

    fn patch(id: &str, fields: Value) -> String {
        json!({ "op": "patch", "id": id, "patch": fields }).to_string()
    }

    fn user(id: &str, text: &str, ts: i64) -> String {
        append(
            json!({ "kind": "user", "id": id, "text": text, "images": 0, "ts": ts, "queued": false }),
        )
    }

    fn claude(id: &str, text: &str) -> String {
        append(json!({ "kind": "text", "id": id, "text": text, "streaming": false }))
    }

    fn tool(id: &str, name: &str, input: Value, output: &str) -> String {
        append(json!({
            "kind": "tool", "id": id, "name": name, "input": input, "status": "ok", "ts": 5,
            "result": { "isError": false, "text": output },
        }))
    }

    /// A folder of conversations, one log per agent.
    fn logs(name: &str, logs: &[(&str, Vec<String>)]) -> PathBuf {
        let dir = test_dir(name);
        for (agent, lines) in logs {
            let mut text = lines.join("\n");
            text.push('\n');
            std::fs::write(dir.join(format!("{agent}.jsonl")), text).unwrap();
        }
        dir
    }

    fn query(text: &str) -> Query<'_> {
        Query {
            text,
            project_id: None,
            archived: true,
        }
    }

    fn run(dir: &Path, agents: &[AgentRef], q: &Query) -> Found {
        search(dir, agents, q, Instant::now() + TIME_LIMIT, &|| false)
    }

    fn find(dir: &Path, agents: &[AgentRef], text: &str) -> Found {
        run(dir, agents, &query(text))
    }

    fn ids(found: &Found) -> Vec<&str> {
        found.hits.iter().map(|h| h.item_id.as_str()).collect()
    }

    /// The part of the snippet its mark covers.
    fn marked(h: &Hit) -> String {
        let units: Vec<u16> = h.snippet.encode_utf16().collect();
        String::from_utf16(&units[h.mark[0]..h.mark[1]]).unwrap()
    }

    /// The characters of the snippet before its match, as the palette's line shows them.
    fn before_mark(h: &Hit) -> usize {
        let units: Vec<u16> = h.snippet.encode_utf16().collect();
        String::from_utf16(&units[..h.mark[0]])
            .unwrap()
            .chars()
            .count()
    }

    #[test]
    fn finds_the_words_of_the_user_and_of_claude_newest_first() {
        let dir = logs(
            "cs-text",
            &[(
                "a1",
                vec![
                    user("u1", "Ajoute la pagination à l'API", 1000),
                    claude("m1:0", "J'ai ajouté la pagination, avec des tests."),
                    user("u2", "Merci", 3000),
                ],
            )],
        );
        let found = find(&dir, &[agent("a1", "p1")], "pagination");
        assert_eq!(ids(&found), ["m1:0", "u1"]);
        let h = &found.hits[1];
        assert_eq!(
            (
                h.agent_id.as_str(),
                h.project_id.as_str(),
                h.agent_name.as_str()
            ),
            ("a1", "p1", "agent-a1")
        );
        assert!(!h.archived);
        assert_eq!((h.event_index, h.at), (0, 1000));
        assert_eq!(h.snippet, "Ajoute la pagination à l'API");
        assert_eq!(marked(h), "pagination");
        assert_eq!(found.hits[0].event_index, 1);
        assert!(!found.capped && !found.timed_out);
    }

    #[test]
    fn ignores_case_and_accents() {
        let dir = logs(
            "cs-accents",
            &[(
                "a1",
                vec![
                    user("u1", "Gère l'événement « clic »", 1),
                    claude("m1:0", "L'ÉVÉNEMENT part deux fois."),
                    // Decomposed accents, as macOS writes file names.
                    claude("m2:0", "Le fichier eve\u{301}nements.md"),
                    claude("m3:0", "Rien à voir."),
                ],
            )],
        );
        let found = find(&dir, &[agent("a1", "p1")], "evenement");
        assert_eq!(ids(&found), ["m2:0", "m1:0", "u1"]);
        assert_eq!(marked(&found.hits[1]), "ÉVÉNEMENT");
        assert_eq!(marked(&found.hits[0]), "eve\u{301}nement");
        assert_eq!(ids(&find(&dir, &[agent("a1", "p1")], "ÉVÈNEMENT")).len(), 3);
        // A typographic apostrophe is the one typed.
        assert_eq!(ids(&find(&dir, &[agent("a1", "p1")], "l’évé")).len(), 2);
    }

    #[test]
    fn finds_a_tool_by_its_command_its_file_and_its_output() {
        let dir = logs(
            "cs-tools",
            &[(
                "a1",
                vec![
                    tool(
                        "t1",
                        "Bash",
                        json!({ "command": "npm test", "description": "Lance les tests" }),
                        "PASS src/api.test.ts\nFAIL src/db.test.ts > connects",
                    ),
                    tool(
                        "t2",
                        "Read",
                        json!({ "file_path": r"C:\code\demo\src\db.ts" }),
                        "export const db = open();",
                    ),
                    tool(
                        "t3",
                        "Edit",
                        json!({ "file_path": r"C:\code\demo\src\api.ts", "old_string": "motdepasse", "new_string": "secret" }),
                        "",
                    ),
                ],
            )],
        );
        let a = [agent("a1", "p1")];
        let failing = find(&dir, &a, "FAIL");
        assert_eq!(ids(&failing), ["t1"]);
        assert!(failing.hits[0].snippet.starts_with("Bash · "));
        assert!(failing.hits[0]
            .snippet
            .contains("src/api.test.ts FAIL src/db.test.ts"));
        assert_eq!(marked(&failing.hits[0]), "FAIL");
        // The file as the row shows it, from the agent's folder; either slash.
        let file = find(&dir, &a, r"src\db.ts");
        assert_eq!(ids(&file), ["t2"]);
        assert_eq!(
            file.hits[0].snippet,
            "Read · src/db.ts export const db = open();"
        );
        assert_eq!(ids(&find(&dir, &a, "npm test")), ["t1"]);
        // What an edit replaces is the file's content, not the gist of the call.
        assert!(find(&dir, &a, "motdepasse").hits.is_empty());
        assert_eq!(ids(&find(&dir, &a, "api.ts")), ["t3"]);
    }

    #[test]
    fn searches_questions_plans_notices_and_turn_errors_but_not_the_thinking() {
        let dir = logs(
            "cs-kinds",
            &[(
                "a1",
                vec![
                    append(json!({
                        "kind": "question", "id": "q1", "toolUseId": "x", "ts": 1,
                        "questions": [{ "question": "Quelle base ?", "options": [{ "label": "Postgres" }, { "label": "SQLite" }] }],
                        "answers": { "Quelle base ?": "SQLite" },
                    })),
                    // The question's call itself: shown by its card.
                    tool(
                        "t1",
                        "AskUserQuestion",
                        json!({ "questions": [{ "question": "Quelle base ?" }] }),
                        "SQLite",
                    ),
                    append(json!({
                        "kind": "permission", "id": "p1", "toolUseId": "y", "toolName": "ExitPlanMode", "ts": 2,
                        "input": { "plan": "1. Migrer vers SQLite" }, "canAlways": false, "defaultNo": false, "decision": "allow",
                    })),
                    append(
                        json!({ "kind": "notice", "id": "n1", "ts": 3, "level": "error", "text": "Quota SQLite atteint" }),
                    ),
                    append(json!({
                        "kind": "turn", "id": "r1", "ts": 4, "durationMs": 1, "cost": 0, "tokens": 0,
                        "isError": true, "interrupted": false, "error": "SQLite verrouillée",
                    })),
                    append(
                        json!({ "kind": "thinking", "id": "k1", "text": "SQLite ou Postgres…", "streaming": false }),
                    ),
                ],
            )],
        );
        assert_eq!(
            ids(&find(&dir, &[agent("a1", "p1")], "sqlite")),
            ["r1", "n1", "p1", "q1"]
        );
    }

    #[test]
    fn reads_the_messages_as_their_last_patches_left_them() {
        let dir = logs(
            "cs-patch",
            &[(
                "a1",
                vec![
                    append(json!({ "kind": "text", "id": "m1:0", "text": "", "streaming": true })),
                    patch(
                        "m1:0",
                        json!({ "text": "Voici le plan final", "streaming": false }),
                    ),
                    append(
                        json!({ "kind": "tool", "id": "t1", "name": "Bash", "input": {}, "status": "running", "ts": 2 }),
                    ),
                    patch("t1", json!({ "input": { "command": "cargo build" } })),
                    patch(
                        "t1",
                        json!({ "status": "error", "result": { "isError": true, "text": "error[E0425]: plan" } }),
                    ),
                ],
            )],
        );
        let found = find(&dir, &[agent("a1", "p1")], "plan");
        assert_eq!(ids(&found), ["t1", "m1:0"]);
        let output = &found.hits[0].snippet;
        assert!(output.starts_with("Bash · ") && output.ends_with("error[E0425]: plan"));
        assert_eq!(
            find(&dir, &[agent("a1", "p1")], "cargo build").hits[0].snippet,
            "Bash · cargo build error[E0425]: plan"
        );
    }

    #[test]
    fn stops_at_200_results() {
        let many = |n: usize| {
            (0..n)
                .map(|i| user(&format!("u{i}"), "pagination", i as i64))
                .collect()
        };
        let dir = logs("cs-cap", &[("a1", many(201)), ("a2", many(200))]);
        let over = find(&dir, &[agent("a1", "p1")], "pagination");
        assert_eq!(over.hits.len(), MAX_HITS);
        assert!(over.capped);
        // The newest ones.
        assert_eq!(over.hits[0].item_id, "u200");
        let exact = find(&dir, &[agent("a2", "p1")], "pagination");
        assert_eq!(exact.hits.len(), MAX_HITS);
        assert!(!exact.capped);
    }

    #[test]
    fn keeps_to_the_project_and_leaves_archived_agents_out_when_asked() {
        let dir = logs(
            "cs-filter",
            &[
                ("a1", vec![user("u1", "pagination", 1)]),
                ("a2", vec![user("u2", "pagination", 1)]),
                ("a3", vec![user("u3", "pagination", 1)]),
            ],
        );
        let mut archived = agent("a3", "p1");
        archived.archived = true;
        let agents = [agent("a1", "p1"), agent("a2", "p2"), archived];
        let of = |project_id, archived| {
            let q = Query {
                text: "pagination",
                project_id,
                archived,
            };
            let mut agents: Vec<String> = run(&dir, &agents, &q)
                .hits
                .into_iter()
                .map(|h| h.agent_id)
                .collect();
            agents.sort();
            agents
        };
        assert_eq!(of(None, true), ["a1", "a2", "a3"]);
        assert_eq!(of(Some("p1"), true), ["a1", "a3"]);
        assert_eq!(of(Some("p1"), false), ["a1"]);
        assert_eq!(of(None, false), ["a1", "a2"]);
        let a3 = run(&dir, &agents, &query("pagination"))
            .hits
            .into_iter()
            .find(|h| h.agent_id == "a3");
        assert!(a3.unwrap().archived);
    }

    #[test]
    fn puts_the_most_recently_active_agents_first() {
        let dir = logs(
            "cs-order",
            &[
                ("old", vec![user("u1", "pagination", 1)]),
                ("new", vec![user("u2", "pagination", 1)]),
            ],
        );
        let (mut old, mut new) = (agent("old", "p1"), agent("new", "p1"));
        old.last_activity = 10;
        new.last_activity = 20;
        assert_eq!(ids(&find(&dir, &[old, new], "pagination")), ["u2", "u1"]);
    }

    #[test]
    fn skips_a_line_cut_short_or_invalid_and_a_missing_log() {
        let dir = test_dir("cs-broken");
        let text = [
            user("u1", "pagination", 1),
            "pas du json".to_string(),
            json!({ "op": "append" }).to_string(),
            user("u3", "pagination encore", 3),
            // The app stopped while writing this one.
            r#"{"op":"append","item":{"kind":"user","id":"u2","text":"paginat"#.to_string(),
        ]
        .join("\n");
        std::fs::write(dir.join("a1.jsonl"), text).unwrap();
        let found = find(
            &dir,
            &[agent("a1", "p1"), agent("gone", "p1")],
            "pagination",
        );
        assert_eq!(ids(&found), ["u3", "u1"]);
        assert_eq!(found.hits[0].event_index, 1);
    }

    #[test]
    fn centers_the_snippet_on_the_match_on_one_line() {
        let long = format!(
            "{}\n\nla pagination\tcasse {}",
            "début ".repeat(40),
            "suite ".repeat(60)
        );
        let dir = logs(
            "cs-snippet",
            &[(
                "a1",
                vec![claude("m1:0", &long), claude("m2:0", "🎉 pagination !")],
            )],
        );
        let found = find(&dir, &[agent("a1", "p1")], "pagination casse");
        let h = &found.hits[0];
        assert!(
            h.snippet.starts_with('…') && h.snippet.ends_with('…'),
            "{}",
            h.snippet
        );
        assert!(
            h.snippet.contains("début la pagination casse suite"),
            "{}",
            h.snippet
        );
        assert!(!h.snippet.contains("  "));
        assert_eq!(marked(h), "pagination casse");
        // Early on the line: the palette shows some 85 characters of it.
        let before = before_mark(h);
        assert!(
            (20..=LEAD).contains(&before),
            "{before} characters before the match"
        );
        assert!(h.snippet.chars().count() <= SNIPPET + 1);
        // Indices of the window's strings: an emoji is two.
        let emoji = &find(&dir, &[agent("a1", "p1")], "pagination !").hits[0];
        assert_eq!(emoji.mark, [3, 15]);
    }

    #[test]
    fn keeps_the_match_early_on_the_line_behind_a_tool_s_label() {
        let output = format!("{}la pagination casse ici", "ligne de résultat ".repeat(10));
        let dir = logs(
            "cs-label",
            &[(
                "a1",
                vec![
                    tool(
                        "t1",
                        "mcp__claude_ai_Atlassian__searchJiraIssuesUsingJql",
                        json!({ "jql": "project = ESC" }),
                        &output,
                    ),
                    tool("t2", "Bash", json!({ "command": "npm test" }), &output),
                ],
            )],
        );
        let found = find(&dir, &[agent("a1", "p1")], "pagination casse");
        assert_eq!(ids(&found), ["t2", "t1"]);
        for h in &found.hits {
            assert_eq!(marked(h), "pagination casse");
            assert!(before_mark(h) <= LEAD, "{}", h.snippet);
        }
        assert!(found.hits[0].snippet.starts_with("Bash · …"));
        // A long label is shortened, the match still in sight.
        assert!(
            found.hits[1].snippet.starts_with("claude_ai_Atlass… · …"),
            "{}",
            found.hits[1].snippet
        );
    }

    #[test]
    fn finds_a_file_of_an_agent_whose_folder_changes_length_in_lowercase() {
        // The Kelvin sign is three bytes, its lowercase `k` one.
        let dir = logs(
            "cs-kelvin",
            &[(
                "a1",
                vec![tool(
                    "t1",
                    "Read",
                    json!({ "file_path": r"C:\code\kelvin\src\db.ts" }),
                    "",
                )],
            )],
        );
        let mut a = agent("a1", "p1");
        a.cwd = "C:\\code\\\u{212A}elvin".into();
        let found = find(&dir, &[a], "db.ts");
        assert_eq!(found.hits[0].snippet, "Read · src/db.ts");
    }

    #[test]
    fn dates_a_message_by_the_one_before_it_when_it_has_no_time_of_its_own() {
        let dir = logs(
            "cs-at",
            &[(
                "a1",
                vec![
                    claude("m0:0", "pagination"),
                    user("u1", "ok", 1000),
                    claude("m1:0", "pagination"),
                ],
            )],
        );
        let found = find(&dir, &[agent("a1", "p1")], "pagination");
        // The first one, before anything else: the agent's creation.
        assert_eq!(
            found.hits.iter().map(|h| h.at).collect::<Vec<_>>(),
            [1000, 7]
        );
    }

    #[test]
    fn finds_nothing_for_blank_text() {
        let dir = logs("cs-blank", &[("a1", vec![user("u1", "a b", 1)])]);
        assert!(find(&dir, &[agent("a1", "p1")], "   ").hits.is_empty());
    }

    #[test]
    fn gives_what_it_found_when_its_time_is_up() {
        let dir = logs("cs-time", &[("a1", vec![user("u1", "pagination", 1)])]);
        let found = search(
            &dir,
            &[agent("a1", "p1")],
            &query("pagination"),
            Instant::now(),
            &|| false,
        );
        assert!(found.hits.is_empty());
        assert!(found.timed_out);
    }

    #[test]
    fn stops_when_a_newer_search_starts() {
        let dir = logs("cs-stale", &[("a1", vec![user("u1", "pagination", 1)])]);
        let searches = Searches::default();
        let first = searches.start();
        assert!(!first());
        let second = searches.start();
        assert!(first());
        assert!(!second());
        let found = search(
            &dir,
            &[agent("a1", "p1")],
            &query("pagination"),
            Instant::now() + TIME_LIMIT,
            &first,
        );
        assert!(found.hits.is_empty());
        assert!(!found.timed_out);
    }
}
