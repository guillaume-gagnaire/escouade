//! What the « Workspace » shows of an agent without loading its conversation (spec
//! docs/superpowers/specs/2026-11-25-vue-workspace-design.md, « D'où viennent les données ») : the
//! skills and plugins it used, its own and each of its subagents', with their calls and the ones
//! running. Fed by `agent.rs` at each tool call and result, as the plan; it goes out with the agent
//! (`AgentMeta::canvas`). Never reset: it says what the agent used in its life, its caps bound it.

use crate::plan::{Change, Seen};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// The most skills and plugins a thread (the agent, a subagent) keeps: the least recent go.
pub const MAX_KIT: usize = 40;
/// The most subagent threads kept: the first come go.
pub const MAX_THREADS: usize = 30;
/// The most artifacts kept: the least recently published go.
pub const MAX_ARTIFACTS: usize = 20;
/// The most terminal calls a thread keeps: the oldest go.
pub const MAX_COMMANDS: usize = 5;
/// The most characters of a command kept.
pub const MAX_COMMAND: usize = 300;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KitKind {
    /// A call of the tool `Skill`.
    #[default]
    Skill,
    /// A server of tools: the `<server>` of `mcp__<server>__<tool>`.
    Plugin,
}

/// A skill or a plugin a thread used.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, from = "StoredKitUse")]
pub struct KitUse {
    pub kind: KitKind,
    pub name: String,
    pub calls: u32,
    /// Its calls running now: none survives a reload (`StoredKitUse`).
    pub running: u32,
    /// Its first call.
    pub first_at: i64,
    pub last_at: i64,
}

/// A `KitUse` as saved: one saved before its first call was kept (no `firstAt`) starts at its last.
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct StoredKitUse {
    kind: KitKind,
    name: String,
    calls: u32,
    first_at: i64,
    last_at: i64,
}

impl From<StoredKitUse> for KitUse {
    fn from(s: StoredKitUse) -> Self {
        KitUse {
            kind: s.kind,
            name: s.name,
            calls: s.calls,
            running: 0,
            first_at: if s.first_at == 0 {
                s.last_at
            } else {
                s.first_at
            },
            last_at: s.last_at,
        }
    }
}

/// The skills and plugins of a subagent, by its call.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ThreadKit {
    pub call: String,
    pub tools: Vec<KitUse>,
    /// Its latest terminal calls.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub commands: Vec<CommandRun>,
}

/// A page the agent published with the tool `Artifact`, by its address.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Artifact {
    pub url: String,
    pub title: String,
    /// The publications seen at this address.
    pub versions: u32,
    /// The last one.
    pub at: i64,
}

/// An `Artifact` publication running: its address comes with its result.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Publishing {
    pub call: String,
    pub name: String,
    pub since: i64,
}

/// A terminal call (`Bash`, in the foreground): the window tells the test runs among them.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CommandRun {
    pub call: String,
    pub command: String,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub failed: bool,
    /// Cut short by the end of the process (a restart): neither passed nor failed.
    pub stopped: bool,
}

/// The first address of a published artifact in `text`, as `ARTIFACT_URL` of `src/lib/tools.ts`.
pub fn artifact_url(text: &str) -> Option<String> {
    let id_char = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-';
    for (at, _) in text.match_indices("https://") {
        let before = text[..at].chars().next_back();
        if before.is_some_and(|c| c.is_alphanumeric() || "_./-".contains(c)) {
            continue;
        }
        let rest = &text[at + "https://".len()..];
        let rest = match rest.split_once('.') {
            // One label of subdomain before `claude.ai`.
            Some((sub, tail))
                if !sub.is_empty()
                    && sub != "claude"
                    && sub
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') =>
            {
                tail
            }
            _ => rest,
        };
        let Some(id) = rest.strip_prefix("claude.ai/artifact/") else {
            continue;
        };
        let len = id.find(|c: char| !id_char(c)).unwrap_or(id.len());
        if len > 0 {
            let end = text.len() - id.len() + len;
            return Some(text[at..end].to_string());
        }
    }
    None
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Canvas {
    /// The agent's own.
    pub tools: Vec<KitUse>,
    /// Each subagent's, in the order they came.
    pub subs: Vec<ThreadKit>,
    /// The pages published, by any thread: the deliverables of the agent.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<Artifact>,
    /// The publications running.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub publishing: Vec<Publishing>,
    /// The agent's own latest terminal calls.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub commands: Vec<CommandRun>,
    /// The latest calls looked at: a frame that comes twice counts once.
    #[serde(skip)]
    seen: Seen,
    /// The calls running, by id: their thread (`None`: the agent's) and their tool.
    #[serde(skip)]
    open: HashMap<String, (Option<String>, KitKind, String)>,
}

/// The skill or plugin a call of `name` uses, if any.
pub fn kit_of(name: &str, input: &Value) -> Option<(KitKind, String)> {
    if name == "Skill" {
        let skill = input["skill"].as_str()?.trim();
        return (!skill.is_empty()).then(|| (KitKind::Skill, skill.to_string()));
    }
    // The tool's name is after the last `__`: a server's own name may hold some.
    let (server, _) = name.strip_prefix("mcp__")?.rsplit_once("__")?;
    (!server.is_empty()).then(|| (KitKind::Plugin, server.to_string()))
}

impl Canvas {
    /// The tools of a thread, `None` the agent's.
    fn thread(&mut self, call: Option<&str>) -> Option<&mut Vec<KitUse>> {
        match call {
            None => Some(&mut self.tools),
            Some(call) => self
                .subs
                .iter_mut()
                .find(|t| t.call == call)
                .map(|t| &mut t.tools),
        }
    }

    /// The thread of a subagent's call, made if new.
    fn make_thread(&mut self, call: &str) -> &mut ThreadKit {
        if let Some(i) = self.subs.iter().position(|t| t.call == call) {
            return &mut self.subs[i];
        }
        if self.subs.len() >= MAX_THREADS {
            // The first come with nothing running: a call that runs keeps its card and its flow.
            let idle = self.subs.iter().position(|t| {
                t.tools.iter().all(|k| k.running == 0)
                    && t.commands.iter().all(|c| c.ended_at.is_some())
            });
            self.subs.remove(idle.unwrap_or(0));
        }
        self.subs.push(ThreadKit {
            call: call.to_string(),
            ..ThreadKit::default()
        });
        self.subs.last_mut().expect("pushed")
    }

    /// A tool call, `parent` being the call of the subagent it comes from.
    pub fn on_tool_use(
        &mut self,
        parent: Option<&str>,
        id: &str,
        name: &str,
        input: &Value,
        now: i64,
    ) -> Change {
        if name == "Artifact" {
            // A file uploaded to a page's assets (`asset`, `file_paths`) is no version of it.
            let publish = input["action"].as_str().is_none_or(|a| a == "publish")
                && input["asset"].as_bool() != Some(true)
                && input.get("file_paths").is_none();
            if !publish || !self.seen.first_time(id) {
                return Change::None;
            }
            let title = input["title"].as_str().map(str::trim).unwrap_or("");
            let file = input["file_path"].as_str().unwrap_or("");
            let name = if title.is_empty() {
                file.rsplit(['/', '\\']).next().unwrap_or("")
            } else {
                title
            };
            self.publishing.push(Publishing {
                call: id.to_string(),
                name: name.to_string(),
                since: now,
            });
            return Change::View;
        }
        if name == "Bash" {
            if input["run_in_background"].as_bool() == Some(true) || !self.seen.first_time(id) {
                return Change::None;
            }
            // A call still running read again after a restart (`seen` is not saved) is the same run.
            let subs = self.subs.iter().flat_map(|t| t.commands.iter());
            if self.commands.iter().chain(subs).any(|c| c.call == id) {
                return Change::None;
            }
            let command = input["command"].as_str().unwrap_or("").trim();
            let commands = match parent {
                None => &mut self.commands,
                Some(call) => &mut self.make_thread(call).commands,
            };
            if commands.len() >= MAX_COMMANDS {
                commands.remove(0);
            }
            commands.push(CommandRun {
                call: id.to_string(),
                command: command.chars().take(MAX_COMMAND).collect(),
                started_at: now,
                ended_at: None,
                failed: false,
                stopped: false,
            });
            return Change::View;
        }
        let Some((kind, kit)) = kit_of(name, input) else {
            return Change::None;
        };
        if !self.seen.first_time(id) {
            return Change::None;
        }
        if let Some(call) = parent {
            self.make_thread(call);
        }
        self.open.insert(
            id.to_string(),
            (parent.map(str::to_string), kind, kit.clone()),
        );
        let tools = self.thread(parent).expect("made above");
        if let Some(t) = tools.iter_mut().find(|t| t.kind == kind && t.name == kit) {
            t.calls += 1;
            t.running += 1;
            t.last_at = now;
            return Change::View;
        }
        if tools.len() >= MAX_KIT {
            // The least recent of those with nothing running, else the least recent.
            let oldest = (0..tools.len()).min_by_key(|&i| (tools[i].running > 0, tools[i].last_at));
            tools.remove(oldest.expect("full"));
        }
        tools.push(KitUse {
            kind,
            name: kit,
            calls: 1,
            running: 1,
            first_at: now,
            last_at: now,
        });
        Change::Saved
    }

    /// A tool's result: `text` holds the address of a page published.
    pub fn on_tool_result(&mut self, id: &str, is_error: bool, text: &str, now: i64) -> Change {
        if let Some(i) = self.publishing.iter().position(|p| p.call == id) {
            let name = self.publishing.remove(i).name;
            let Some(url) = artifact_url(text).filter(|_| !is_error) else {
                return Change::View;
            };
            if let Some(a) = self.artifacts.iter_mut().find(|a| a.url == url) {
                a.versions += 1;
                a.title = name;
                a.at = now;
                return Change::Saved;
            }
            if self.artifacts.len() >= MAX_ARTIFACTS {
                let oldest = (0..self.artifacts.len()).min_by_key(|&i| self.artifacts[i].at);
                self.artifacts.remove(oldest.expect("full"));
            }
            self.artifacts.push(Artifact {
                url,
                title: name,
                versions: 1,
                at: now,
            });
            return Change::Saved;
        }
        let subs = self.subs.iter_mut().flat_map(|t| t.commands.iter_mut());
        if let Some(c) = self.commands.iter_mut().chain(subs).find(|c| c.call == id) {
            if c.ended_at.is_none() {
                c.ended_at = Some(now);
                c.failed = is_error;
                return Change::View;
            }
            return Change::None;
        }
        let Some((thread, kind, name)) = self.open.remove(id) else {
            return Change::None;
        };
        let Some(t) = self
            .thread(thread.as_deref())
            .and_then(|tools| tools.iter_mut().find(|t| t.kind == kind && t.name == name))
        else {
            return Change::None;
        };
        t.running = t.running.saturating_sub(1);
        t.last_at = now;
        Change::View
    }

    /// The process is gone: nothing it called runs any more.
    pub fn close_running(&mut self, now: i64) -> Change {
        let mut change = Change::None;
        if !self.publishing.is_empty() {
            self.publishing.clear();
            change = Change::View;
        }
        let commands = self.subs.iter_mut().flat_map(|t| t.commands.iter_mut());
        for c in self.commands.iter_mut().chain(commands) {
            if c.ended_at.is_none() {
                c.ended_at = Some(now);
                c.stopped = true;
                change = Change::View;
            }
        }
        let subs = self.subs.iter_mut().flat_map(|t| t.tools.iter_mut());
        for t in self.tools.iter_mut().chain(subs) {
            if t.running > 0 {
                t.running = 0;
                change = Change::View;
            }
        }
        self.open.clear();
        change
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
            && self.artifacts.is_empty()
            && self.publishing.is_empty()
            && self.commands.is_empty()
            && self
                .subs
                .iter()
                .all(|t| t.tools.is_empty() && t.commands.is_empty())
    }
}
