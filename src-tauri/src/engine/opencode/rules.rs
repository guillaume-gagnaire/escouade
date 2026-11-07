//! What the core keeps of OpenCode's rules: the models as it lists them for an account, a model's two names. The agents' own
//! are its plugin's (`plugins/opencode/src/rules.rs`), which it does not link (docs/PLUGINS.md §8.2).

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[cfg(test)]
/// The name of Escouade's MCP server in its config and on its tools.
pub const SERVER: &str = "escouade";

/// The efforts Escouade's menu has, in its order: a model's `variants` are offered among them.
pub const EFFORTS: [&str; 5] = ["low", "medium", "high", "xhigh", "max"];

#[cfg(test)]
/// What the next prompt is sent with (OpenCode takes them at each prompt).
#[derive(Debug, Clone, Default)]
pub struct Settings {
    /// Escouade's mode.
    pub mode: String,
    /// The paths its tools are refused (« Projet privé »), as OpenCode names them: rules of its
    /// session, after its mode's (`rules`).
    pub fence: Vec<String>,
    /// A discussion of the Docs tab (ESC-261): its edits and commands refused, whatever its mode
    /// (`session_rules`).
    pub read_only: bool,
}

/// The provider and the model of `<provider>/<model>` (a model's id may hold more slashes).
pub fn split_model(model: &str) -> Option<(&str, &str)> {
    let (p, m) = model.split_once('/')?;
    (!p.is_empty() && !m.is_empty()).then_some((p, m))
}

#[cfg(test)]
/// OpenCode's agent of Escouade's mode: `plan` studies without changing anything, `build` works.
pub fn agent_of(mode: &str) -> &'static str {
    if mode == "plan" {
        "plan"
    } else {
        "build"
    }
}

#[cfg(test)]
/// The rules of a session in Escouade's mode (OpenCode has no sandbox: only they protect), as
/// Codex's modes are: the files of the agent's folder changed without a question, a command asked,
/// `git add` and `git commit` accepted at once in « Accepter les modifications ». The last rule that
/// matches wins, and a session merges them: each mode gives all of them.
pub fn rules(mode: &str, fence: &[String]) -> Value {
    let (bash, git, edit, web, outside) = match mode {
        "bypassPermissions" => ("allow", "allow", "allow", "allow", "allow"),
        "acceptEdits" => ("ask", "allow", "allow", "ask", "ask"),
        // The plan agent edits nothing but its plan.
        "plan" => ("ask", "ask", "ask", "ask", "ask"),
        _ => ("ask", "ask", "allow", "ask", "ask"),
    };
    let rule = |p: &str, pattern: &str, a: &str| json!({ "permission": p, "pattern": pattern, "action": a });
    let mut all = vec![
        rule("bash", "*", bash),
        rule("bash", "git add *", git),
        rule("bash", "git commit *", git),
        rule("edit", "*", edit),
        rule("webfetch", "*", web),
        rule("external_directory", "*", outside),
    ];
    all.extend(fence_rules(fence));
    Value::Array(all)
}

#[cfg(test)]
/// The rules of the session of an agent of `s`: its mode's, its fence's, and for a discussion
/// (ESC-261) every edit and command refused, last so that they win whatever came before. Refused by
/// OpenCode itself, never a card: Escouade's tools (an MCP server's) stay as they were.
pub fn session_rules(s: &Settings) -> Value {
    let mut all = rules(&s.mode, &s.fence);
    if s.read_only {
        if let Value::Array(a) = &mut all {
            for p in ["edit", "bash"] {
                a.push(json!({ "permission": p, "pattern": "*", "action": "deny" }));
            }
        }
    }
    all
}

#[cfg(test)]
/// « Projet privé » (ESC-140): each path of a fence refused to the tools that take a path (`read`,
/// `edit`, `list`, and `external_directory`, which OpenCode asks of any tool reaching out of the
/// agent's folder), and to a command that names it (`bash`, a guard: a command can reach it
/// without naming it). After the mode's rules: the last that matches wins, whatever the mode. Under
/// Windows, in both spellings (`\` and `/`): `*` of OpenCode's patterns is any character.
pub fn fence_rules(fence: &[String]) -> Vec<Value> {
    let rule =
        |p: &str, pattern: String| json!({ "permission": p, "pattern": pattern, "action": "deny" });
    let mut out = Vec::new();
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
    }
    out
}

#[cfg(test)]
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
}

/// `catalog` from what the server answered.
pub fn catalog(providers: &Value, config: &Value) -> Catalog {
    let chosen = config["model"].as_str();
    let mut models = Vec::new();
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
    Catalog { models }
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

#[cfg(test)]
/// What the server is given on top of the user's config (`OPENCODE_CONFIG_CONTENT`, merged with
/// it): Escouade's server kept out of every folder (an entry of the user's own included), each
/// agent's folder given its own by the API (`Session::give_escouade`), no share nor update of its own,
/// and the model of the agent that starts it, which OpenCode would otherwise pick itself (OpenCode
/// Zen without an account, tried).
pub fn config_content(model: &str) -> Value {
    let mut c = json!({
        "autoupdate": false,
        "share": "disabled",
        "mcp": { SERVER: { "enabled": false } },
        // The rules of « Demander » for every agent of the server: a subagent's session (`task`)
        // does not take those of its agent's (seen on the real run), and runs its commands without a
        // question by default. Each agent's session has its mode's on top (`rules`).
        "permission": {
            "bash": { "*": "ask", "git add *": "ask", "git commit *": "ask" },
            "edit": "allow",
            "webfetch": "ask",
            "external_directory": "ask",
        },
    });
    if split_model(model).is_some() {
        c["model"] = json!(model);
        c["small_model"] = json!(model);
    }
    c
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

    #[test]
    fn a_fenced_session_is_refused_each_path_whatever_its_mode() {
        // As OpenCode reads them: the last rule whose permission and pattern match, `*` any
        // character.
        fn matches(pattern: &str, what: &str) -> bool {
            let parts: Vec<&str> = pattern.split('*').collect();
            let mut rest = what;
            for (i, part) in parts.iter().enumerate() {
                if i == 0 {
                    let Some(r) = rest.strip_prefix(part) else {
                        return false;
                    };
                    rest = r;
                } else if i == parts.len() - 1 {
                    return rest.ends_with(part);
                } else {
                    let Some(at) = rest.find(part) else {
                        return false;
                    };
                    rest = &rest[at + part.len()..];
                }
            }
            rest.is_empty()
        }
        let fence = vec!["C:/code/secret".to_string()];
        let action = |mode: &str, p: &str, what: &str| {
            rules(mode, &fence)
                .as_array()
                .unwrap()
                .iter()
                .rev()
                .find(|r| r["permission"] == p && matches(r["pattern"].as_str().unwrap(), what))
                .map(|r| r["action"].as_str().unwrap().to_string())
                .unwrap()
        };
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
        let both = fence_rules(&[r"C:\code\secret".to_string()]);
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
            };
            assert_eq!(last(&s, "edit"), "deny", "{mode}");
            assert_eq!(last(&s, "bash"), "deny", "{mode}");
            // Reads, the web and the paths outside go as the mode says.
            assert_eq!(
                last(&s, "webfetch"),
                rules(mode, &[])[4]["action"],
                "{mode}"
            );
        }
        // Another agent: its mode's rules alone.
        let s = Settings {
            mode: "default".into(),
            ..Default::default()
        };
        assert_eq!(session_rules(&s), rules("default", &[]));
        assert_eq!(last(&s, "edit"), "allow");
    }

    #[test]
    fn each_mode_of_escouade_has_its_rules_and_agent_in_opencode() {
        // As OpenCode reads them: the last rule whose permission and pattern match.
        let action = |mode: &str, p: &str, what: &str| {
            rules(mode, &[])
                .as_array()
                .unwrap()
                .iter()
                .rev()
                .find(|r| {
                    let pattern = r["pattern"].as_str().unwrap();
                    r["permission"] == p
                        && (pattern == "*" || what.starts_with(pattern.trim_end_matches('*')))
                })
                .map(|r| r["action"].as_str().unwrap().to_string())
                .unwrap()
        };
        for (p, what) in [
            ("bash", "ls"),
            ("bash", "git add a"),
            ("edit", "a.py"),
            ("webfetch", "https://x"),
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
        // A subagent asks for its commands too.
        assert_eq!(c["permission"]["bash"]["*"], "ask");
        assert_eq!(c["permission"]["external_directory"], "ask");
        assert!(config_content("sonnet").get("model").is_none());
        assert_eq!(split_model("openrouter/a/b"), Some(("openrouter", "a/b")));
        assert_eq!(split_model("sonnet"), None);
    }
}
