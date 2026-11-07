//! What Escouade asks of OpenCode, without a server: the rules of a session in each mode (OpenCode has no sandbox: only
//! they protect), the fence of a private project and read-only as rules, the config a server starts with, the answers of a
//! question's card, and the models as OpenCode lists them.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// The name of Escouade's MCP server in its config and on its tools.
pub const SERVER: &str = "escouade";

/// The efforts Escouade's menu has, in its order: a model's `variants` are offered among them.
pub const EFFORTS: [&str; 5] = ["low", "medium", "high", "xhigh", "max"];

/// What the next prompt is sent with (OpenCode takes them at each prompt).
#[derive(Debug, Clone, Default)]
pub struct Settings {
    /// `<provider>/<model>` (`ollama/qwen3-coder:30b-32k`).
    pub model: String,
    /// The model's variant (`low`…), when it has it.
    pub effort: String,
    /// Escouade's mode.
    pub mode: String,
    /// The paths its tools are refused (« Projet privé »), as OpenCode names them: rules of its
    /// session, after its mode's (`rules`).
    pub fence: Vec<String>,
    /// A discussion of the Docs tab (ESC-261): its edits and commands refused, whatever its mode
    /// (`session_rules`).
    pub read_only: bool,
    /// What OpenCode judges an edit against (`/` out of a repository), read from the server under
    /// a fence: `relative`.
    pub worktree: Option<String>,
}

/// The provider and the model of `<provider>/<model>` (a model's id may hold more slashes).
pub fn split_model(model: &str) -> Option<(&str, &str)> {
    let (p, m) = model.split_once('/')?;
    (!p.is_empty() && !m.is_empty()).then_some((p, m))
}

/// OpenCode's agent of Escouade's mode: `plan` studies without changing anything, `build` works.
pub fn agent_of(mode: &str) -> &'static str {
    if mode == "plan" {
        "plan"
    } else {
        "build"
    }
}

/// The rules of a session in Escouade's mode (OpenCode has no sandbox: only they protect), as
/// Codex's modes are: the files of the agent's folder changed without a question, a command asked,
/// `git add` and `git commit` accepted at once in « Accepter les modifications » as long as they are
/// plain (`git_rules`), and what git or OpenCode run stays on a card (`KEPT`). The last rule that
/// matches wins, and a session adds them to those it has: each mode gives all of them.
pub fn rules(mode: &str, fence: &[String], worktree: Option<&str>) -> Value {
    let (bash, git, edit, web, outside) = match mode {
        "bypassPermissions" => ("allow", "allow", "allow", "allow", "allow"),
        "acceptEdits" => ("ask", "allow", "allow", "ask", "ask"),
        // The plan agent edits nothing but its plan.
        "plan" => ("ask", "ask", "ask", "ask", "ask"),
        _ => ("ask", "ask", "allow", "ask", "ask"),
    };
    // Only « Sans permissions » lets them go.
    let guard = if mode == "bypassPermissions" {
        "allow"
    } else {
        "ask"
    };
    let mut all = vec![rule("bash", "*", bash)];
    all.extend(git_rules(git, guard));
    all.push(rule("edit", "*", edit));
    all.extend(kept_paths().map(|p| rule("edit", &p, guard)));
    all.push(rule("webfetch", "*", web));
    all.push(rule("websearch", "*", web));
    all.push(rule("external_directory", "*", outside));
    all.extend(fence_rules(fence, worktree));
    Value::Array(all)
}

fn rule(permission: &str, pattern: &str, action: &str) -> Value {
    json!({ "permission": permission, "pattern": pattern, "action": action })
}

/// The options that leave `git add` or `git commit` plain, when they come first.
const PLAIN: [(&str, &[&str]); 2] = [
    ("git add", &["-A", "-u", "--all", "--update"]),
    ("git commit", &["-m", "-am"]),
];

/// What makes such a command more than it says, read on its text. OpenCode's own reading of it sees
/// `;`, `|`, `&`, a new line, `$()` and `workdir`, not this: an option after the first (`-F`,
/// `--file`, `--template`, `--pathspec-from-file` read any file, glued in `-avF`, shortened to
/// `--fil`), an option written so that no pattern reads it (quoted, escaped: OpenCode's matcher takes
/// `\` for `/`, and its `*` covers new lines), a redirection, and what a shell would expand into an
/// option (`X=-F; git commit $X` shows no `-F`), in bash, PowerShell or cmd, whichever it runs.
const GUARDED: [&str; 13] = [
    "* -*", "*\t*", "*\"-*", "*'-*", "*/-*", "*/\n*", "*>*", "*<*", "*$*", "*`*", "*{*", "*^*",
    "*%*",
];

/// `git add` and `git commit`: `git` when plain, `guard` for any other form, an option not in `PLAIN`
/// or anything of `GUARDED`. The plain forms are listed, not the dangerous ones: the matcher cannot
/// say « a word that ends in F », and ignores case, so a rule for `-avF` would also take every
/// `-m "fix`.
fn git_rules(git: &str, guard: &str) -> Vec<Value> {
    let mut out = Vec::new();
    for (command, plain) in PLAIN {
        out.push(rule("bash", &format!("{command} *"), git));
        out.push(rule("bash", &format!("{command} -*"), guard));
        for option in plain {
            out.push(rule("bash", &format!("{command} {option} *"), git));
        }
        for tail in GUARDED {
            out.push(rule("bash", &format!("{command} {tail}"), guard));
        }
    }
    out
}

/// What git or OpenCode run or take as rules, whatever the file is called on (ESC-364): a hook or a
/// `.git/config` at the next `git commit`, a file of `.opencode` (a plugin, a tool, an agent) or an
/// `opencode.json` (`plugin`, `mcp`, `permission`) at the next start of the folder's instance.
/// `.husky` and `.githooks` are where `core.hooksPath` points by habit. The last two are the short
/// names of Windows, which reach `.git` and `.opencode` as well (`GIT~1/hooks/x`, tried).
const KEPT: [&str; 8] = [
    ".git",
    ".opencode",
    "opencode.json",
    "opencode.jsonc",
    ".husky",
    ".githooks",
    "git*~*",
    "openco*~*",
];

/// `KEPT` as a file or a folder, at the root or below it (out of a repository OpenCode judges from
/// `/`: all is below), and any `:`, which no relative path has but a stream of Windows
/// (`.git::$INDEX_ALLOCATION/hooks/x` reaches `.git`, tried; `.git./` and `.git /` do not, OpenCode
/// makes such a folder as it is spelled).
fn kept_paths() -> impl Iterator<Item = String> {
    KEPT.iter()
        .flat_map(|p| {
            [
                p.to_string(),
                format!("{p}/*"),
                format!("*/{p}"),
                format!("*/{p}/*"),
            ]
        })
        .chain(["*:*".to_string()])
}

/// The rules of the session of an agent of `s`: its mode's, its fence's, and for a discussion
/// (ESC-261) every edit and command refused, last so that they win whatever came before. Refused by
/// OpenCode itself, never a card: Escouade's tools (an MCP server's) stay as they were.
pub fn session_rules(s: &Settings) -> Value {
    let mut all = rules(&s.mode, &s.fence, s.worktree.as_deref());
    if s.read_only {
        if let Value::Array(a) = &mut all {
            for p in ["edit", "bash"] {
                a.push(json!({ "permission": p, "pattern": "*", "action": "deny" }));
            }
        }
    }
    all
}

/// « Projet privé » (ESC-140): each path of a fence refused to the tools that take a path (`read`,
/// `edit`, `list`, and `external_directory`, which OpenCode asks of any tool reaching out of the
/// agent's folder), and to a command that names it (`bash`, a guard: a command can reach it
/// without naming it). After the mode's rules: the last that matches wins, whatever the mode. Under
/// Windows, in both spellings (`\` and `/`): `*` of OpenCode's patterns is any character.
///
/// OpenCode judges `read` and `list` by the absolute path, `edit` by the path from `worktree`: for a
/// fence inside it, which `external_directory` does not stop, `edit` is held in that spelling
/// (`relative`). `git add` and `git commit` read a file of their own, which no rule of a path sees:
/// `git commit -F ../prive/x` is refused, in « Sans permissions » as in the others.
pub fn fence_rules(fence: &[String], worktree: Option<&str>) -> Vec<Value> {
    let rule =
        |p: &str, pattern: String| json!({ "permission": p, "pattern": pattern, "action": "deny" });
    let mut out = Vec::new();
    if !fence.is_empty() {
        let forms = READS_A_FILE.iter().chain(&ADD_READS_A_FILE);
        out.extend(forms.map(|form| rule("bash", form.to_string())));
    }
    for path in fence {
        let mut spellings = vec![path.clone()];
        if path.contains('\\') {
            spellings.push(path.replace('\\', "/"));
        }
        for p in &spellings {
            let sep = if p.contains('\\') { '\\' } else { '/' };
            for permission in ["read", "edit", "list", "external_directory"] {
                out.push(rule(permission, p.clone()));
                out.push(rule(permission, format!("{p}{sep}*")));
            }
            out.push(rule("bash", format!("*{p}*")));
        }
        if let Some(rel) = worktree.and_then(|w| relative(w, path)) {
            out.push(rule("edit", rel.clone()));
            out.push(rule("edit", format!("{rel}/*")));
            // A word alone (`secret`) would refuse every message that says it.
            let named = if rel.contains('/') {
                format!("*{rel}*")
            } else {
                format!("*{rel}/*")
            };
            out.push(rule("bash", named));
        }
    }
    out
}

/// How `git add` and `git commit` read a file: `-F` and `-t`, alone or after one flag, `--file`,
/// `--template`, `--pathspec-from-file`, shortened as git allows them, first or further. A rule for
/// longer clusters would refuse every `-m "fix`; the fence's own rules still see the path they name.
const READS_A_FILE: [&str; 14] = [
    "git commit -F*",
    "git commit * -F*",
    "git commit -?F*",
    "git commit * -?F*",
    "git commit --fil*",
    "git commit * --fil*",
    "git commit -t*",
    "git commit * -t*",
    "git commit -?t*",
    "git commit * -?t*",
    "git commit --te*",
    "git commit * --te*",
    "git commit --path*",
    "git commit * --path*",
];

/// `git add` reads a file by one option: its `-f` is another thing.
const ADD_READS_A_FILE: [&str; 2] = ["git add --path*", "git add * --path*"];

/// `to` as Node's `path.relative(from, to)` spells it, which is how OpenCode names an edit, in `/`
/// (its matcher takes `\` for `/`). `from` may be `/`, the worktree of a folder out of any repository.
/// None on another drive of Windows: Node then gives the absolute path, which the rules already say.
/// By hand: a module's `Path` says WASI, even under Windows.
pub fn relative(from: &str, to: &str) -> Option<String> {
    fn split(path: &str) -> (Option<char>, Vec<String>) {
        let path = path.replace('\\', "/");
        let drive = match path.as_bytes() {
            [d, b':', ..] if d.is_ascii_alphabetic() => Some(d.to_ascii_lowercase() as char),
            _ => None,
        };
        let rest = if drive.is_some() { &path[2..] } else { &path };
        let parts = rest
            .split('/')
            .filter(|part| !part.is_empty() && *part != ".")
            .map(str::to_string)
            .collect();
        (drive, parts)
    }
    let (from_drive, from) = split(from);
    let (to_drive, to) = split(to);
    if from_drive.is_some() && from_drive != to_drive {
        return None;
    }
    let windows = to_drive.is_some();
    let common = from
        .iter()
        .zip(&to)
        .take_while(|(a, b)| a == b || (windows && a.eq_ignore_ascii_case(b)))
        .count();
    let mut out = vec!["..".to_string(); from.len() - common];
    out.extend_from_slice(&to[common..]);
    (!out.is_empty()).then(|| out.join("/"))
}

/// The id of the card of a plan to approve, its turn over on it (`translate`).
pub fn plan_approval(message: &str) -> String {
    format!("plan:{message}")
}

pub fn is_plan_approval(request_id: &str) -> bool {
    request_id.starts_with("plan:")
}

/// The answers of a question's card (`answers`, by question as Escouade keeps them) as
/// `POST /question/:id/reply` takes them: for each question in order, the labels chosen.
pub fn answers(questions: &Value, given: &Value) -> Value {
    let all: Vec<Value> = questions
        .as_array()
        .into_iter()
        .flatten()
        .map(|q| {
            let text = q["question"].as_str().unwrap_or_default();
            let labels: Vec<String> = match &given[text] {
                Value::String(s) => {
                    // Several chosen, as the card joins them.
                    let options: Vec<&str> = q["options"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|o| o["label"].as_str())
                        .collect();
                    let parts: Vec<&str> = s.split(", ").collect();
                    if q["multiSelect"] == true
                        && parts.len() > 1
                        && parts.iter().all(|p| options.contains(p))
                    {
                        parts.into_iter().map(str::to_string).collect()
                    } else {
                        vec![s.clone()]
                    }
                }
                Value::Array(a) => a
                    .iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect(),
                _ => Vec::new(),
            };
            json!(labels)
        })
        .collect();
    Value::Array(all)
}

/// A model as OpenCode lists it: the fields of the core's `ToldModel` it has.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Told {
    pub id: String,
    pub label: String,
    pub efforts: Vec<String>,
    pub default_effort: Option<String>,
    pub images: bool,
    pub default: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

/// The models of the connected providers (`GET /config/providers`), the user's default one
/// (`model` of `GET /config`) marked, each with its variants.
pub struct Catalog {
    pub models: Vec<Told>,
    /// The context window of each model (`limit.context`), by its id, when OpenCode knows it.
    pub contexts: std::collections::BTreeMap<String, u64>,
}

/// `catalog` from what the server answered.
pub fn catalog(providers: &Value, config: &Value) -> Catalog {
    let chosen = config["model"].as_str();
    let mut models = Vec::new();
    let mut contexts = std::collections::BTreeMap::new();
    for p in providers["providers"].as_array().into_iter().flatten() {
        let Some(pid) = p["id"].as_str() else {
            continue;
        };
        let pname = p["name"].as_str().unwrap_or(pid).to_string();
        let Some(list) = p["models"].as_object() else {
            continue;
        };
        let mut ids: Vec<&String> = list.keys().collect();
        ids.sort();
        for mid in ids {
            let m = &list[mid];
            if m["status"] == "deprecated" {
                continue;
            }
            let mut efforts: Vec<String> = m["variants"]
                .as_object()
                .map(|v| v.keys().cloned().collect())
                .unwrap_or_default();
            efforts.sort_by_key(|e| EFFORTS.iter().position(|x| x == e).unwrap_or(EFFORTS.len()));
            if let Some(n) = m["limit"]["context"].as_u64().filter(|n| *n > 0) {
                contexts.insert(format!("{pid}/{mid}"), n);
            }
            models.push(Told {
                id: format!("{pid}/{mid}"),
                label: m["name"].as_str().unwrap_or(mid).to_string(),
                efforts,
                default_effort: None,
                images: m["capabilities"]["input"]["image"] == true,
                default: false,
                provider: Some(pname.clone()),
            });
        }
    }
    // The default model of the first provider, in OpenCode's order.
    let first_default = providers["providers"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|p| p["id"].as_str())
        .find_map(|p| providers["default"][p].as_str().map(|m| format!("{p}/{m}")));
    let default = chosen
        .filter(|c| models.iter().any(|m| m.id == *c))
        .map(str::to_string)
        .or(first_default.filter(|d| models.iter().any(|m| m.id == *d)))
        .or_else(|| models.first().map(|m| m.id.clone()));
    for m in &mut models {
        m.default = Some(&m.id) == default.as_ref();
    }
    Catalog { models, contexts }
}

/// The models `opencode models --verbose` lists (`<provider>/<model>`, then its JSON), as
/// `catalog` reads those of the server: its first one the default.
pub fn listed(text: &str) -> Catalog {
    let mut providers: Vec<(String, serde_json::Map<String, Value>)> = Vec::new();
    let mut current: Option<(String, String, String)> = None;
    // The first listed: its default (a map keeps its keys sorted).
    let mut first: Option<(String, String)> = None;
    let flush = |cur: &mut Option<(String, String, String)>,
                 providers: &mut Vec<(String, serde_json::Map<String, Value>)>| {
        if let Some((p, m, json)) = cur.take() {
            let model = serde_json::from_str::<Value>(&json).unwrap_or_else(|_| json!({}));
            match providers.iter_mut().find(|(id, _)| *id == p) {
                Some((_, models)) => {
                    models.insert(m, model);
                }
                None => {
                    let mut models = serde_json::Map::new();
                    models.insert(m, model);
                    providers.push((p, models));
                }
            }
        }
    };
    for line in text.lines() {
        let head = line.trim_end();
        let starts = !head.starts_with([' ', '{', '}', '"', '\t']) && !head.is_empty();
        match (starts, split_model(head)) {
            (true, Some((p, m))) if !head.contains(' ') => {
                flush(&mut current, &mut providers);
                first.get_or_insert_with(|| (p.to_string(), m.to_string()));
                current = Some((p.to_string(), m.to_string(), String::new()));
            }
            _ => {
                if let Some((_, _, json)) = current.as_mut() {
                    json.push_str(line);
                    json.push('\n');
                }
            }
        }
    }
    flush(&mut current, &mut providers);
    let list: Vec<Value> = providers
        .into_iter()
        .map(|(id, models)| json!({ "id": id, "name": id, "models": models }))
        .collect();
    let mut default = serde_json::Map::new();
    if let Some((p, m)) = first {
        default.insert(p, json!(m));
    }
    catalog(
        &json!({ "providers": list, "default": default }),
        &Value::Null,
    )
}

/// What the server is given on top of the user's config (`OPENCODE_CONFIG_CONTENT`, merged with
/// it): Escouade's server kept out of every folder (an entry of the user's own included), each
/// agent's folder given its own by the API (`Session::give_escouade`), no share nor update of its own,
/// and the model of the agent that starts it, which OpenCode would otherwise pick itself (OpenCode
/// Zen without an account, tried).
pub fn config_content(model: &str) -> Value {
    let mut edit = serde_json::Map::new();
    edit.insert("*".to_string(), json!("allow"));
    edit.extend(kept_paths().map(|p| (p, json!("ask"))));
    let mut c = json!({
        "autoupdate": false,
        "share": "disabled",
        "mcp": { SERVER: { "enabled": false } },
        // The rules of « Demander » for every agent of the server: a subagent's session (`task`)
        // does not take those of its agent's (seen on the real run), and runs its commands without a
        // question by default. Each agent's session has its mode's on top (`rules`).
        "permission": {
            "bash": { "*": "ask", "git add *": "ask", "git commit *": "ask" },
            "edit": edit,
            "webfetch": "ask",
            "websearch": "ask",
            "external_directory": "ask",
        },
    });
    if split_model(model).is_some() {
        c["model"] = json!(model);
        c["small_model"] = json!(model);
    }
    c
}

/// Escouade's MCP entry of an agent (`POST /mcp`): its server, reached with the agent's token.
pub fn entry(url: &str, token: &str) -> Value {
    json!({
        "name": SERVER,
        "config": {
            "type": "remote",
            "url": url,
            "enabled": true,
            "headers": { "Authorization": format!("Bearer {token}") },
            // Its own: no OAuth to look for.
            "oauth": false,
        },
    })
}

/// `tr(lang, fr, en)`: the text in the app's language (`fr`, else English).
pub(crate) fn tr(lang: &str, fr: String, en: String) -> String {
    if lang == "en" {
        en
    } else {
        fr
    }
}

/// No model of a connected provider to send a message with.
pub fn no_model(lang: &str) -> String {
    tr(
        lang,
        "Aucun modèle d’OpenCode à utiliser : connecte un fournisseur à OpenCode (opencode auth login) ou déclare-en un dans sa config, puis choisis son modèle.".into(),
        "No OpenCode model to use: connect a provider to OpenCode (opencode auth login) or declare one in its config, then pick its model.".into(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Wildcard.match` of OpenCode 1.19.3: any case, `\` read as `/`, `*` any character, new lines
    /// too, `?` one, and a last ` *` optional (`git add *` takes `git add`). The cases of
    /// `rules_verdicts.json` are played on the real one by `scripts/opencode-real-rules.mjs`.
    fn opencode_matches(pattern: &str, what: &str) -> bool {
        fn glob(p: &[char], w: &[char]) -> bool {
            match p.first() {
                None => w.is_empty(),
                Some('*') => (0..=w.len()).any(|i| glob(&p[1..], &w[i..])),
                Some('?') => !w.is_empty() && glob(&p[1..], &w[1..]),
                Some(c) => w.first() == Some(c) && glob(&p[1..], &w[1..]),
            }
        }
        let read = |s: &str| -> Vec<char> { s.replace('\\', "/").to_lowercase().chars().collect() };
        let pattern = read(pattern);
        let what = read(what);
        if let [head @ .., ' ', '*'] = pattern.as_slice() {
            if glob(head, &what) {
                return true;
            }
        }
        glob(&pattern, &what)
    }

    /// The action of the last rule that matches, as OpenCode takes it.
    fn decides(rules: &Value, permission: &str, what: &str) -> String {
        rules
            .as_array()
            .unwrap()
            .iter()
            .rev()
            .find(|r| {
                r["permission"] == permission
                    && opencode_matches(r["pattern"].as_str().unwrap(), what)
            })
            .map(|r| r["action"].as_str().unwrap().to_string())
            .unwrap_or_else(|| panic!("no rule of {permission} for {what}"))
    }

    fn verdicts() -> Value {
        serde_json::from_str(include_str!("rules_verdicts.json")).unwrap()
    }

    /// The rules of a case, its folder `wt` and the one above it `up`.
    fn case_rules(mode: &str, fenced: bool, wt: &str, up: &str) -> Value {
        let fence: Vec<String> = if fenced {
            verdicts()["fence"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| p.as_str().unwrap().replace("{wt}", wt).replace("{up}", up))
                .collect()
        } else {
            Vec::new()
        };
        session_rules(&Settings {
            mode: mode.into(),
            fence,
            worktree: Some(wt.to_string()),
            ..Default::default()
        })
    }

    #[test]
    fn the_rules_of_each_mode_do_what_the_real_opencode_was_seen_to_do() {
        for case in verdicts()["cases"].as_array().unwrap() {
            let rules = case_rules(
                case["mode"].as_str().unwrap(),
                case["fenced"] == true,
                "C:/code/proj",
                "C:/code",
            );
            let (permission, what) = match case["edit"].as_str() {
                Some(path) => ("edit", path),
                None => (
                    "bash",
                    case["seen"].as_str().or(case["bash"].as_str()).unwrap(),
                ),
            };
            assert_eq!(
                decides(&rules, permission, what),
                case["want"].as_str().unwrap(),
                "{case}"
            );
        }
    }

    #[test]
    #[ignore = "writes what scripts/opencode-real-rules.mjs plays on the real OpenCode"]
    fn dump_the_rules_for_the_real_opencode() {
        let path = std::env::var("ESCOUADE_RULES_OUT").unwrap();
        let wt = std::env::var("ESCOUADE_RULES_WT").unwrap();
        let up = std::env::var("ESCOUADE_RULES_UP").unwrap();
        let mut out = serde_json::Map::new();
        for mode in ["default", "acceptEdits", "plan", "bypassPermissions"] {
            out.insert(mode.into(), case_rules(mode, false, &wt, &up));
            out.insert(format!("{mode}+fence"), case_rules(mode, true, &wt, &up));
        }
        out.insert("config".into(), config_content("fake/m"));
        std::fs::write(path, Value::Object(out).to_string()).unwrap();
    }

    #[test]
    fn a_path_is_spelled_from_the_worktree_as_node_does() {
        let r = |from, to| relative(from, to);
        assert_eq!(
            r("C:/code/proj", "C:/code/proj/secret").as_deref(),
            Some("secret")
        );
        assert_eq!(
            r(r"C:\code\proj", r"C:\code\prive\x").as_deref(),
            Some("../prive/x")
        );
        // Windows ignores case, a drive above all; a system of `/` does not.
        assert_eq!(
            r(r"c:\Code\proj", r"C:\code\prive").as_deref(),
            Some("../prive")
        );
        assert_eq!(
            r("/code/Proj", "/code/proj/x").as_deref(),
            Some("../proj/x")
        );
        // Out of a repository OpenCode says `/`, and the path is from the root of its drive.
        assert_eq!(r("/", r"C:\tmp\plain\a").as_deref(), Some("tmp/plain/a"));
        assert_eq!(r("/", "/tmp/plain").as_deref(), Some("tmp/plain"));
        // Another drive: Node gives the absolute path, already in the rules.
        assert_eq!(r("C:/code", "D:/code"), None);
        assert_eq!(r("C:/code", "C:/code"), None);
    }

    #[test]
    fn a_fence_inside_the_worktree_is_refused_to_edits_as_opencode_names_them() {
        let fence = vec![r"C:\code\proj\secret".to_string()];
        let all = Value::Array(fence_rules(&fence, Some(r"C:\code\proj")));
        assert_eq!(decides(&all, "edit", r"secret\a.txt"), "deny");
        assert_eq!(decides(&all, "edit", "secret"), "deny");
        // Without it the rules say the absolute path alone, which OpenCode never asks of `edit`.
        let absolute = Value::Array(fence_rules(&fence, None));
        assert!(absolute
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["permission"] == "edit")
            .all(|r| !opencode_matches(r["pattern"].as_str().unwrap(), "secret/a.txt")));
    }

    #[test]
    fn a_fenced_session_is_refused_each_path_whatever_its_mode() {
        let fence = vec!["C:/code/secret".to_string()];
        let action = |mode: &str, p: &str, what: &str| decides(&rules(mode, &fence, None), p, what);
        for mode in ["default", "acceptEdits", "plan", "bypassPermissions"] {
            for p in ["read", "edit", "list", "external_directory"] {
                assert_eq!(action(mode, p, "C:/code/secret"), "deny", "{mode} {p}");
                assert_eq!(
                    action(mode, p, "C:/code/secret/a.txt"),
                    "deny",
                    "{mode} {p}"
                );
            }
            assert_eq!(
                action(mode, "bash", "cat C:/code/secret/a.txt"),
                "deny",
                "{mode}"
            );
        }
        // The rest as the mode says.
        assert_eq!(
            action("bypassPermissions", "external_directory", "C:/code/other"),
            "allow"
        );
        assert_eq!(action("default", "bash", "ls"), "ask");
        // A path of Windows in both spellings.
        let both = fence_rules(&[r"C:\code\secret".to_string()], None);
        let patterns: Vec<&str> = both
            .iter()
            .filter(|r| r["permission"] == "read")
            .map(|r| r["pattern"].as_str().unwrap())
            .collect();
        assert_eq!(
            patterns,
            [
                r"C:\code\secret",
                r"C:\code\secret\*",
                "C:/code/secret",
                "C:/code/secret/*"
            ]
        );
    }

    #[test]
    fn a_discussion_has_its_edits_and_commands_refused_in_every_mode() {
        // As OpenCode reads them: the last rule of the permission (patterns `*` here).
        let last = |s: &Settings, p: &str| {
            session_rules(s)
                .as_array()
                .unwrap()
                .iter()
                .rev()
                .find(|r| r["permission"] == p)
                .map(|r| r["action"].as_str().unwrap().to_string())
                .unwrap()
        };
        for mode in ["default", "acceptEdits", "plan", "bypassPermissions"] {
            let s = Settings {
                mode: mode.into(),
                fence: vec!["C:/code/secret".into()],
                read_only: true,
                ..Default::default()
            };
            assert_eq!(last(&s, "edit"), "deny", "{mode}");
            assert_eq!(last(&s, "bash"), "deny", "{mode}");
            // Reads, the web and the paths outside go as the mode says.
            assert_eq!(
                last(&s, "webfetch"),
                rules(mode, &[], None)
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["permission"] == "webfetch")
                    .unwrap()["action"],
                "{mode}"
            );
        }
        // Another agent: its mode's rules alone.
        let s = Settings {
            mode: "default".into(),
            ..Default::default()
        };
        assert_eq!(session_rules(&s), rules("default", &[], None));
        assert_eq!(decides(&session_rules(&s), "edit", "a.py"), "allow");
    }

    #[test]
    fn each_mode_of_escouade_has_its_rules_and_agent_in_opencode() {
        let action = |mode: &str, p: &str, what: &str| decides(&rules(mode, &[], None), p, what);
        for (p, what) in [
            ("bash", "ls"),
            ("bash", "git add a"),
            ("edit", "a.py"),
            ("webfetch", "https://x"),
            ("websearch", "rust"),
            ("external_directory", "/x"),
        ] {
            assert_eq!(action("bypassPermissions", p, what), "allow", "{p}");
            assert_eq!(action("plan", p, what), "ask", "{p}");
        }
        assert_eq!(action("default", "edit", "a.py"), "allow");
        assert_eq!(action("default", "bash", "git commit -m x"), "ask");
        assert_eq!(action("acceptEdits", "bash", "git add a"), "allow");
        assert_eq!(action("acceptEdits", "bash", "git commit -m x"), "allow");
        assert_eq!(action("acceptEdits", "bash", "rm -rf build"), "ask");
        assert_eq!(action("default", "external_directory", "/x"), "ask");
        assert_eq!(action("default", "websearch", "rust"), "ask");
        assert_eq!(action("acceptEdits", "websearch", "rust"), "ask");
        assert_eq!(agent_of("plan"), "plan");
        assert_eq!(agent_of("acceptEdits"), "build");
    }

    #[test]
    fn the_answers_of_a_card_go_back_in_the_order_of_the_questions() {
        let questions = json!([
            { "question": "Quel nom ?", "options": [{ "label": "a" }, { "label": "b" }] },
            { "question": "Quels systèmes ?", "multiSelect": true,
              "options": [{ "label": "Linux" }, { "label": "macOS" }] },
        ]);
        let given = json!({ "Quels systèmes ?": "Linux, macOS", "Quel nom ?": "a" });
        assert_eq!(
            answers(&questions, &given),
            json!([["a"], ["Linux", "macOS"]])
        );
    }

    #[test]
    fn the_listing_of_opencode_models_gives_the_models_and_its_first_as_default() {
        let text = "ollama/qwen3-coder:30b-32k\n{\n  \"id\": \"qwen3-coder:30b-32k\",\n  \"name\": \"Qwen3 Coder\"\n}\nollama/gpt-oss:20b-32k\n{\n  \"name\": \"gpt-oss\",\n  \"variants\": { \"low\": {}, \"high\": {} }\n}\nopenrouter/a/b\n{\n  \"name\": \"B\"\n}\n";
        let c = listed(text);
        let ids: Vec<&str> = c.models.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "ollama/gpt-oss:20b-32k",
                "ollama/qwen3-coder:30b-32k",
                "openrouter/a/b"
            ]
        );
        assert_eq!(c.models[0].efforts, ["low", "high"]);
        assert!(c.models[1].default, "the first listed");
        assert!(listed("").models.is_empty());
    }

    #[test]
    fn the_config_always_gives_a_model_and_keeps_escouades_server_out() {
        let c = config_content("ollama/qwen3-coder:30b-32k");
        assert_eq!(c["model"], "ollama/qwen3-coder:30b-32k");
        assert_eq!(c["small_model"], c["model"]);
        assert_eq!(c["mcp"]["escouade"]["enabled"], false);
        // A subagent asks for its commands too, and takes `edit` of the config alone: `.git` stays
        // on a card there as well.
        assert_eq!(c["permission"]["bash"]["*"], "ask");
        assert_eq!(c["permission"]["external_directory"], "ask");
        assert_eq!(c["permission"]["websearch"], "ask");
        let edit: Vec<Value> = c["permission"]["edit"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(pattern, action)| rule("edit", pattern, action.as_str().unwrap()))
            .collect();
        let edit = Value::Array(edit);
        assert_eq!(decides(&edit, "edit", "src/a.rs"), "allow");
        assert_eq!(decides(&edit, "edit", ".git/hooks/pre-commit"), "ask");
        assert_eq!(decides(&edit, "edit", ".opencode/plugin/x.ts"), "ask");
        assert_eq!(decides(&edit, "edit", "sub/opencode.json"), "ask");
        assert!(config_content("sonnet").get("model").is_none());
        assert_eq!(split_model("openrouter/a/b"), Some(("openrouter", "a/b")));
        assert_eq!(split_model("sonnet"), None);
    }
}
