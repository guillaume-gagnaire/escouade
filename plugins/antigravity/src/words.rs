//! What Antigravity CLI's words are, as Escouade drives it: its launch, its modes and rules, the files of the agent's own
//! home, its calls in ACP's form, its questions. Pure.
//!
//! What it does comes from the real runs of `agy` 1.4.9 (ESC-269, docs/PROTOCOL.md, « Antigravity CLI »), whose streams are
//! in `tests/fixtures/agy/`.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// The variable the core gives each agent's own home in: its plugin gives it to `agy` as `USERPROFILE` and `HOME`.
pub const HOME_VAR: &str = "ESCOUADE_AGY_HOME";

/// The keys of the machine that would take the place of its Google account: the first, an account added by its key goes
/// by (`modelProvider: gemini`, ESC-271).
pub const KEY_VARS: [&str; 2] = ["GEMINI_API_KEY", "GOOGLE_API_KEY"];

/// What keeps `agy` from updating itself (a string of its program, ESC-269): stopped with its tree, an update it began would
/// leave no `agy` behind (ESC-133). In every launch, by `home_env`.
pub const NO_SELF_UPDATE: (&str, &str) = ("AGY_CLI_DISABLE_AUTO_UPDATE", "1");

/// Its default model: Gemini Flash, its effort apart.
pub const DEFAULT: &str = "gemini-3.8-flash";

/// The efforts of a Gemini model: part of its id in `agy models` (`gemini-3.8-flash-low`), its `--effort` beside its id
/// without them (`xhigh` and `max` refused by Flash, tried).
pub const EFFORTS: [&str; 3] = ["low", "medium", "high"];

/// What an approved plan is followed by.
pub const PLAN_GO: &str = "The plan is approved: carry it out now.";

/// What the first message tells the model of its questions: `-p` skips `ask_question`, Escouade asks the user once the
/// turn is over.
pub const QUESTIONS: &str = "<escouade>\nWhen you ask the user something with ask_question, end your turn right after it, without commenting on the answer you get: Escouade shows your questions to the user and sends the answers as the next message.\n</escouade>";

/// The model, effort and mode in force.
#[derive(Debug, Clone, Default)]
pub struct Settings {
    pub model: String,
    pub effort: String,
    pub mode: String,
}

/// What the rules of an agent hold beside its mode: its folder, the folders it reads, its fence, a discussion that changes
/// nothing.
#[derive(Debug, Clone, Default)]
pub struct AgyRules {
    pub cwd: String,
    pub read_dirs: Vec<String>,
    pub fence: Vec<String>,
    pub read_only: bool,
}

/// The tokens a turn spent, as its steps told them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Usage {
    pub input: u64,
    pub output: u64,
    pub cached: u64,
}

impl Usage {
    /// What a step of `agy` told (`usage`).
    pub fn add(&mut self, u: &Value) {
        let n = |k: &str| u[k].as_u64().unwrap_or(0);
        self.input += n("input_tokens");
        self.output += n("output_tokens") + n("thinking_tokens");
        self.cached += n("cache_read_tokens");
    }
}

/// A call `agy` refused for want of a permission.
#[derive(Debug, Clone)]
pub struct Denied {
    pub step: u64,
    pub tool: String,
    pub args: Value,
}

/// What `agy`'s error says of a refusal for want of a permission (its step's `error.message`).
pub fn is_denial(message: &str) -> bool {
    message.contains("permission check failed") || message.contains("denied permission")
}

/// The `--mode` of `agy` for Escouade's `mode`: its plan alone. « Demander », « Édits auto » and « Tout accepter » are its
/// default mode, where what Escouade's rules do not allow is refused and comes back to the plugin, which judges it
/// (`judge`): `agy`'s own accept-edits would let a write in `.git` or `.agents` through unseen.
pub fn agy_mode(mode: &str) -> Option<&'static str> {
    match mode {
        "plan" => Some("plan"),
        _ => None,
    }
}

/// The efforts in which `id` (`gemini-3.8-flash-low`) ends, and its id without it.
pub fn effort_of(id: &str) -> Option<(&str, &'static str)> {
    EFFORTS
        .iter()
        .find_map(|e| id.strip_suffix(&format!("-{e}")).map(|base| (base, *e)))
}

/// A Gemini model named without its effort (`gemini-3.8-flash`): `--effort` beside it.
pub fn supports_effort(model: &str) -> bool {
    model.starts_with("gemini-") && effort_of(model).is_none()
}

/// The arguments of a program of the agent: stream-json both ways, its model (and effort), its mode, its conversation.
pub fn launch_args(settings: &Settings, conversation: Option<&str>) -> Vec<String> {
    let mut args: Vec<String> = [
        "-p=",
        "--input-format",
        "stream-json",
        "--output-format",
        "stream-json",
    ]
    .map(String::from)
    .to_vec();
    let model = settings.model.trim();
    if !model.is_empty() && !model.contains(char::is_whitespace) {
        args.push("--model".into());
        args.push(model.into());
        if supports_effort(model) && EFFORTS.contains(&settings.effort.as_str()) {
            args.push("--effort".into());
            args.push(settings.effort.clone());
        }
    }
    if let Some(mode) = agy_mode(&settings.mode) {
        args.push("--mode".into());
        args.push(mode.into());
    }
    if let Some(id) = conversation.filter(|c| !c.is_empty()) {
        args.push("--conversation".into());
        args.push(id.into());
    }
    args
}

/// Its folder of config in the agent's home.
pub fn gemini_dir(home: &Path) -> PathBuf {
    home.join(".gemini")
}

/// The settings of `agy` in the agent's home.
pub fn settings_file(home: &Path) -> PathBuf {
    gemini_dir(home)
        .join("antigravity-cli")
        .join("settings.json")
}

/// Its MCP config in the agent's home (`agy mcp add` writes the same file).
pub fn mcp_file(home: &Path) -> PathBuf {
    gemini_dir(home).join("config").join("mcp_config.json")
}

/// The transcript of the conversation `id`, step by step, as `agy` writes it.
pub fn transcript(home: &Path, id: &str) -> PathBuf {
    gemini_dir(home)
        .join("antigravity-cli")
        .join("brain")
        .join(id)
        .join(".system_generated")
        .join("logs")
        .join("transcript_full.jsonl")
}

/// The rules a mode allows without asking: everything in « Tout accepter » when nothing is left to judge (no fence, no
/// discussion), else none. What `agy` refuses comes back at the end of its turn, judged by `judge`; what is allowed then
/// is a rule for the turn that goes on, which `agy` may read as a start of the text (« commands that start with »): until
/// the real `agy` was tried (`real_agy_rules`), no command is allowed that way but by the user (ESC-376).
pub fn mode_rules(rules: &AgyRules, mode: &str) -> Vec<String> {
    if mode != "bypassPermissions" || !rules.fence.is_empty() || rules.read_only {
        return Vec::new();
    }
    [
        "command(*)",
        "write_file(*)",
        "read_file(*)",
        "mcp(*)",
        "read_url(*)",
        "execute_url(*)",
    ]
    .map(String::from)
    .to_vec()
}

/// The rules refused whatever the mode: the paths of a private project (refused to read refuses to write too, `agy`'s
/// rule), and for a discussion every write and command.
pub fn deny_rules(rules: &AgyRules) -> Vec<String> {
    let mut deny: Vec<String> = rules
        .fence
        .iter()
        .map(|p| p.trim_end_matches(['/', '\\']))
        .filter(|p| !p.is_empty())
        .map(|p| format!("read_file({p})"))
        .collect();
    if rules.read_only {
        deny.push("write_file(*)".into());
        deny.push("command(*)".into());
    }
    deny
}

/// The keys of the settings the plugin takes out whatever the file held: the agent can write its home, so what it planted
/// there is not kept (`agy`'s defaults hold), but `toolPermission`, set again (`TOOL_PERMISSION`).
pub const SET_KEYS: [&str; 2] = ["toolPermission", "agentMode"];

/// The `toolPermission` the plugin writes: what no rule allows needs a review, refused in `-p` (`agy` 1.5.3's default,
/// measured by `real_agy_rules`; `agent-decides` or `always-proceed` would let `agy` judge alone, ESC-388).
pub const TOOL_PERMISSION: &str = "request-review";

/// The settings `before` (a JSON object, else none) with their `permissions` as Escouade sets them (its mode's rules, what
/// the user allowed for the next turn, its refusals: `deny` before all, `agy`'s rule), `SET_KEYS` taken out and
/// `toolPermission` set to `TOOL_PERMISSION`.
pub fn settings_doc(before: &str, rules: &AgyRules, mode: &str, allowed: &[String]) -> Value {
    let mut doc: Value = serde_json::from_str(before)
        .ok()
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}));
    if let Some(o) = doc.as_object_mut() {
        for k in SET_KEYS {
            o.remove(k);
        }
    }
    let mut allow = mode_rules(rules, mode);
    for r in allowed {
        if !allow.contains(r) {
            allow.push(r.clone());
        }
    }
    let mut deny = deny_rules(rules);
    deny.extend(guard_rules(&rules.cwd, allowed));
    doc["permissions"] = json!({ "allow": allow, "deny": deny, "ask": [] });
    doc["toolPermission"] = json!(TOOL_PERMISSION);
    doc
}

/// The separator `path` is spelled with.
fn separator(path: &str) -> char {
    if path.contains('\\') {
        '\\'
    } else {
        '/'
    }
}

/// The refusals that guard a turn going on with a write allowed (`allowed`, rules `write_file(<path>)`, ESC-376): each
/// folder of `KEPT` at the root of the agent's folder `cwd`, and right under each path allowed: `agy` 1.5.3's
/// `write_file(<path>)` covers what is under it, `<path>\.git` too, and a path allowed may be a folder, a submodule's
/// (ESC-389). But one a write allowed is in (`deny` wins over `allow`, `real_agy_rules`). They hold against the name they
/// say alone, not a short name (`GIT~1`) nor a final dot (`.git.`): what is accepted by the rules reaches a file alone
/// (`fs/fileOnly`), nothing under it; for a folder the user allows on its card, they keep the plain names. No refusal of `<path>\..`:
/// `agy` 1.5.3 reads it as the folder of `<path>`, which would refuse the write allowed itself, and refuses
/// `<path>\..\.git\x` with no such refusal (ESC-388).
pub fn guard_rules(cwd: &str, allowed: &[String]) -> Vec<String> {
    let writes: Vec<&str> = allowed
        .iter()
        .filter_map(|r| r.strip_prefix("write_file(")?.strip_suffix(')'))
        .filter(|p| *p != "*")
        .collect();
    if writes.is_empty() {
        return Vec::new();
    }
    let bases = std::iter::once(cwd)
        .chain(writes.iter().copied())
        .map(|b| (b.trim_end_matches(['/', '\\']), separator(b)))
        .filter(|(b, _)| !b.is_empty());
    let mut deny: Vec<String> = Vec::new();
    for (base, sep) in bases {
        for k in KEPT {
            let folder = format!("{base}{sep}{k}");
            let rule = format!("write_file({folder})");
            if !writes.iter().any(|w| escouade_acp::inside(w, &folder)) && !deny.contains(&rule) {
                deny.push(rule);
            }
        }
    }
    deny
}

/// `doc`, the settings of an agent, with the provider of its models: the Gemini API by `GEMINI_API_KEY` for an account added
/// by its key (its reference: `modelProvider: "gemini"`), else its Google sign-in (the setting taken out).
pub fn set_provider(doc: &mut Value, by_key: bool) {
    match doc.as_object_mut() {
        Some(o) if by_key => {
            o.insert("modelProvider".into(), json!("gemini"));
        }
        Some(o) => {
            o.remove("modelProvider");
        }
        None => {}
    }
}

/// The MCP config `before` (a JSON object, else none) with Escouade's server alone in it, its token in the file (`agy`
/// expands no variable there, ESC-269), or none. A server the agent wrote there would start at each launch: not kept.
pub fn grant_doc(before: &str, escouade: Option<(&str, &str)>) -> Value {
    let mut doc: Value = serde_json::from_str(before)
        .ok()
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}));
    doc["mcpServers"] = json!({});
    if let Some((url, token)) = escouade {
        doc["mcpServers"][escouade_acp::SERVER] = json!({
            "serverUrl": url, "headers": { "Authorization": format!("Bearer {token}") },
            "disabled": false,
        });
    }
    doc
}

/// Whether the call `name` with `args` writes the plan of the mode `plan` (`brain/<id>/plan_*.md`).
pub fn is_plan_file(name: &str, args: &Value) -> bool {
    if !matches!(name, "write_file" | "write_to_file") {
        return false;
    }
    let path = args["Path"].as_str().unwrap_or_default().replace('\\', "/");
    let file = path.rsplit('/').next().unwrap_or_default();
    path.contains("/brain/") && file.starts_with("plan") && file.ends_with(".md")
}

/// `path` relative to the agent's home `home`, with `/`, when it is in it and climbs nowhere: by its text, as a module
/// places a path of Windows (its `Path` is POSIX's). None else.
pub fn in_home(path: &str, home: &str) -> Option<String> {
    let p = path.trim().replace('\\', "/");
    let h = home.trim().replace('\\', "/");
    let h = h.trim_end_matches('/');
    if h.is_empty() || p.len() <= h.len() + 1 || !p.is_char_boundary(h.len()) {
        return None;
    }
    let (start, rest) = p.split_at(h.len());
    let rel = rest.strip_prefix('/')?;
    let same = if cfg!(windows) || h.as_bytes().get(1) == Some(&b':') {
        start.eq_ignore_ascii_case(h)
    } else {
        start == h
    };
    let climbs = rel
        .split('/')
        .any(|c| c.is_empty() || c == "." || c == "..");
    (same && !climbs).then(|| rel.to_string())
}

/// The path a call names (`Path`, `AbsolutePath`, `TargetFile`, `DirectoryPath`).
pub fn path_of(args: &Value) -> Option<&str> {
    [
        "Path",
        "AbsolutePath",
        "TargetFile",
        "DirectoryPath",
        "SearchPath",
    ]
    .iter()
    .find_map(|k| args[*k].as_str())
}

/// The folders a write in is no work of the agent's: what git runs (`.git`, the hooks of `.husky` and `.githooks`) and
/// what `agy` reads (`.agents/mcp_config.json` and `hooks.json` of the project, `.gemini`). Always a card, never
/// « Toujours ».
pub const KEPT: [&str; 5] = [".git", ".husky", ".githooks", ".agents", ".gemini"];

/// The tools of `agy` that write a file.
pub fn writes(tool: &str) -> bool {
    matches!(
        tool,
        "write_file"
            | "write_to_file"
            | "edit_file"
            | "replace_file_content"
            | "multi_replace_file_content"
            | "sed_file"
            | "notebook_edit"
    )
}

/// The tools of `agy` that read.
pub fn reads(tool: &str) -> bool {
    matches!(
        tool,
        "read_file" | "view_file" | "list_dir" | "find_by_name" | "grep_search"
    )
}

/// Whether `path` starts at a root: `/`, `\`, a drive.
fn rooted(path: &str) -> bool {
    let b = path.as_bytes();
    path.starts_with(['/', '\\']) || (b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':')
}

/// `path` as the plugin places it: a relative one joined to the agent's folder `cwd` (`escouade_acp::inside` resolves its
/// `..`). None for a path whose place a shell or `agy` would make (`~`, `$`, `%`): it is in no folder.
pub fn resolved(path: &str, cwd: &str) -> Option<String> {
    let path = path.trim();
    if path.is_empty() || path.starts_with('~') || path.contains(['$', '%']) {
        return None;
    }
    if rooted(path) || cwd.is_empty() {
        return Some(path.to_string());
    }
    Some(format!("{}/{path}", cwd.trim_end_matches(['/', '\\'])))
}

/// The parts of `path`, lower case, its `.` and `..` resolved (None when it climbs over its root).
fn parts(path: &str) -> Option<Vec<String>> {
    let mut out: Vec<String> = Vec::new();
    for part in path.split(['/', '\\']) {
        match part {
            "" | "." => {}
            ".." => {
                out.pop()?;
            }
            p => out.push(p.to_lowercase()),
        }
    }
    Some(out)
}

/// Whether `path` (resolved) is in a folder of `KEPT`, under the agent's folder `cwd` (anywhere in it when it is not under
/// it), whatever its case and the names Windows takes for it (`.git.`, `.git::$INDEX_ALLOCATION`, a short name like
/// `GIT~1` or `HUSKY~1`: any `~` or `:` counts). What cannot be placed is in it.
pub fn kept(path: &str, cwd: &str) -> bool {
    if escouade_acp::in_git_folder(path, cwd) {
        return true;
    }
    let (Some(p), Some(d)) = (parts(path), parts(cwd)) else {
        return true;
    };
    let rest = p.strip_prefix(d.as_slice()).unwrap_or(&p);
    let drive = |c: &str| c.len() == 2 && c.ends_with(':') && c.as_bytes()[0].is_ascii_alphabetic();
    rest.iter().filter(|c| !drive(c)).any(|c| {
        let name = c
            .split(':')
            .next()
            .unwrap_or(c)
            .trim_end_matches(['.', ' ']);
        c.contains([':', '~']) || KEPT.contains(&name)
    })
}

/// The keys of a call that name a path.
const PATH_KEYS: [&str; 5] = [
    "Path",
    "AbsolutePath",
    "TargetFile",
    "DirectoryPath",
    "SearchPath",
];

/// The words of a command line that may name a path: split on blanks, quotes and the signs of a shell (`;`, `|`, `&`,
/// `<`, `>`, `=`, `,`, a parenthesis, a backtick).
fn command_words(line: &str) -> impl Iterator<Item = &str> {
    line.split(|c: char| {
        c.is_whitespace()
            || matches!(
                c,
                '"' | '\'' | '`' | ';' | '|' | '&' | '<' | '>' | '=' | ',' | '(' | ')'
            )
    })
    .filter(|w| !w.is_empty())
}

/// Whether a call of `agy` names a path of `fence`: a text that names it, or a path that is in it once resolved against
/// the agent's folder `cwd` (`C:\wt\..\prive\x`, `..\prive\x`), a word of its command line too (`type ..\prive\x`,
/// ESC-376). A short name, a link or a path a shell makes (`$`, `%`, `~`) is not followed: the rules `deny` of `agy` and
/// the cards of the commands are what is left there.
pub fn fenced(args: &Value, fence: &[String], cwd: &str) -> bool {
    if fence.is_empty() {
        return false;
    }
    let named = args.as_object().into_iter().flatten().any(|(_, v)| {
        v.as_str()
            .is_some_and(|t| fence.iter().any(|p| escouade_acp::names(t, p)))
    });
    named
        || PATH_KEYS
            .iter()
            .filter_map(|k| args[*k].as_str())
            .chain(
                args["CommandLine"]
                    .as_str()
                    .into_iter()
                    .flat_map(command_words),
            )
            .filter_map(|p| resolved(p, cwd))
            .any(|p| fence.iter().any(|f| escouade_acp::inside(&p, f)))
}

/// Whether a text can be put in a rule of `agy` as it is: none of the signs its rules might read as their own (`*`, `?`,
/// a parenthesis, a bracket, a brace: a glob's) nor a line break. `agy` has no way known to escape them.
fn plain(text: &str) -> bool {
    !text.is_empty()
        && !text
            .chars()
            .any(|c| matches!(c, '*' | '?' | '(' | ')' | '[' | ']' | '{' | '}') || c.is_control())
}

/// Whether `path`, as the call spells it, is already the absolute and normal path the plugin judges: rooted, one
/// separator, no empty part, no `.` nor `..`, no part ending in a dot or a blank (Windows drops them), no separator at its
/// end, nothing a rule would read as its own (`plain`). A rule made of anything else is not what was judged (ESC-376).
pub fn normal(path: &str) -> bool {
    if !rooted(path) || !plain(path) || (path.contains('/') && path.contains('\\')) {
        return false;
    }
    let mut parts = path.split(['/', '\\']);
    // A root `/` or `\` leaves an empty first part; a drive, its `C:`.
    let first = parts.next().unwrap_or_default();
    let drive = first.len() == 2 && first.ends_with(':');
    if !(first.is_empty() || drive) {
        return false;
    }
    let rest: Vec<&str> = parts.collect();
    !rest.is_empty()
        && rest.iter().all(|p| {
            !p.is_empty()
                && *p != "."
                && *p != ".."
                && !p.ends_with(['.', ' '])
                && !p.starts_with(' ')
        })
}

/// Whether `path` (resolved) is strictly under the agent's folder `cwd`: in it, and not the folder itself.
fn under(path: &str, cwd: &str) -> bool {
    !cwd.is_empty() && escouade_acp::inside(path, cwd) && !escouade_acp::inside(cwd, path)
}

/// Whether a write of `path` (resolved) would cover more than a file of the agent's: the folder `cwd` itself or what holds
/// it, or a last part a name of `KEPT` starts with (`.g`, `.ag`: a rule read as a start of the text would reach it).
fn covers_kept(path: &str, cwd: &str) -> bool {
    if !cwd.is_empty() && escouade_acp::inside(cwd, path) {
        return true;
    }
    let last = path
        .trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or_default()
        .to_lowercase();
    last.is_empty() || KEPT.iter().any(|k| k.starts_with(last.as_str()))
}

/// The rule of `agy` that would allow the call `tool` with `args`, the call as the model is told it, and whether the rule
/// can be written (`plain`).
fn shape_of(tool: &str, args: &Value) -> Option<(String, String, bool)> {
    let path = path_of(args);
    match tool {
        "run_command" => {
            let c = args["CommandLine"].as_str()?.trim();
            Some((
                format!("command({c})"),
                format!("the command `{c}`"),
                plain(c),
            ))
        }
        t if writes(t) => {
            let p = path?;
            Some((format!("write_file({p})"), format!("writing {p}"), plain(p)))
        }
        t if reads(t) => {
            let p = path?;
            Some((format!("read_file({p})"), format!("reading {p}"), plain(p)))
        }
        "call_mcp_tool" => {
            let (s, t) = (args["ServerName"].as_str()?, args["ToolName"].as_str()?);
            Some((
                format!("mcp({s}/{t})"),
                format!("the tool {t} of the MCP server {s}"),
                plain(s) && plain(t) && !s.contains('/') && !t.contains('/'),
            ))
        }
        "read_url_content" | "open_browser_url" => {
            let url = args["Url"].as_str().or(args["url"].as_str())?;
            let host = url
                .split("://")
                .nth(1)
                .unwrap_or(url)
                .split(['/', ':', '?'])
                .next()?;
            Some((
                format!("read_url({host})"),
                format!("reading {url}"),
                plain(host),
            ))
        }
        _ => None,
    }
}

/// What `agy` names the call `tool` with `args` in the `denied_actions` of its `result` (`command`, `write_file`,
/// `read_file`, `mcp`, `read_url`): the action of its rule.
pub fn action_of(tool: &str, args: &Value) -> Option<String> {
    let (rule, _, _) = shape_of(tool, args)?;
    rule.split('(').next().map(str::to_string)
}

/// The rule of `agy` that allows the call `tool` with `args` (`command(echo hi)`, `write_file(C:\x\b.txt)`,
/// `mcp(escouade/get_board)`), and the call as the model is told it was allowed or refused. None for a call no rule names,
/// or whose text no rule can hold as it is (`unruled`).
pub fn rule_of(tool: &str, args: &Value) -> Option<(String, String)> {
    shape_of(tool, args)
        .filter(|(_, _, plain)| *plain)
        .map(|(r, s, _)| (r, s))
}

/// The call as the model is told it, when a rule would name it but cannot hold it as it is (a `*`, a `?`, a parenthesis, a
/// line break).
pub fn unruled(tool: &str, args: &Value) -> Option<String> {
    shape_of(tool, args)
        .filter(|(_, _, plain)| !*plain)
        .map(|(_, s, _)| s)
}

/// What the plugin makes of a call `agy` refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Refused without a card (« Projet privé », a discussion).
    Refused,
    /// Allowed without a card, by Escouade's rules.
    Allowed,
    /// A card, with « Toujours » or without.
    Asked { always: bool },
}

/// What Escouade's rules make of the call `d` that `agy` refused (its rule `rule`), the agent in the mode `mode`, with what
/// the user allowed it for good (`always`: exact rules, kept by the plugin). What it allows becomes a rule of `agy` for
/// the turn that goes on, which `agy` may read as a start of the text: until the real `agy` was tried, nothing is allowed
/// that way that would not hold in that worst case (ESC-376).
/// - a discussion, or a path of the fence (`fenced`, the words of a command too): refused, never a card;
/// - exactly a call the user allowed for good: allowed;
/// - a command: a card (`git add` and `git commit` in « Édits auto », any in « Tout accepter » with a fence);
/// - a call naming a path that is not already the absolute and normal path judged (`normal`): a card;
/// - « Tout accepter »: allowed (it only comes back with a fence, or what `agy` refuses whatever its rules);
/// - a read in a folder of the files joined to the messages: allowed;
/// - in « Demander » and « Édits auto », a write strictly under the agent's folder, neither in `KEPT` nor covering it
///   (`covers_kept`: the folder itself, `.g`), that reaches a file alone (`file_only`, the host's `fs/fileOnly`: a file
///   there, or nothing and no submodule under it; `write_file(<path>)` covers what is under `<path>`, a folder's `.git`
///   at any depth, ESC-389): allowed, the turn going on with each name of `KEPT` right under it refused (`guard_rules`);
/// - anything else a card, without « Toujours » for a fenced agent (what it covers would never be judged again), a write
///   in `KEPT` or covering it, a write in the agent's folder that may reach more than a file (a folder, a submodule), or
///   a path not normal (its rule would not be what was judged).
pub fn judge(
    rules: &AgyRules,
    mode: &str,
    always: &[String],
    d: &Denied,
    rule: &str,
    file_only: &dyn Fn(&str) -> bool,
) -> Verdict {
    if rules.read_only || fenced(&d.args, &rules.fence, &rules.cwd) {
        return Verdict::Refused;
    }
    let cwd = rules.cwd.as_str();
    let fenced_agent = !rules.fence.is_empty();
    let spelled = path_of(&d.args);
    let exact = spelled.is_none_or(normal);
    let path = spelled.and_then(|p| resolved(p, cwd));
    let write = writes(&d.tool);
    let in_kept = write
        && path
            .as_deref()
            .is_none_or(|p| kept(p, cwd) || covers_kept(p, cwd));
    // A write in the agent's folder that may reach more than a file (a folder, a submodule): what is under it would be
    // allowed, a `.git` at any depth (ESC-389). Asked of the host only for a write that could be accepted.
    let wide = write
        && exact
        && !in_kept
        && path
            .as_deref()
            .is_some_and(|p| under(p, cwd) && !file_only(p));
    let asked = Verdict::Asked {
        always: !fenced_agent && !in_kept && !wide && exact,
    };
    if exact && !in_kept && !wide && !fenced_agent && always.iter().any(|r| r == rule) {
        return Verdict::Allowed;
    }
    if d.tool == "run_command" || !exact {
        return asked;
    }
    if mode == "bypassPermissions" {
        return Verdict::Allowed;
    }
    let read_joined = reads(&d.tool)
        && path.as_deref().is_some_and(|p| {
            rules
                .read_dirs
                .iter()
                .any(|dir| escouade_acp::inside(p, dir))
        });
    let write_here = write && !in_kept && !wide && path.as_deref().is_some_and(|p| under(p, cwd));
    if read_joined || (matches!(mode, "default" | "acceptEdits") && write_here) {
        return Verdict::Allowed;
    }
    asked
}

/// The id of the call of step `step` in the conversation `session`, as both the translation and the permissions name it.
pub fn call_id(session: &str, step: u64) -> String {
    let short: String = session.chars().take(8).collect();
    format!("agy-{short}-{step}")
}

/// A call of `agy` (its tool `name`, its arguments) in the form Cursor's translation reads (ACP's `tool_call`): its `kind`,
/// `title`, `rawInput`, and the diff of a write.
pub fn acp_call(name: &str, args: &Value) -> Value {
    let path = path_of(args);
    let (kind, raw) = match name {
        "run_command" => ("execute", json!({ "command": args["CommandLine"] })),
        "write_file"
        | "write_to_file"
        | "edit_file"
        | "replace_file_content"
        | "multi_replace_file_content"
        | "sed_file" => ("edit", json!({ "path": path })),
        "read_file" | "view_file" => ("read", json!({ "path": path })),
        "grep_search" | "find_by_name" => (
            "search",
            json!({
                "pattern": args["Query"].as_str().or(args["Pattern"].as_str()).unwrap_or_default(),
                "path": path,
            }),
        ),
        "read_url_content" => ("fetch", json!({ "url": args["Url"] })),
        "call_mcp_tool" => (
            "other",
            json!({
                "providerIdentifier": args["ServerName"], "toolName": args["ToolName"],
                "args": args["Arguments"].clone(),
            }),
        ),
        "invoke_subagent" => (
            "other",
            json!({
                "_toolName": "task",
                "description": args["Role"].as_str().unwrap_or("subagent"),
                "prompt": args["Prompt"].as_str().or(args["InitialPrompt"].as_str()),
                "subagentType": args["TypeName"],
            }),
        ),
        _ => {
            let mut raw = if args.is_object() {
                args.clone()
            } else {
                json!({})
            };
            raw["_toolName"] = json!(name);
            ("other", raw)
        }
    };
    let mut call = json!({ "kind": kind, "title": name, "rawInput": raw });
    if let Some(d) = diff_of(name, args) {
        call["content"] = json!([d]);
    }
    call
}

/// The diff of a write, from its arguments: a file written whole (`Content`, `CodeContent`), or a text replaced (`Target` →
/// `Replacement`).
pub fn diff_of(name: &str, args: &Value) -> Option<Value> {
    let path = path_of(args)?;
    let pick = |keys: &[&str]| keys.iter().find_map(|k| args[*k].as_str());
    match name {
        "write_file" | "write_to_file" => {
            let new = pick(&["Content", "CodeContent"])?;
            Some(json!({ "type": "diff", "path": path, "oldText": null, "newText": new }))
        }
        "edit_file" | "replace_file_content" => {
            let old = pick(&["Target", "TargetContent"])?;
            let new = pick(&["Replacement", "ReplacementContent"])?;
            Some(json!({ "type": "diff", "path": path, "oldText": old, "newText": new }))
        }
        _ => None,
    }
}

/// The permission of a call `agy` refused, in ACP's form (`session/request_permission`), with « Toujours » or without.
pub fn permission_params(session: &str, d: &Denied, always: bool) -> Value {
    let mut tc = acp_call(&d.tool, &d.args);
    tc["toolCallId"] = json!(call_id(session, d.step));
    let mut options =
        vec![json!({ "optionId": "allow-once", "kind": "allow_once", "name": "Allow once" })];
    if always {
        options.push(
            json!({ "optionId": "allow-always", "kind": "allow_always", "name": "Always allow" }),
        );
    }
    options.push(json!({ "optionId": "reject-once", "kind": "reject_once", "name": "Reject" }));
    json!({ "sessionId": session, "toolCall": tc, "options": options })
}

/// What the card of a permission says « Autoriser » gives: `agy` may judge its rules as a start of the text, for the next
/// turn; « Toujours » is Escouade's, for this very call, its rule given again each time it comes back. Short (`ONCE_NOTE_MAX`): with a card's reason, the list beside
/// the conversation still offers its choices (it takes a note of 240 characters at most, `LIST_NOTE_MAX`).
pub fn once_note(lang: &str) -> String {
    if lang == "fr" {
        "« Autoriser » vaut pour le tour suivant ; une écriture couvre aussi ce qui est sous son chemin. « Toujours » : de même, à chaque retour de cet appel.".into()
    } else {
        "“Allow” holds for the next turn; a write also covers what is under its path. “Always”: the same, each time this call comes back.".into()
    }
}

/// Said when Escouade's rules allowed a call without a card: as « Autoriser », for the turn that goes on.
pub fn allowed_notice(lang: &str, said: &str) -> String {
    if lang == "fr" {
        format!("Escouade a autorisé d’office {said} : pour le tour suivant.")
    } else {
        format!("Escouade allowed {said} by its rules: for the next turn.")
    }
}

/// The most characters of `once_note`, leaving room for a card's reason on the list (`LIST_NOTE_MAX` is 240).
pub const ONCE_NOTE_MAX: usize = 160;

/// Said when a call is refused that no rule of `agy` can hold as it is (`unruled`).
pub fn unruled_notice(lang: &str, said: &str) -> String {
    if lang == "fr" {
        format!("Antigravity s’est vu refuser {said} : ses règles ne savent pas l’autoriser tel quel (un *, un ?, une parenthèse, un crochet, une accolade ou un retour à la ligne).")
    } else {
        format!("Antigravity was refused {said}: its rules cannot allow it as it is (a *, a ?, a parenthesis, a bracket, a brace or a line break).")
    }
}

/// Said when a call allowed for this very turn was refused again: it is not asked once more.
pub fn refused_again(lang: &str, said: &str) -> String {
    if lang == "fr" {
        format!("Antigravity a de nouveau refusé {said}, que sa règle autorisait : ce n’est pas redemandé.")
    } else {
        format!("Antigravity refused {said} again, which its rule allowed: it is not asked again.")
    }
}

/// The result of a call `agy` refused but told done (ESC-388).
pub fn refused_by_agy(lang: &str) -> &'static str {
    if lang == "fr" {
        "Refusé par Antigravity : aucune règle ne l’autorisait."
    } else {
        "Refused by Antigravity: no rule allowed it."
    }
}

/// Said when `agy`'s `result` lists a refusal of `action` (`denied_actions`) that no call of the turn matches: no card can
/// be put on it, the user is told a call was refused (ESC-389).
pub fn unmatched_refusal(lang: &str, action: &str) -> String {
    if lang == "fr" {
        format!("Antigravity a refusé un appel ({action}) sans dire lequel : aucune carte n’a pu être posée. Redemandez-le si besoin.")
    } else {
        format!("Antigravity refused a call ({action}) without saying which: no card could be put. Ask for it again if needed.")
    }
}

/// The label of a choice of `ask_question`, its mark of the one it recommends left out.
fn choice_label(o: &Value) -> String {
    let text = o
        .as_str()
        .or(o["label"].as_str())
        .unwrap_or_default()
        .trim();
    text.strip_prefix("(Recommended)")
        .unwrap_or(text)
        .trim()
        .to_string()
}

/// The questions of `ask_question` (`questions: [{question, options, is_multi_select}]`) as Cursor's `cursor/ask_question`
/// gives them.
pub fn question_params(call: &str, args: &Value) -> Value {
    let questions: Vec<Value> = args["questions"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
        .map(|(i, q)| {
            let options: Vec<Value> = q["options"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|o| {
                    let l = choice_label(o);
                    json!({ "id": l, "label": l })
                })
                .collect();
            json!({
                "id": format!("q{i}"), "prompt": q["question"], "options": options,
                "allowMultiple": q["is_multi_select"] == true,
            })
        })
        .collect();
    json!({ "toolCallId": call, "questions": questions })
}

/// What the user answered to the questions `params`, a line each.
pub fn replies(params: &Value, answers: &Value) -> Vec<String> {
    params["questions"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|q| {
            let text = q["prompt"].as_str().unwrap_or_default();
            let a = answers[text].as_str().unwrap_or_default().trim();
            format!("- {text} → {a}")
        })
        .collect()
}

/// The message that has the turn go on once the user answered (in the model's words): what was allowed, to do again, what
/// was refused, the answers; None when nothing was allowed nor answered.
pub fn follow_up(allowed: &[String], refused: &[String], replies: &[String]) -> Option<String> {
    if allowed.is_empty() && replies.is_empty() {
        return None;
    }
    let mut parts = Vec::new();
    if !allowed.is_empty() {
        parts.push(format!(
            "The user allowed {}: do it again now, then go on.",
            allowed.join(", ")
        ));
    }
    if !refused.is_empty() {
        parts.push(format!(
            "The user refused {}: don't do it.",
            refused.join(", ")
        ));
    }
    if !replies.is_empty() {
        parts.push(format!(
            "The user answered your questions:\n{}",
            replies.join("\n")
        ));
    }
    Some(parts.join("\n"))
}

/// The variables of `env` with `home` as the home of `agy` (`USERPROFILE`, `HOME`), and its own update off. Where the home
/// of the user's tools stays (its git config, Rust's) is the core's to add (`Agy::spawn`, its `tools_env`): a module knows
/// neither that home nor the machine's variables.
pub fn home_env(env: &[(String, String)], home: &Path) -> Vec<(String, String)> {
    let mut env: Vec<(String, String)> = env
        .iter()
        .filter(|(k, _)| !matches!(k.as_str(), HOME_VAR | "USERPROFILE" | "HOME"))
        .cloned()
        .collect();
    if !env.iter().any(|(k, _)| k == NO_SELF_UPDATE.0) {
        env.push((NO_SELF_UPDATE.0.into(), NO_SELF_UPDATE.1.into()));
    }
    // `agy` 1.5.3 writes in its temporary folder with no rule nor question (ESC-388): one of the agent's own.
    let tmp = temp_dir(home).to_string_lossy().into_owned();
    env.retain(|(k, _)| !TEMP_VARS.contains(&k.as_str()));
    for var in TEMP_VARS {
        env.push((var.into(), tmp.clone()));
    }
    let home = home.to_string_lossy().into_owned();
    env.push(("USERPROFILE".into(), home.clone()));
    env.push(("HOME".into(), home));
    env
}

/// The variables that name the temporary folder (Windows, then Unix).
pub const TEMP_VARS: [&str; 3] = ["TEMP", "TMP", "TMPDIR"];

/// The temporary folder of `agy` in the agent's home: what it writes there with no question touches no file of the user's.
pub fn temp_dir(home: &Path) -> PathBuf {
    home.join("tmp")
}

/// The file that makes `temp_dir` (the core writes a launch's files, their folder made).
pub fn temp_mark(home: &Path) -> PathBuf {
    temp_dir(home).join(".escouade")
}

/// What the card of a turn stopped by its limit says (`said`: `agy`'s own words).
pub fn limited_error(lang: &str, said: &str) -> String {
    if lang == "fr" {
        format!("Limite d’usage d’Antigravity atteinte : {said}")
    } else {
        format!("Antigravity’s usage limit reached: {said}")
    }
}

/// No conversation opened for the agent: what `agy` said.
pub fn session_refused(lang: &str, said: &str) -> String {
    if lang == "fr" {
        format!("Antigravity CLI n’a pas démarré la conversation de l’agent : {said}. S’il n’est pas connecté, lance agy dans un terminal pour te connecter à ton compte Google.")
    } else {
        format!("Antigravity CLI didn’t start the agent’s conversation: {said}. If it isn’t signed in, run agy in a terminal to sign in to your Google account.")
    }
}

/// A conversation that could not be resumed.
pub fn not_resumed(lang: &str) -> String {
    if lang == "fr" {
        "La session Antigravity n’a pas pu être reprise : nouvelle session (contexte vidé)".into()
    } else {
        "The Antigravity session couldn’t be resumed: new session (context cleared)".into()
    }
}
