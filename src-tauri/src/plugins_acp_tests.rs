//! Tests of the generic ACP plugin (PLG-6, `dev.escouade.acp`, docs/PLUGINS-DESIGN.md §1.1): a harness only described by a
//! manifest (`contributes.harnesses[].acp`), served by `escouade-plugins acp` on the contract of the harnesses in
//! process (INT-9). The bench of the harnesses on the fake ACP agent of `tests/fixtures/fake-acp.mjs`, described by
//! `tests/fixtures/plugins/fake-acp/` without a line of Rust of its own; Cursor CLI's fake (`fake-cursor.mjs`) and its
//! recorded streams (`tests/fixtures/cursor/`) through the same client, its extension `cursor` on; the fence of a
//! private project on the files that go through the client, and said missing when they do not; Escouade's token never
//! on the command line; the harness put in the registry with its plugin and taken out with it (« harness absent »).

use crate::agent::{AgentRt, Effects};
use crate::board::TurnEnd;
use crate::engine::bench::{bench_dir, run, Scenario, Target};
use crate::engine::{Engine, Escouade, Process, Program, Spawn};
use crate::integrations::secrets::MemoryStore;
use crate::model::AgentMeta;
use crate::paths::DataDir;
use crate::plugins::builtin;
use crate::plugins::harness::{self, ProcessEngine};
use crate::plugins::runtime::{Limits, Runtime};
use crate::plugins::Plugins;
use escouade_plugin_protocol::engine::{self as proto, Item};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

const OWNER: &str = "dev.example.fake-acp";
const VERSION: &str = "2.2.0";

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tests")
        .join("fixtures")
}

/// The folder of the build, beside the tests' own (`deps/`): the workspace's modules are beside it (`plugins-wasm/`).
pub(crate) fn app_dir() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    exe.parent().unwrap().parent().unwrap().to_path_buf()
}

/// The manifest of the fake ACP agent's plugin, as its file says it.
fn fake_manifest() -> Value {
    let text = std::fs::read_to_string(
        fixtures()
            .join("plugins")
            .join("fake-acp")
            .join("escouade-plugin.json"),
    )
    .unwrap();
    serde_json::from_str(&text).unwrap()
}

/// The fake ACP agent's program: its `.cmd` under Windows, its shell script elsewhere.
fn fake_acp() -> Program {
    let name = if cfg!(windows) {
        "fake-acp.cmd"
    } else {
        "fake-acp"
    };
    Program::Native(fixtures().join("acp-bin").join(name))
}

/// The runtime of a data folder of its own: the workspace's plugins in it (the generic ACP plugin among them).
pub(crate) fn runtime(name: &str) -> (Arc<Runtime>, Arc<Plugins>, DataDir) {
    let data = DataDir::new(bench_dir(&format!("plg6-{name}")).join("data"));
    let plugins = Plugins::load_in(data.clone(), VERSION, app_dir());
    let rt = Runtime::new(
        plugins.clone(),
        Arc::new(MemoryStore::default()),
        &data,
        VERSION,
        Limits {
            initialize: Duration::from_secs(20),
            shutdown: Duration::from_secs(2),
            ..Limits::default()
        },
    );
    (rt, plugins, data)
}

/// The harness the plugin `owner` describes by `decl`, served by the generic ACP plugin. A third party's has its command
/// on the fake programs agreed to (SEC-01, ESC-589): its agreement is `third_party` tests'.
fn described(rt: &Arc<Runtime>, owner: &str, decl: Value) -> &'static ProcessEngine {
    let decl: proto::HarnessDecl = serde_json::from_value(decl).unwrap();
    let engine: &'static ProcessEngine =
        Box::leak(Box::new(ProcessEngine::described(rt.clone(), owner, decl)));
    for program in [fake_acp(), fake_unharnessed()] {
        if let Some(line) = engine.third_party_line(&program) {
            crate::plugins::consent::agree(&line);
        }
    }
    engine
}

/// The fake ACP agent's harness, from its manifest alone.
fn fake(name: &str) -> (Arc<Runtime>, &'static ProcessEngine) {
    let (rt, _, _) = runtime(name);
    let decl = fake_manifest()["contributes"]["harnesses"][0].clone();
    let engine = described(&rt, OWNER, decl);
    (rt, engine)
}

struct FakeAcp(&'static ProcessEngine);

impl Target for FakeAcp {
    fn engine(&self) -> &'static dyn Engine {
        self.0
    }
    fn program(&self) -> Program {
        fake_acp()
    }
    fn env(&self, dir: &Path) -> Vec<(String, String)> {
        vec![
            (
                "FAKE_ACP_HOME".into(),
                dir.join("home").to_string_lossy().into(),
            ),
            (
                "FAKE_ACP_LOG".into(),
                dir.join("fake-acp.jsonl").to_string_lossy().into(),
            ),
        ]
    }
    fn prompt(&self, scenario: Scenario) -> Option<String> {
        Some(
            match scenario {
                Scenario::Text | Scenario::Resume => "bonjour",
                Scenario::Thinking => "raisonnement",
                Scenario::Approval => "commande",
                Scenario::Edit => "fichier",
                Scenario::Question => "question",
                Scenario::Interrupt => "lent",
                Scenario::Subagents => "sous-agent",
                // No mode Plan, no copy, no quota: none asked of it.
                Scenario::PlanMode | Scenario::Limit | Scenario::Fork | Scenario::Quota => {
                    return None
                }
            }
            .to_string(),
        )
    }
    fn model(&self) -> Option<String> {
        Some("rapide".into())
    }
    /// Its writes in the folder go through in « Accepter les modifications » (they ask in « Demander »).
    fn mode(&self, scenario: Scenario) -> Option<&'static str> {
        (scenario == Scenario::Edit).then_some("acceptEdits")
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_fake_acp_agent_described_by_a_manifest_passes_the_bench() {
    let (rt, engine) = fake("bench");
    assert_eq!(engine.id().as_str(), "plugin:dev.example.fake-acp:fake-acp");
    assert_eq!(engine.plugin(), builtin::ACP);
    let played = run(&FakeAcp(engine), &bench_dir("plg6-bench-run")).await;
    assert_eq!(
        played,
        [
            Scenario::Text,
            Scenario::Thinking,
            Scenario::Approval,
            Scenario::Edit,
            Scenario::Question,
            Scenario::Interrupt,
            Scenario::Subagents,
            Scenario::Resume,
        ]
    );
    assert_eq!(rt.status(builtin::ACP).restarts, 0);
    rt.shutdown();
}

/// What the manifest says of it reaches the registry's declaration: its modes and their meaning, its models, its
/// installers, its folder variable, read-only held.
#[test]
fn what_the_manifest_declares_is_what_the_harness_is() {
    let (_rt, engine) = fake("declared");
    assert_eq!(engine.name(), "Faux ACP");
    let caps = engine.caps();
    let modes: Vec<&str> = caps.modes.iter().map(|m| m.id).collect();
    assert_eq!(modes, ["default", "acceptEdits", "bypassPermissions"]);
    assert!(caps.modes[0].meaning.is_some());
    assert!(caps.read_only);
    let models: Vec<&str> = engine.models().iter().map(|m| m.id).collect();
    assert_eq!(models, ["rapide", "fin"]);
    assert_eq!(engine.install().unix, "npm install -g fake-acp");
    assert_eq!(engine.tested().oldest, "1.0.0");
    assert!(matches!(
        engine.accounts(),
        crate::engine::registry::AccountStore::ConfigDir {
            var: "FAKE_ACP_HOME"
        }
    ));
    // A declarative plugin names no path to run: its program is found by its names.
    let mut decl = fake_manifest()["contributes"]["harnesses"][0].clone();
    decl["locate"]["program"] = json!(if cfg!(windows) {
        "C:\\Windows\\System32\\cmd.exe"
    } else {
        "/bin/sh"
    });
    let (rt, _, _) = runtime("declared-path");
    let e = described(&rt, OWNER, decl);
    let found = e.locate("", "").map(|p| p.describe());
    assert!(
        found
            .as_deref()
            .is_none_or(|p| !p.contains("cmd.exe") && !p.ends_with("/bin/sh")),
        "{found:?}"
    );
}

/// Cursor CLI as the manifest of its plugin declares it (INT-10.1): an ACP harness with its extension `cursor`.
fn cursor_decl() -> Value {
    let v: Value =
        serde_json::from_str(include_str!("../../plugins/cursor/escouade-plugin.json")).unwrap();
    v["contributes"]["harnesses"][0].clone()
}

struct FakeCursor(&'static ProcessEngine);

impl Target for FakeCursor {
    fn engine(&self) -> &'static dyn Engine {
        self.0
    }
    fn program(&self) -> Program {
        crate::engine::CURSOR
            .locate(&crate::engine::cursor::fake_cursor(), "")
            .expect("the fake agent")
    }
    fn env(&self, dir: &Path) -> Vec<(String, String)> {
        vec![
            (
                "CURSOR_CONFIG_DIR".into(),
                dir.join("cursor").to_string_lossy().into(),
            ),
            (
                "FAKE_CURSOR_LOG".into(),
                dir.join("fake-cursor.jsonl").to_string_lossy().into(),
            ),
        ]
    }
    fn prompt(&self, scenario: Scenario) -> Option<String> {
        Some(
            match scenario {
                Scenario::Text | Scenario::Resume => "bonjour",
                Scenario::Thinking => "raisonnement",
                Scenario::Approval => "commande",
                Scenario::Edit => "fichier",
                Scenario::Question => "question",
                Scenario::Interrupt => "lent",
                Scenario::PlanMode => "propose un plan",
                Scenario::Subagents => "sous-agent",
                Scenario::Limit | Scenario::Fork | Scenario::Quota => return None,
            }
            .to_string(),
        )
    }
    fn model(&self) -> Option<String> {
        Some("default".into())
    }
    fn mode(&self, scenario: Scenario) -> Option<&'static str> {
        (scenario == Scenario::Edit).then_some("acceptEdits")
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_fake_cursor_replayed_by_the_generic_client_passes_the_bench() {
    let (rt, _, _) = runtime("cursor-bench");
    let engine = described(&rt, "dev.example.cursor-acp", cursor_decl());
    // A third party's description: its command on the fake Cursor agreed to (SEC-01, ESC-589).
    crate::plugins::consent::agree(
        &engine
            .third_party_line(&FakeCursor(engine).program())
            .unwrap(),
    );
    let played = run(&FakeCursor(engine), &bench_dir("plg6-cursor-bench-run")).await;
    assert_eq!(
        played,
        [
            Scenario::Text,
            Scenario::Thinking,
            Scenario::Approval,
            Scenario::Edit,
            Scenario::Question,
            Scenario::Interrupt,
            Scenario::PlanMode,
            Scenario::Subagents,
            Scenario::Resume,
        ]
    );
    rt.shutdown();
}

/// The recorded stream `lines` (`{"dir", "msg"}` a line: `s2c` or `in` what the harness said, `c2s` or `out` what its
/// client said) through the client of `escouade_acp` for the harness `decl`, its items applied to an agent of `meta` in
/// `conv`; what the app said (a message, an answer) done as the core does it.
pub(crate) fn replayed_through_the_client(
    decl: &Value,
    meta: AgentMeta,
    lines: &[Value],
    conv: &Path,
) -> AgentRt {
    let decl: proto::HarnessDecl = serde_json::from_value(decl.clone()).unwrap();
    let attach = proto::Attach {
        agent: "a".into(),
        cwd: meta.cwd.clone(),
        session: meta.session_id.clone(),
        model: meta.model.clone(),
        mode: meta.mode.clone(),
        lang: "fr".into(),
        ..proto::Attach::default()
    };
    let mut client = escouade_acp::Agent::new(&attach, &decl, VERSION);
    let mut rt = AgentRt::new(meta, conv);
    // The ids of the recording's requests, as the client named them; the forms of Vibe's questions, by them.
    let mut asked: std::collections::HashMap<String, String> = Default::default();
    let mut forms: std::collections::HashMap<String, Value> = Default::default();
    for f in lines {
        // Gemini's are the message itself, with its `dir`.
        let msg = if f["msg"].is_null() { f } else { &f["msg"] };
        let mut fx = Effects::default();
        let harness = matches!(f["dir"].as_str(), Some("s2c" | "in"));
        let client_said = matches!(f["dir"].as_str(), Some("c2s" | "out"));
        if harness {
            let items = client.frames(&[msg.to_string()]);
            for i in &items {
                if let Item::Permission { request, .. } | Item::Question { request, .. } = i {
                    asked.insert(msg["id"].to_string(), request.clone());
                    if msg["method"] == "elicitation/create" {
                        forms.insert(request.clone(), msg["params"].clone());
                    }
                }
            }
            let items: Vec<Value> = items
                .into_iter()
                .filter(|i| !matches!(i, Item::Write { .. }))
                .map(|i| serde_json::to_value(i).unwrap())
                .collect();
            rt.apply_items(&items, &mut fx);
        } else if client_said && msg["method"] == "session/prompt" {
            let text = msg["params"]["prompt"][0]["text"]
                .as_str()
                .unwrap_or_default();
            let id = crate::model::new_id();
            rt.push_user(&id, text, 0, &[], &mut fx);
            client.recorded(msg);
        } else if client_said && msg["method"].is_string() {
            client.recorded(msg);
        } else if client_said && msg["method"].is_null() && msg["result"]["outcome"].is_object() {
            if let Some(rid) = asked.get(&msg["id"].to_string()) {
                let decision = match msg["result"]["outcome"]["optionId"].as_str() {
                    Some(o)
                        if o.starts_with("allow-always")
                            || o.starts_with("allow_always")
                            || o.starts_with("proceed_always") =>
                    {
                        "always"
                    }
                    Some(o) if o.starts_with("allow") || o.starts_with("proceed") => "allow",
                    _ => "deny",
                };
                rt.recorded_answer(rid, decision, &mut fx);
            }
        } else if client_said && msg["method"].is_null() && msg["result"]["action"].is_string() {
            // The trial's answer to Vibe's form: its card says it, by question.
            if let Some(rid) = asked.get(&msg["id"].to_string()) {
                let form = forms.get(rid).cloned().unwrap_or_default();
                let mut answers = serde_json::Map::new();
                for (key, q) in escouade_acp::vibe::elicitation_questions(&form) {
                    let given = &msg["result"]["content"][&key];
                    let text = match given.as_array() {
                        Some(list) => list
                            .iter()
                            .filter_map(Value::as_str)
                            .collect::<Vec<_>>()
                            .join(", "),
                        None => given.as_str().unwrap_or_default().to_string(),
                    };
                    let question = q["question"].as_str().unwrap_or_default().to_string();
                    answers.insert(question, json!(text));
                }
                rt.recorded_answers(rid, Value::Object(answers), &mut fx);
            }
        }
    }
    rt
}

/// The recorded streams of `harness` (its cases of tests/fixtures/replays/), through the client of the harness `decl`:
/// the conversation is the one the core's own translation made, compared by `replay::within`, and the agent after it.
fn check_replays_through_the_client(harness: &str, decl: &Value, cases: &[&str]) {
    for case in cases {
        let path = crate::engine::bench::replay::cases_of(harness).join(format!("{case}.json"));
        let c: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let flux =
            std::fs::read_to_string(path.parent().unwrap().join(c["flux"].as_str().unwrap()))
                .unwrap();
        let mut meta = serde_json::to_value(AgentMeta {
            id: crate::model::new_id(),
            ..Default::default()
        })
        .unwrap();
        for (k, v) in c["agent"].as_object().unwrap() {
            meta[k] = v.clone();
        }
        let meta: AgentMeta = serde_json::from_value(meta).unwrap();
        let lines: Vec<Value> = flux
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        let conv = crate::paths::test_dir(&format!("int10-replay-{harness}-{case}"));
        let mut rt = replayed_through_the_client(decl, meta, &lines, &conv);
        let made: Vec<Value> = rt
            .conv
            .items()
            .into_iter()
            .map(|mut i| {
                if let Some(o) = i.as_object_mut() {
                    o.remove("id");
                    if o.get("kind").and_then(Value::as_str) == Some("turn") {
                        o.insert("durationMs".into(), Value::Null);
                    }
                }
                crate::engine::bench::replay::strip_times(&mut i);
                i
            })
            .collect();
        let expected = c["items"].as_array().unwrap();
        let kinds = |items: &[Value]| -> Vec<String> {
            items
                .iter()
                .map(|i| i["kind"].as_str().unwrap_or("?").to_string())
                .collect()
        };
        assert_eq!(
            kinds(expected),
            kinds(&made),
            "{case}: {:#}",
            Value::Array(made.clone())
        );
        for (n, (want, got)) in expected.iter().zip(&made).enumerate() {
            // The id of a request: its client's own name for it, not the conversation's (`recorded-<id>` in the core).
            let mut want = want.clone();
            if want["toolUseId"]
                .as_str()
                .is_some_and(|i| i.starts_with("recorded-"))
            {
                want.as_object_mut().unwrap().remove("toolUseId");
            }
            let want = &want;
            if let Err(at) = crate::engine::bench::replay::within(want, got, "") {
                panic!("{case}: item {n}, at {at}\nexpected: {want:#}\nmade: {got:#}");
            }
        }
        let after = serde_json::to_value(&rt.meta).unwrap();
        for (k, v) in c["agentAfter"].as_object().unwrap() {
            if k == "lastEntry" {
                continue;
            }
            assert_eq!(&after[k], v, "{case}: the agent's {k}");
        }
    }
}

/// The recorded streams of Cursor CLI 2026.10.22 (tests/fixtures/cursor/), what Cursor said read by the generic
/// client with its extension `cursor` and its items applied to the agent; what the app said (a message, an answer)
/// done as the core does it. The conversation is the one the core's own translation makes
/// (tests/fixtures/replays/cursor/), compared by `replay::within`.
#[test]
fn the_recorded_streams_of_cursor_make_through_the_generic_client_the_items_of_the_core() {
    check_replays_through_the_client("cursor", &cursor_decl(), &["full-turn", "load-modes-model"]);
}

/// Mistral Vibe as the manifest of its plugin declares it (INT-10.6): an ACP harness with its extension `vibe`.
pub(crate) fn vibe_decl() -> Value {
    let v: Value =
        serde_json::from_str(include_str!("../../plugins/mistral/escouade-plugin.json")).unwrap();
    v["contributes"]["harnesses"][0].clone()
}

/// The recorded streams of Mistral Vibe 3.0.3 (tests/fixtures/vibe/), what Vibe said read by the client of its plugin
/// with its extension `vibe` (INT-10.6): the conversation is the one the core's own translation made
/// (tests/fixtures/replays/vibe/), its cost and tokens included, and the agent after it.
#[test]
fn the_recorded_streams_of_vibe_make_through_its_plugin_s_client_the_items_of_the_core() {
    check_replays_through_the_client(
        "vibe",
        &vibe_decl(),
        &[
            "full-turn",
            "question-elicitation",
            "todo-subagent",
            "load-fork-reject",
            "cancel-config-options",
            "error-invalid-key",
        ],
    );
}

/// Gemini CLI as the manifest of its plugin declares it (INT-10.7): an ACP harness with its extension `gemini`.
pub(crate) fn gemini_decl() -> Value {
    let v: Value =
        serde_json::from_str(include_str!("../../plugins/gemini/escouade-plugin.json")).unwrap();
    v["contributes"]["harnesses"][0].clone()
}

/// The streams of the study of Gemini CLI 0.66.0 (tests/fixtures/gemini/), what Gemini said read by the client of its
/// plugin with its extension `gemini` (INT-10.7): the conversation is the one the core's own translation made
/// (tests/fixtures/replays/gemini/), its tokens included, and the agent after it.
#[test]
fn the_recorded_streams_of_gemini_make_through_its_plugin_s_client_the_items_of_the_core() {
    check_replays_through_the_client(
        "gemini",
        &gemini_decl(),
        &[
            "load-replay",
            "mcp-tool",
            "permission-shell",
            "permission-write",
            "plan-exit-approval",
            "policy-fence-deny",
            "rate-limit",
            "simple-turn",
            "subagent-yolo",
            "thoughts",
        ],
    );
}

/// What a process says, as the engine's callbacks pass it on.
enum Heard {
    Frame(Value),
    Exit(String),
}

/// An agent of the fake ACP agent, driven as the core drives it.
struct Driven {
    engine: &'static ProcessEngine,
    rt: AgentRt,
    heard: mpsc::UnboundedReceiver<Heard>,
    ends: Vec<TurnEnd>,
    exit: Option<String>,
    log: PathBuf,
}

impl Driven {
    fn spawn_with(
        engine: &'static ProcessEngine,
        dir: &Path,
        f: impl FnOnce(&mut Spawn),
    ) -> Driven {
        let meta = AgentMeta {
            id: crate::model::new_id(),
            cwd: dir.to_string_lossy().to_string(),
            model: "rapide".into(),
            mode: "default".into(),
            ..Default::default()
        };
        let log = dir.join("fake-acp.jsonl");
        let env = vec![
            (
                "FAKE_ACP_HOME".into(),
                dir.join("home").to_string_lossy().into_owned(),
            ),
            ("FAKE_ACP_LOG".into(), log.to_string_lossy().into_owned()),
        ];
        Driven::spawn_on(engine, dir, meta, fake_acp(), env, log, f)
    }

    /// An agent of `meta` on `program`, its log `log`: another fake than Faux ACP's (Unharnessed's, ESC-484).
    fn spawn_on(
        engine: &'static ProcessEngine,
        dir: &Path,
        meta: AgentMeta,
        program: Program,
        env: Vec<(String, String)>,
        log: PathBuf,
        f: impl FnOnce(&mut Spawn),
    ) -> Driven {
        let mut spawn = Spawn::of(&meta, program, env);
        f(&mut spawn);
        let (tx, heard) = mpsc::unbounded_channel();
        let exit = tx.clone();
        let proc: Process = engine
            .spawn(
                spawn,
                Box::new(move |frame| {
                    let _ = tx.send(Heard::Frame(frame));
                }),
                Box::new(move |_, said| {
                    let _ = exit.send(Heard::Exit(said));
                }),
            )
            .unwrap();
        let mut rt = AgentRt::new(meta, &dir.join("conv"));
        rt.attach(proc);
        Driven {
            engine,
            rt,
            heard,
            ends: Vec::new(),
            exit: None,
            log,
        }
    }

    fn proc(&self) -> Process {
        self.rt.proc.clone().unwrap()
    }

    fn send(&mut self, text: &str) {
        let message = self.engine.message(text, &[]).unwrap();
        let uid = uuid::Uuid::new_v4().to_string();
        self.proc().send_message(&uid, &message).unwrap();
        let mut fx = Effects::default();
        self.rt.push_user(&uid, text, 0, &[], &mut fx);
    }

    async fn until(&mut self, what: &str, pred: impl Fn(&mut Driven) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(60);
        while !pred(self) {
            let left = deadline.saturating_duration_since(Instant::now());
            match tokio::time::timeout(left, self.heard.recv()).await {
                Ok(Some(Heard::Frame(frame))) => {
                    let mut fx = Effects::default();
                    self.engine.translate(&mut self.rt, &frame, &mut fx);
                    if let Some(end) = fx.turn_end {
                        self.ends.push(end);
                    }
                }
                Ok(Some(Heard::Exit(said))) => {
                    self.exit = Some(said);
                    if !pred(self) {
                        panic!("the process ended before {what}: {:?}", self.exit);
                    }
                }
                _ => panic!(
                    "no {what}; the conversation: {:#}",
                    Value::Array(self.rt.conv.items())
                ),
            }
        }
    }

    async fn turn(&mut self, text: &str) -> TurnEnd {
        let n = self.ends.len();
        self.send(text);
        self.until("the end of the turn", |a| a.ends.len() > n)
            .await;
        self.ends[n].clone()
    }

    /// What the fake was told and said.
    fn logged(&self) -> Vec<Value> {
        std::fs::read_to_string(&self.log)
            .unwrap_or_default()
            .lines()
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect()
    }

    async fn stop(&mut self) {
        if let Some(p) = self.rt.detach() {
            p.close_input();
            if !p.wait_exit(Duration::from_secs(10)).await {
                p.kill();
            }
        }
    }
}

/// « Projet privé »: a file of another project read through the client is refused, the agent told why; one the agent
/// reads by itself is said in the conversation: the fence does not hold for it.
#[tokio::test(flavor = "multi_thread")]
async fn a_private_project_is_fenced_on_the_files_that_go_through_the_client_and_said_missing_otherwise(
) {
    let (rt, engine) = fake("fence");
    let dir = bench_dir("plg6-fence-run");
    let other = bench_dir("plg6-fence-other");
    let secret = other.join("secret.txt");
    std::fs::write(&secret, "le secret").unwrap();
    let fence = crate::fence::Fence::new(vec![other.clone()]);
    let mut a = Driven::spawn_with(engine, &dir, |s| s.fence = fence);
    let secret = secret.to_string_lossy().into_owned();
    let end = a.turn(&format!("lis {secret}")).await;
    assert!(
        matches!(&end, TurnEnd::Finished(t) if t.contains("refusé") && !t.contains("le secret")),
        "{end:?}"
    );
    let items = a.rt.conv.items();
    assert!(
        !items
            .iter()
            .any(|i| i["kind"] == "notice" && i["level"] == "warning"),
        "a read through the client is fenced, nothing to say: {items:#?}"
    );
    // Read by itself: what the client cannot fence, said once.
    let alone = other.join("autre.txt");
    std::fs::write(&alone, "un autre").unwrap();
    a.turn(&format!("lis seul {}", alone.display())).await;
    a.turn(&format!("lis seul {}", alone.display())).await;
    let items = a.rt.conv.items();
    let said: Vec<&Value> = items
        .iter()
        .filter(|i| i["kind"] == "notice" && i["level"] == "warning")
        .collect();
    assert_eq!(said.len(), 1, "{items:#?}");
    assert!(
        said[0]["text"]
            .as_str()
            .unwrap()
            .contains("sans le faire passer par Escouade"),
        "{said:?}"
    );
    // The fence was told to the client, never to the agent's command line.
    let launch = a
        .logged()
        .into_iter()
        .find_map(|e| e.get("launch").cloned())
        .unwrap();
    assert_eq!(launch["argv"], json!(["acp"]));
    a.stop().await;
    rt.shutdown();
}

/// An agent that reads and writes files by itself (`acp.fs` false) does not start in a private project: the fence
/// would not hold, and its start says why.
#[tokio::test(flavor = "multi_thread")]
async fn an_agent_that_passes_no_file_through_the_client_does_not_start_in_a_private_project() {
    let (rt, _, _) = runtime("nofs");
    let mut decl = fake_manifest()["contributes"]["harnesses"][0].clone();
    decl["acp"]["fs"] = json!(false);
    let engine = described(&rt, OWNER, decl);
    let dir = bench_dir("plg6-nofs-run");
    let fence = crate::fence::Fence::new(vec![bench_dir("plg6-nofs-other")]);
    let a = Driven::spawn_with(engine, &dir, |s| s.fence = fence);
    let err = a.proc().initialize().await.unwrap_err().to_string();
    assert!(
        err.contains("la clôture des projets privés ne tiendrait pas"),
        "{err}"
    );
    rt.shutdown();
}

/// The word `fenceOnPermissions` of a manifest of anyone's is not taken: nothing passes through the client, so no private
/// project is open to it, as for any `"fs": false`. The client's fence on `fs/*` is its own, and stays for a harness of
/// anyone's that goes through it.
#[tokio::test(flavor = "multi_thread")]
async fn a_third_party_manifest_does_not_hold_the_fence_by_saying_so() {
    let (rt, _, _) = runtime("tiers-cloture");
    let mut decl = fake_manifest()["contributes"]["harnesses"][0].clone();
    decl["acp"]["fs"] = json!(false);
    decl["acp"]["fenceOnPermissions"] = json!(true);
    let engine = described(&rt, OWNER, decl);
    let dir = bench_dir("plg6-tiers-cloture-run");
    let fence = crate::fence::Fence::new(vec![bench_dir("plg6-tiers-cloture-other")]);
    let a = Driven::spawn_with(engine, &dir, |s| s.fence = fence);
    let err = a.proc().initialize().await.unwrap_err().to_string();
    assert!(
        err.contains("la clôture des projets privés ne tiendrait pas"),
        "{err}"
    );
    assert!(
        !dir.join("fake-acp.jsonl").exists(),
        "its program never ran"
    );
    rt.shutdown();
}

/// A plugin of Escouade's own keeps its word on a harness it describes: `dev.escouade.goose` is in no registry of the app,
/// so the rule cannot be `is_builtin` of its manifest's plugin.
#[tokio::test(flavor = "multi_thread")]
async fn a_first_hand_manifest_keeps_its_word_on_the_fence() {
    let (rt, _, _) = runtime("goose-cloture");
    let mut decl = fake_manifest()["contributes"]["harnesses"][0].clone();
    decl["acp"]["fs"] = json!(false);
    decl["acp"]["fenceOnPermissions"] = json!(true);
    let engine = described(&rt, "dev.escouade.goose", decl);
    let dir = bench_dir("plg6-goose-cloture-run");
    let fence = crate::fence::Fence::new(vec![bench_dir("plg6-goose-cloture-other")]);
    let mut a = Driven::spawn_with(engine, &dir, |s| s.fence = fence);
    a.proc().initialize().await.unwrap();
    a.stop().await;
    rt.shutdown();
}

/// Goose as its manifest describes it starts, and answers.
#[tokio::test(flavor = "multi_thread")]
async fn goose_as_its_manifest_describes_it_starts() {
    let (rt, _, _) = runtime("goose");
    let text = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../plugins/goose/escouade-plugin.json"),
    )
    .unwrap();
    let manifest: Value = serde_json::from_str(&text).unwrap();
    let engine = described(
        &rt,
        "dev.escouade.goose",
        manifest["contributes"]["harnesses"][0].clone(),
    );
    assert_eq!(engine.id().as_str(), "goose");
    let dir = bench_dir("plg6-goose-run");
    let mut a = Driven::spawn_with(engine, &dir, |_| {});
    a.proc().initialize().await.unwrap();
    let end = a.turn("bonjour").await;
    assert!(matches!(&end, TurnEnd::Finished(_)), "{end:?}");
    a.stop().await;
    rt.shutdown();
}

/// Unharnessed as the manifest of its plugin declares it (ESC-484): a harness described over ACP whose one mode is Bypass.
fn unharnessed_decl() -> Value {
    let v: Value = serde_json::from_str(include_str!(
        "../../plugins/unharnessed/escouade-plugin.json"
    ))
    .unwrap();
    v["contributes"]["harnesses"][0].clone()
}

/// The fake Unharnessed (tests/fixtures/fake-unharnessed.mjs), by the name its manifest finds: its `.cmd` under Windows.
fn fake_unharnessed() -> Program {
    let name = if cfg!(windows) {
        "unharnessed.cmd"
    } else {
        "unharnessed"
    };
    Program::Native(fixtures().join("unharnessed-bin").join(name))
}

/// An agent of Unharnessed in `dir` (its account's folder `dir/home`), in the mode it has, on the session `session`.
fn unharnessed_agent(
    engine: &'static ProcessEngine,
    dir: &Path,
    session: Option<String>,
    f: impl FnOnce(&mut Spawn),
) -> Driven {
    let meta = AgentMeta {
        id: crate::model::new_id(),
        cwd: dir.to_string_lossy().to_string(),
        mode: crate::bypass::BYPASS.into(),
        session_id: session,
        ..Default::default()
    };
    let log = dir.join("fake-unharnessed.jsonl");
    let env = vec![
        (
            "UNHARNESSED_CODING_AGENT_DIR".into(),
            dir.join("home").to_string_lossy().into_owned(),
        ),
        (
            "FAKE_UNHARNESSED_LOG".into(),
            log.to_string_lossy().into_owned(),
        ),
    ];
    Driven::spawn_on(engine, dir, meta, fake_unharnessed(), env, log, f)
}

/// The harness of `dev.escouade.unharnessed`, from its manifest alone.
fn unharnessed(name: &str) -> (Arc<Runtime>, &'static ProcessEngine) {
    let (rt, _, _) = runtime(name);
    let engine = described(&rt, "dev.escouade.unharnessed", unharnessed_decl());
    (rt, engine)
}

/// Unharnessed as its manifest describes it, on a fake that speaks as Unharnessed 0.9.8 did in the study: it answers,
/// its one tool shows as Python with its code and its output, a turn under way stops, no mode and no model are set
/// (it has none of the first, the second is each user's own), and a new process takes its session back by `--resume`
/// (ESC-491); its session gone, the agent starts again on a new one, said, given its instructions again.
#[tokio::test(flavor = "multi_thread")]
async fn unharnessed_answers_runs_its_cell_stops_and_takes_its_session_back() {
    let (rt, engine) = unharnessed("unharnessed");
    assert_eq!(engine.id().as_str(), "unharnessed");
    let dir = bench_dir("esc484-unharnessed-run");
    let mut a = unharnessed_agent(engine, &dir, None, |_| {});
    a.proc().initialize().await.unwrap();
    let end = a.turn("bonjour").await;
    assert!(
        matches!(&end, TurnEnd::Finished(t) if t.contains("Unharnessed a reçu : bonjour")),
        "{end:?}"
    );
    // Its cell: a tool « Python », its code as its input, its output as its result.
    let end = a.turn("python").await;
    assert!(
        matches!(&end, TurnEnd::Finished(t) if t.contains("1267650600228229401496703205376")),
        "{end:?}"
    );
    let items = a.rt.conv.items();
    let cell = items
        .iter()
        .find(|i| i["kind"] == "tool")
        .unwrap_or_else(|| panic!("no tool: {items:#?}"));
    assert_eq!(cell["name"], "Python", "{cell:#}");
    assert_eq!(cell["input"]["code"], "print(2 ** 100)", "{cell:#}");
    assert!(
        cell["result"]["text"]
            .as_str()
            .is_some_and(|t| t.contains("1267650600228229401496703205376")),
        "{cell:#}"
    );
    // A turn under way stops: its cell is aborted, the turn interrupted.
    let n = a.ends.len();
    a.send("long");
    a.until("the cell under way", |d| {
        d.rt.conv
            .items()
            .iter()
            .any(|i| i["kind"] == "tool" && i["input"]["code"] == "r = await bash('sleep 90')")
    })
    .await;
    a.rt.interrupted = true;
    a.proc()
        .interrupt(false, Duration::from_secs(15))
        .await
        .unwrap();
    a.until("the end of the stopped turn", |d| d.ends.len() > n)
        .await;
    assert_eq!(a.ends[n], TurnEnd::Interrupted);
    // Asked nothing, set nothing: no permission, no mode, no model, no effort.
    let said: Vec<String> = a
        .logged()
        .into_iter()
        .filter(|e| e["dir"] == "c2s")
        .filter_map(|e| e["msg"]["method"].as_str().map(str::to_string))
        .collect();
    for never in [
        "session/set_mode",
        "session/set_model",
        "session/set_config_option",
    ] {
        assert!(!said.iter().any(|m| m == never), "{never}: {said:?}");
    }
    let launch = a
        .logged()
        .into_iter()
        .find_map(|e| e.get("launch").cloned())
        .unwrap();
    assert_eq!(launch["argv"], json!(["--mode", "acp"]));
    assert_eq!(
        launch["home"],
        json!(dir.join("home").to_string_lossy()),
        "its account's folder"
    );
    let session = a.rt.meta.session_id.clone().expect("its session");
    a.stop().await;

    // A new process takes its session back by `--resume <id>` (`acp.resume`, ESC-491): the id it saves it under, then
    // `session/load` of it, nothing said, nothing given again.
    let _ = std::fs::remove_file(dir.join("fake-unharnessed.jsonl"));
    let mut b = unharnessed_agent(engine, &dir, Some(session.clone()), |s| {
        s.append_prompt = Some("Le protocole du ticket ESC-491.".into())
    });
    b.proc().initialize().await.unwrap();
    let end = b.turn("encore là ?").await;
    assert!(
        matches!(&end, TurnEnd::Finished(t) if t.contains("Unharnessed a reçu : encore là ?")),
        "{end:?}"
    );
    let launch = b
        .logged()
        .into_iter()
        .find_map(|e| e.get("launch").cloned())
        .unwrap();
    assert_eq!(
        launch["argv"],
        json!(["--mode", "acp", "--resume", session.clone()])
    );
    let items = b.rt.conv.items();
    assert!(!items.iter().any(|i| i["kind"] == "notice"), "{items:#?}");
    assert_eq!(b.rt.meta.session_id.as_deref(), Some(session.as_str()));
    let prompts: Vec<String> = b
        .logged()
        .into_iter()
        .filter(|e| e["msg"]["method"] == "session/prompt")
        .map(|e| e["msg"]["params"]["prompt"].to_string())
        .collect();
    assert!(
        !prompts.iter().any(|p| p.contains("ESC-491")),
        "its session has them: {prompts:?}"
    );
    b.stop().await;

    // Its session gone (its file removed): a new one, said, given its instructions again.
    for f in std::fs::read_dir(dir.join("home").join("sessions")).unwrap() {
        std::fs::remove_file(f.unwrap().path()).unwrap();
    }
    let _ = std::fs::remove_file(dir.join("fake-unharnessed.jsonl"));
    let mut c = unharnessed_agent(engine, &dir, Some(session.clone()), |s| {
        s.append_prompt = Some("Le protocole du ticket ESC-491.".into())
    });
    c.proc().initialize().await.unwrap();
    let end = c.turn("toujours là ?").await;
    assert!(matches!(&end, TurnEnd::Finished(_)), "{end:?}");
    let items = c.rt.conv.items();
    assert!(
        items.iter().any(|i| i["kind"] == "notice"
            && i["text"]
                .as_str()
                .is_some_and(|t| t.contains("n’a pas pu être reprise")
                    && t.contains("Ses consignes lui sont redonnées"))),
        "{items:#?}"
    );
    // The fake saves the new one under the id `--resume` named: what the real one does then was not tried.
    let prompt = c
        .logged()
        .into_iter()
        .find(|e| e["msg"]["method"] == "session/prompt")
        .map(|e| e["msg"]["params"]["prompt"].to_string())
        .unwrap();
    assert!(prompt.contains("ESC-491"), "{prompt}");
    c.stop().await;
    rt.shutdown();
}

/// No discussion of the Docs tab on Unharnessed: it has no mode that changes nothing, and it does not start.
#[tokio::test(flavor = "multi_thread")]
async fn a_discussion_of_unharnessed_does_not_start() {
    let (rt, engine) = unharnessed("unharnessed-readonly");
    let dir = bench_dir("esc484-unharnessed-readonly");
    let a = unharnessed_agent(engine, &dir, None, |s| s.read_only = true);
    let err = a.proc().initialize().await.unwrap_err().to_string();
    assert!(
        err.contains("ne tient pas une discussion en lecture seule"),
        "{err}"
    );
    assert!(
        !dir.join("fake-unharnessed.jsonl").exists(),
        "its program never ran"
    );
    rt.shutdown();
}

/// Nor beside a private project: it reads and writes by itself, the fence would not hold.
#[tokio::test(flavor = "multi_thread")]
async fn an_agent_of_unharnessed_does_not_start_beside_a_private_project() {
    let (rt, engine) = unharnessed("unharnessed-fence");
    let dir = bench_dir("esc484-unharnessed-fence");
    let fence = crate::fence::Fence::new(vec![bench_dir("esc484-unharnessed-fence-other")]);
    let a = unharnessed_agent(engine, &dir, None, |s| s.fence = fence);
    let err = a.proc().initialize().await.unwrap_err().to_string();
    assert!(
        err.contains("la clôture des projets privés ne tiendrait pas"),
        "{err}"
    );
    assert!(!dir.join("fake-unharnessed.jsonl").exists());
    rt.shutdown();
}

/// Its turns of its own (a « wake », once a command of its last cell ended) come with no prompt under way (ESC-484):
/// a turn of the conversation of its own (ESC-491), the agent at work while it runs, ended by the marks of its stream
/// (`acp.turnEnd`), its words its own; the next turn's answer does not begin with them.
#[tokio::test(flavor = "multi_thread")]
async fn a_turn_unharnessed_starts_by_itself_is_a_turn_of_its_own() {
    let (rt, engine) = unharnessed("unharnessed-wake");
    let dir = bench_dir("esc484-unharnessed-wake");
    let mut a = unharnessed_agent(engine, &dir, None, |_| {});
    a.proc().initialize().await.unwrap();
    let end = a.turn("réveil").await;
    assert!(matches!(&end, TurnEnd::Finished(_)), "{end:?}");
    // The fake's wake, 300 ms after the end of the turn: a turn that begins and ends by itself.
    let n = a.ends.len();
    a.until("the end of the wake", |d| d.ends.len() > n).await;
    assert!(
        matches!(&a.ends[n], TurnEnd::Finished(t) if t == "Réveillé par la fin de sa commande."),
        "{:?}",
        a.ends
    );
    assert_eq!(a.rt.meta.status, crate::model::AgentStatus::Done);
    let items = a.rt.conv.items();
    let turns = items.iter().filter(|i| i["kind"] == "turn").count();
    assert_eq!(turns, 2, "the wake's turn closed: {items:#?}");
    assert!(
        !items.iter().any(|i| i["streaming"] == true),
        "nothing left open: {items:#?}"
    );
    let end = a.turn("bonjour").await;
    assert!(
        matches!(&end, TurnEnd::Finished(t) if t == "Unharnessed a reçu : bonjour"),
        "{end:?}"
    );
    a.stop().await;
    rt.shutdown();
}

/// What its stream tells of its context (`_meta.unharnessed.usage`: the tokens it holds, its window) is read by its
/// manifest's pointers (ESC-491); its cost, always 0 and `billed: false` in the study, is not read, nor are tokens in
/// and out, which it does not tell.
#[tokio::test(flavor = "multi_thread")]
async fn unharnessed_tells_what_its_context_holds_and_nothing_else_is_guessed() {
    let (rt, engine) = unharnessed("unharnessed-context");
    let dir = bench_dir("esc491-unharnessed-context");
    let mut a = unharnessed_agent(engine, &dir, None, |_| {});
    a.proc().initialize().await.unwrap();
    let end = a.turn("bonjour").await;
    assert!(matches!(&end, TurnEnd::Finished(_)), "{end:?}");
    assert_eq!((a.rt.context_tokens, a.rt.context_window), (1301, 32768));
    assert_eq!(a.rt.meta.cost, 0.0);
    assert_eq!(a.rt.meta.tokens, 0, "tokens in and out are not told");
    a.stop().await;
    rt.shutdown();
}

/// The streams of Unharnessed 0.9.8 recorded in the study's sandbox (tests/fixtures/unharnessed/: a fix with five cells
/// on a local model, a turn cancelled in its cell, a call of an MCP server from the kernel), through the client of its
/// manifest: each cell a tool « Python » with its code and its output, the cancelled one failed, the notifications of
/// its own (`_unharnessed/kernel/*`) and its `session_info_update` ignored.
#[test]
fn the_recorded_streams_of_unharnessed_make_python_cells_with_their_code() {
    let decl = unharnessed_decl();
    for (case, cells, last) in [
        (
            "full-turn",
            7,
            "The bug in the `add()` function in `calc.py` has been fixed and verified",
        ),
        ("cancel", 1, "Hello"),
        ("mcp", 1, "3 tickets"),
    ] {
        let flux =
            std::fs::read_to_string(fixtures().join("unharnessed").join(format!("{case}.jsonl")))
                .unwrap();
        let lines: Vec<Value> = flux
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        let meta = AgentMeta {
            id: crate::model::new_id(),
            cwd: "/home/tester/proj".into(),
            mode: crate::bypass::BYPASS.into(),
            ..Default::default()
        };
        let conv = crate::paths::test_dir(&format!("esc484-replay-{case}"));
        let mut rt = replayed_through_the_client(&decl, meta, &lines, &conv);
        let items = rt.conv.items();
        let tools: Vec<&Value> = items.iter().filter(|i| i["kind"] == "tool").collect();
        assert_eq!(tools.len(), cells, "{case}: {items:#?}");
        for t in &tools {
            assert_eq!(t["name"], "Python", "{case}: {t:#}");
            assert!(
                t["input"]["code"].as_str().is_some_and(|c| !c.is_empty()),
                "{case}: {t:#}"
            );
        }
        let texts: String = items
            .iter()
            .filter(|i| i["kind"] == "text")
            .filter_map(|i| i["text"].as_str())
            .collect();
        assert!(texts.contains(last), "{case}: {texts}");
        if case == "cancel" {
            assert!(
                tools[0]["result"]["isError"] == true
                    && tools[0]["result"]["text"]
                        .as_str()
                        .is_some_and(|t| t.contains("Tool execution aborted")),
                "{:#}",
                tools[0]
            );
        }
    }
}

/// The gap ESC-484 showed, closed by ESC-489: a harness whose one mode is Bypass has its agents in that mode, one asked in « Demander »
/// (`default`) too; the mode judged is the one it runs in, Bypass. Without « Autoriser le mode
/// Bypass » an agent of Unharnessed does not start, and the refusal says why; with it, it runs in Bypass.
#[test]
fn an_agent_of_a_harness_whose_one_mode_is_bypass_runs_in_bypass_and_only_when_it_is_allowed() {
    let (_rt, engine) = unharnessed("unharnessed-gap");
    crate::engine::registry::register(engine);
    let modes: Vec<&str> = engine.caps().modes.iter().map(|m| m.id).collect();
    assert_eq!(modes, [crate::bypass::BYPASS]);
    // Its new agents' mode, as the core gives it to an account that says none.
    let mode = crate::accounts::defaults_with(None, engine.id(), "").mode;
    assert_eq!(
        mode,
        crate::bypass::BYPASS,
        "its new agents are in the one mode it has"
    );
    // One asked in « Demander » (an account that says it, a handover) runs in Bypass too: judged as Bypass.
    let lang = crate::i18n::Lang::Fr;
    let err = crate::modes::judge(engine, "default", false, lang)
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("Unharnessed ne tourne qu’en Bypass")
            && err.contains("Autoriser le mode Bypass"),
        "{err}"
    );
    assert_eq!(
        crate::modes::judge(engine, "default", true, lang).unwrap(),
        crate::bypass::BYPASS
    );
    crate::engine::registry::unregister(engine.id());
}

/// What `proc/*` refuses to a plugin with a module, a plugin without code does not get by naming it in its manifest: its
/// harness does not start, and the conversation says which variable and why.
#[tokio::test(flavor = "multi_thread")]
async fn a_variable_proc_refuses_in_a_third_party_description_is_refused_with_its_reason() {
    let (rt, _, _) = runtime("tiers-env");
    for var in [
        "NODE_OPTIONS",
        "LD_PRELOAD",
        "DYLD_INSERT_LIBRARIES",
        "PATH",
        "path",
        "ESCOUADE_TOKEN",
    ] {
        let mut decl = fake_manifest()["contributes"]["harnesses"][0].clone();
        decl["acp"]["env"] = json!({ var: "--require ./x.js" });
        let engine = described(&rt, OWNER, decl);
        let dir = bench_dir(&format!("plg6-tiers-env-{var}"));
        let a = Driven::spawn_with(engine, &dir, |_| {});
        let err = a.proc().initialize().await.unwrap_err().to_string();
        assert!(err.contains(var), "{var}: {err}");
        assert!(err.contains("ne démarre pas"), "{var}: {err}");
        assert!(
            !dir.join("fake-acp.jsonl").exists(),
            "{var}: its program never ran"
        );
    }
    rt.shutdown();
}

/// The variables the core itself sets by a manifest's word (`accountVar`, `keyVar`) are judged too.
#[tokio::test(flavor = "multi_thread")]
async fn a_third_party_description_cannot_name_a_refused_variable_for_the_core_to_set() {
    let (rt, _, _) = runtime("tiers-compte");
    for field in ["accountVar", "keyVar"] {
        let mut decl = fake_manifest()["contributes"]["harnesses"][0].clone();
        decl[field] = json!("LD_PRELOAD");
        let engine = described(&rt, OWNER, decl);
        let dir = bench_dir(&format!("plg6-tiers-compte-{field}"));
        let meta = AgentMeta {
            id: crate::model::new_id(),
            cwd: dir.to_string_lossy().to_string(),
            ..Default::default()
        };
        let spawn = Spawn::of(&meta, fake_acp(), Vec::new());
        let err = engine
            .spawn(spawn, Box::new(|_| {}), Box::new(|_, _| {}))
            .err()
            .unwrap_or_else(|| panic!("{field}: it started"))
            .to_string();
        assert!(err.contains("LD_PRELOAD"), "{field}: {err}");
    }
    rt.shutdown();
}

/// Any other variable goes through for a third party, and the first hand is not judged.
#[tokio::test(flavor = "multi_thread")]
async fn a_plain_variable_of_a_third_party_and_any_of_the_first_hand_go_through() {
    let (rt, _, _) = runtime("env-permis");
    for (owner, var) in [
        (OWNER, "FAKE_ACP_COLOR"),
        ("dev.escouade.goose", "NODE_OPTIONS"),
    ] {
        let mut decl = fake_manifest()["contributes"]["harnesses"][0].clone();
        decl["acp"]["env"] = json!({ var: "" });
        let engine = described(&rt, owner, decl);
        let dir = bench_dir(&format!("plg6-env-permis-{var}"));
        let mut a = Driven::spawn_with(engine, &dir, |_| {});
        a.proc().initialize().await.unwrap();
        a.stop().await;
    }
    rt.shutdown();
}

/// « Un plugin tiers ne donne jamais de chemin » holds for `names` too, which are joined to each folder of the PATH: a
/// path there would be the program itself.
#[test]
fn a_third_party_names_no_path_among_the_names_of_its_program() {
    let (rt, _, _) = runtime("tiers-noms");
    let Program::Native(path) = fake_acp() else {
        unreachable!()
    };
    let path = path.to_string_lossy().into_owned();
    let relative = format!("..{}fake-acp", std::path::MAIN_SEPARATOR);
    let decl_with = |names: Vec<&str>| {
        let mut decl = fake_manifest()["contributes"]["harnesses"][0].clone();
        decl["locate"]["names"] = json!(names);
        decl
    };
    let third = described(&rt, OWNER, decl_with(vec![&path, &relative]));
    assert!(third.locate("", "").is_none(), "a path was run");
    let first = described(&rt, "dev.escouade.goose", decl_with(vec![&path]));
    assert!(first.locate("", "").is_some(), "the first hand may");
}

/// The manifest of the fake's plugin, read as a third party's, once `change` went by.
fn third_party_manifest(change: impl FnOnce(&mut Value)) -> crate::plugins::manifest::Manifest {
    let mut v = fake_manifest();
    change(&mut v["contributes"]["harnesses"][0]);
    crate::plugins::manifest::check(&v, crate::plugins::manifest::Hand::Third)
        .manifest
        .unwrap()
}

/// What runs is what the user was shown before installing, as `permissions.commands` shows each command: every name the
/// program may be found by, its variables, and what asks its version, which runs with no agent; and what else its
/// manifest says (SEC-30): the commands that install and update it, the variables it takes off.
#[test]
fn the_installation_shows_all_that_a_description_runs() {
    let plain = crate::plugins::asks(&third_party_manifest(|_| {}));
    assert_eq!(
        plain,
        [
            "harness:fake-acp acp",
            "harnessInstall:npm install -g fake-acp"
        ],
        "nothing more for the plain one"
    );
    let told = crate::plugins::asks(&third_party_manifest(|h| {
        h["install"] =
            json!({ "windows": "winget install fake-acp", "unix": "brew install fake-acp" });
        h["updateCommand"] = json!("fake-acp upgrade");
        h["acp"]["removeEnv"] = json!(["FAKE_ACP_TOKEN"]);
    }));
    assert_eq!(
        told,
        [
            "harness:fake-acp acp",
            "harnessInstall:winget install fake-acp",
            "harnessInstall:brew install fake-acp",
            "harnessUpdate:fake-acp upgrade",
            "harnessUnset:FAKE_ACP_TOKEN",
        ]
    );
    let asks = crate::plugins::asks(&third_party_manifest(|h| {
        h["locate"]["names"] = json!(["fake-acp", "faux"]);
        h["acp"]["env"] = json!({ "FAKE_ACP_COLOR": "bleu" });
        h["versionArgs"] = json!(["-e", "faire()"]);
    }));
    for word in [
        "harness:FAKE_ACP_COLOR=bleu fake-acp acp",
        "harness:FAKE_ACP_COLOR=bleu faux acp",
        "harness:fake-acp -e faire()",
        "harness:faux -e faire()",
    ] {
        assert!(asks.iter().any(|w| w == word), "{word} in {asks:?}");
    }
}

/// Its version is asked of the harnesses of the registry alone: nothing of a plugin runs before it is installed and on.
#[test]
fn the_version_of_a_description_is_asked_once_its_plugin_is_installed_and_on() {
    let (rt, plugins, _) = runtime("version");
    let id = harness::described_id(OWNER, "fake-acp");
    let listed = || crate::engine::registry::all().iter().any(|e| e.id() == id);
    let preview = plugins
        .inspect_folder(
            &fixtures().join("plugins").join("fake-acp"),
            crate::i18n::Lang::Fr,
        )
        .unwrap();
    assert!(!harness::sync(&rt, &plugins).contains(&id));
    assert!(!listed(), "previewed, it is not installed");
    plugins
        .install(&preview.token, crate::i18n::Lang::Fr)
        .unwrap();
    assert!(harness::sync(&rt, &plugins).contains(&id));
    assert!(listed());
    plugins.choose(OWNER, |c| c.enabled = false).unwrap();
    assert!(!harness::sync(&rt, &plugins).contains(&id));
    assert!(!listed(), "off, it is not asked");
}

/// A discussion of the Docs tab: the agent is put in its mode that changes nothing, and its writes are refused.
#[tokio::test(flavor = "multi_thread")]
async fn a_read_only_agent_is_put_in_its_read_only_mode_and_writes_nothing() {
    let (rt, engine) = fake("readonly");
    let dir = bench_dir("plg6-readonly-run");
    let mut a = Driven::spawn_with(engine, &dir, |s| s.read_only = true);
    let end = a.turn("fichier").await;
    assert!(
        matches!(&end, TurnEnd::Finished(t) if t.contains("refusée")),
        "{end:?}"
    );
    assert!(!dir.join("notes.txt").exists());
    let modes: Vec<Value> = a
        .logged()
        .into_iter()
        .filter(|e| e["dir"] == "c2s" && e["msg"]["method"] == "session/set_mode")
        .map(|e| e["msg"]["params"]["modeId"].clone())
        .collect();
    assert_eq!(modes, [json!("read")]);
    a.stop().await;
    rt.shutdown();
}

/// Escouade's MCP server goes to the session (`mcpServers` of `session/new`), its token in a header on the agent's
/// input: never among its arguments nor its variables.
#[tokio::test(flavor = "multi_thread")]
async fn no_engine_puts_the_token_on_the_command_line_the_harness_of_the_acp_plugin_neither() {
    let (rt, engine) = fake("token");
    let dir = bench_dir("plg6-token-run");
    let token = "s3cr3t-plg6-token";
    let mut a = Driven::spawn_with(engine, &dir, |s| {
        s.escouade = Escouade::Granted {
            url: "http://127.0.0.1:4110/mcp".into(),
            token: token.into(),
            file: None,
        }
    });
    let end = a.turn("serveurs").await;
    assert!(
        matches!(&end, TurnEnd::Finished(t) if t.contains("\"name\":\"escouade\"") && t.contains("Authorization")),
        "{end:?}"
    );
    let logged = a.logged();
    let launch = logged.iter().find_map(|e| e.get("launch")).unwrap();
    assert!(!launch.to_string().contains(token), "{launch}");
    let opened = logged
        .iter()
        .find(|e| e["dir"] == "c2s" && e["msg"]["method"] == "session/new")
        .unwrap();
    assert_eq!(
        opened["msg"]["params"]["mcpServers"][0]["headers"][0]["value"],
        format!("Bearer {token}")
    );
    a.stop().await;
    rt.shutdown();
}

/// Installed from its folder, the plugin's harness is in the registry; turned off, it is taken out and its agents are
/// « harness absent » (their data kept); on again, it is back.
#[test]
fn the_harness_comes_and_goes_with_its_plugin() {
    let (rt, plugins, _) = runtime("sync");
    let id = harness::described_id(OWNER, "fake-acp");
    let preview = plugins
        .inspect_folder(
            &fixtures().join("plugins").join("fake-acp"),
            crate::i18n::Lang::Fr,
        )
        .unwrap();
    assert!(
        preview.asks.iter().any(|w| w == "harness:fake-acp acp"),
        "the program its agents run is shown before it is installed: {:?}",
        preview.asks
    );
    plugins
        .install(&preview.token, crate::i18n::Lang::Fr)
        .unwrap();
    assert!(harness::sync(&rt, &plugins).contains(&id));
    assert_eq!(crate::engine::by_id(id).name(), "Faux ACP");
    plugins.choose(OWNER, |c| c.enabled = false).unwrap();
    assert!(!harness::sync(&rt, &plugins).contains(&id));
    let absent = crate::engine::by_id(id);
    assert_eq!(absent.name(), id.as_str(), "« harness absent »");
    assert!(absent.locate("", "").is_none());
    plugins.choose(OWNER, |c| c.enabled = true).unwrap();
    assert!(harness::sync(&rt, &plugins).contains(&id));
    // Without the generic ACP plugin, none is served.
    plugins.choose(builtin::ACP, |c| c.enabled = false).unwrap();
    assert!(!harness::sync(&rt, &plugins).contains(&id));
}

/// The real trial (ESC-178), on Goose described by `plugins/goose/` and a model of the machine (Ollama): ignored, run
/// by hand with its program and its own configuration folder, never the user's:
/// `ESCOUADE_GOOSE=…\goose.exe ESCOUADE_GOOSE_ROOT=…\root cargo nextest run --run-ignored only real_goose --no-capture`.
/// A reply, then a file written and a command run, each asked and allowed, the turn's tokens counted.
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn real_goose() {
    let (Ok(program), Ok(root)) = (
        std::env::var("ESCOUADE_GOOSE"),
        std::env::var("ESCOUADE_GOOSE_ROOT"),
    ) else {
        panic!(
            "ESCOUADE_GOOSE and ESCOUADE_GOOSE_ROOT name its program and its configuration folder"
        );
    };
    let (rt, _, _) = runtime("goose");
    let manifest: Value = serde_json::from_str(
        &std::fs::read_to_string(
            fixtures()
                .join("..")
                .join("..")
                .join("plugins")
                .join("goose")
                .join("escouade-plugin.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let engine = described(
        &rt,
        "dev.escouade.goose",
        manifest["contributes"]["harnesses"][0].clone(),
    );
    let dir = bench_dir("plg6-real-goose");
    let _ = std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(&dir)
        .status();
    let started = Instant::now();
    let meta = AgentMeta {
        id: crate::model::new_id(),
        cwd: dir.to_string_lossy().to_string(),
        model: String::new(),
        mode: "default".into(),
        ..Default::default()
    };
    let env = vec![("GOOSE_PATH_ROOT".to_string(), root.clone())];
    let spawn = Spawn::of(&meta, Program::Native(PathBuf::from(&program)), env);
    let (tx, mut heard) = mpsc::unbounded_channel();
    let exit = tx.clone();
    let proc: Process = engine
        .spawn(
            spawn,
            Box::new(move |f| {
                let _ = tx.send(Heard::Frame(f));
            }),
            Box::new(move |_, said| {
                let _ = exit.send(Heard::Exit(said));
            }),
        )
        .unwrap();
    let mut a = AgentRt::new(meta, &dir.join("conv"));
    a.attach(proc.clone());
    proc.initialize().await.unwrap();
    let mut turn = |text: &str| {
        let uid = uuid::Uuid::new_v4().to_string();
        proc.send_message(&uid, &engine.message(text, &[]).unwrap())
            .unwrap();
        let mut fx = Effects::default();
        a.push_user(&uid, text, 0, &[], &mut fx);
    };
    turn("Réponds seulement : bonjour");
    let mut ends = 0;
    let mut allowed = 0;
    let deadline = Instant::now() + Duration::from_secs(600);
    let mut second = false;
    while ends < 2 {
        let left = deadline.saturating_duration_since(Instant::now());
        match tokio::time::timeout(left, heard.recv()).await {
            Ok(Some(Heard::Frame(f))) => {
                let mut fx = Effects::default();
                engine.translate(&mut a, &f, &mut fx);
                if fx.turn_end.is_some() {
                    ends += 1;
                    eprintln!(
                        "turn {ends} ended after {:?}: {:?}",
                        started.elapsed(),
                        fx.turn_end
                    );
                    if !second {
                        second = true;
                        let uid = uuid::Uuid::new_v4().to_string();
                        let text = "Crée le fichier notes.txt qui contient exactement : bonjour. Puis lance la commande git status --short. Réponds en une phrase.";
                        proc.send_message(&uid, &engine.message(text, &[]).unwrap())
                            .unwrap();
                        a.push_user(&uid, text, 0, &[], &mut Effects::default());
                    }
                }
                let waiting: Vec<String> = a
                    .conv
                    .items()
                    .into_iter()
                    .filter(|i| i["kind"] == "permission" && i["decision"].is_null())
                    .filter_map(|i| i["id"].as_str().map(str::to_string))
                    .collect();
                for rid in waiting {
                    let mut fx = Effects::default();
                    if a.answer_permission_by(&rid, "allow", None, None, &mut fx)
                        .is_ok()
                    {
                        allowed += 1;
                    }
                }
            }
            Ok(Some(Heard::Exit(said))) => panic!("goose ended: {said}"),
            _ => panic!(
                "no end; the conversation: {:#}",
                Value::Array(a.conv.items())
            ),
        }
    }
    let items = a.conv.items();
    for i in &items {
        let mut i = i.clone();
        if let Some(t) = i["text"].as_str() {
            i["text"] = json!(t.chars().take(200).collect::<String>());
        }
        eprintln!("{i}");
    }
    assert!(items.iter().any(|i| i["kind"] == "text"
        && i["text"]
            .as_str()
            .is_some_and(|t| t.to_lowercase().contains("bonjour"))));
    assert!(allowed >= 2, "the write and the command asked: {allowed}");
    assert_eq!(
        std::fs::read_to_string(dir.join("notes.txt"))
            .unwrap()
            .trim(),
        "bonjour"
    );
    assert!(items
        .iter()
        .any(|i| i["kind"] == "tool" && i["name"] == "Bash" && i["status"] == "ok"));
    assert!(items
        .iter()
        .any(|i| i["kind"] == "tool" && i["name"] == "Write"));
    assert!(a.meta.tokens > 0, "its tokens counted");
    proc.kill();
    rt.shutdown();
}

/// The fence that holds compares the paths' components, not their text: a `..` that climbs into the private project,
/// a relative path (ACP wants them absolute) and, under Unix, a link of the agent's folder that points into it are
/// all refused.
#[tokio::test(flavor = "multi_thread")]
async fn the_fence_on_the_client_s_files_is_not_got_round_by_dots_a_relative_path_or_a_link() {
    let (rt, engine) = fake("fence-round");
    let dir = bench_dir("plg6-fence-round-run");
    let other = bench_dir("plg6-fence-round-other");
    std::fs::write(other.join("secret.txt"), "le secret").unwrap();
    std::fs::write(dir.join("ouvert.txt"), "ouvert").unwrap();
    let fence = crate::fence::Fence::new(vec![other.clone()]);
    let mut a = Driven::spawn_with(engine, &dir, |s| s.fence = fence);
    let refused = |end: &TurnEnd| matches!(end, TurnEnd::Finished(t) if t.contains("refusé") && !t.contains("le secret"));
    // Through `..`, from the agent's own folder.
    let name = other.file_name().unwrap().to_string_lossy().into_owned();
    let sep = std::path::MAIN_SEPARATOR;
    let climbed = format!("{}{sep}..{sep}{name}{sep}secret.txt", dir.display());
    let end = a.turn(&format!("lis {climbed}")).await;
    assert!(refused(&end), "{end:?}");
    // Relative: refused, whatever it names.
    let end = a.turn("lis ouvert.txt").await;
    assert!(
        matches!(&end, TurnEnd::Finished(t) if t.contains("not an absolute path")),
        "{end:?}"
    );
    // A file of its own folder, absolute, still reads.
    let end = a
        .turn(&format!("lis {}", dir.join("ouvert.txt").display()))
        .await;
    assert!(
        matches!(&end, TurnEnd::Finished(t) if t.contains("ouvert")),
        "{end:?}"
    );
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&other, dir.join("lien")).unwrap();
        let end = a
            .turn(&format!(
                "lis {}",
                dir.join("lien").join("secret.txt").display()
            ))
            .await;
        assert!(refused(&end), "a link into the private project: {end:?}");
        // A link to a folder of the private project, then `..`: the kernel would follow the link first and open the
        // project's own file; what is opened is the path checked, the agent's folder's.
        let sub = other.join("sous");
        std::fs::create_dir_all(&sub).unwrap();
        std::os::unix::fs::symlink(&sub, dir.join("lien-sous")).unwrap();
        let end = a
            .turn(&format!("lis {}/lien-sous/../secret.txt", dir.display()))
            .await;
        assert!(
            matches!(&end, TurnEnd::Finished(t) if !t.contains("le secret")),
            "the project's file read through link/..: {end:?}"
        );
    }
    a.stop().await;
    rt.shutdown();
}

/// INT-10.1: Cursor's effort goes by the option of its model, read from `cursor/list_available_models` (asked beside
/// the session), as the core's client set it: once the session is open and the list there, and at each change.
#[test]
fn cursors_effort_is_set_by_the_option_its_models_list_gives() {
    let decl: proto::HarnessDecl = serde_json::from_value(cursor_decl()).unwrap();
    let attach = proto::Attach {
        agent: "a".into(),
        cwd: "C:/wt".into(),
        model: "claude-opus-5-5".into(),
        effort: "high".into(),
        mode: "default".into(),
        lang: "fr".into(),
        ..proto::Attach::default()
    };
    let mut client = escouade_acp::Agent::new(&attach, &decl, VERSION);
    let launch = client
        .launch(&proto::LaunchParams {
            agent: "a".into(),
            cwd: "C:/wt".into(),
            model: "claude-opus-5-5".into(),
            effort: "high".into(),
            mode: "default".into(),
            lang: "fr".into(),
            ..proto::LaunchParams::default()
        })
        .unwrap();
    let sent = |lines: &[String]| -> Vec<Value> {
        lines
            .iter()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    };
    let writes = |items: Vec<Item>| -> Vec<Value> {
        items
            .into_iter()
            .filter_map(|i| match i {
                Item::Write { line } => Some(serde_json::from_str(&line).unwrap()),
                _ => None,
            })
            .collect()
    };
    let init = &sent(&launch.writes)[0];
    let hello = json!({ "jsonrpc": "2.0", "id": init["id"], "result": {
        "protocolVersion": 1, "agentCapabilities": { "loadSession": true },
    } });
    let asked = writes(client.frames(&[hello.to_string()]));
    let id_of = |m: &str| asked.iter().find(|w| w["method"] == m).unwrap()["id"].clone();
    let (new, list) = (id_of("session/new"), id_of("cursor/list_available_models"));
    let models = json!({ "jsonrpc": "2.0", "id": list, "result": { "models": [
        { "value": "claude-opus-5-5", "configOptions": [
            { "id": "effort", "category": "thought_level", "options": [{ "value": "low" }, { "value": "high" }] },
        ] },
    ] } });
    // Before the session: nothing to set yet.
    assert!(writes(client.frames(&[models.to_string()])).is_empty());
    let opened = json!({ "jsonrpc": "2.0", "id": new, "result": { "sessionId": "s1" } });
    let set = writes(client.frames(&[opened.to_string()]));
    let effort = |w: &Vec<Value>| -> Vec<Value> {
        w.iter()
            .filter(|w| w["params"]["configId"] == "effort")
            .map(|w| w["params"]["value"].clone())
            .collect()
    };
    assert_eq!(effort(&set), [json!("high")], "{set:?}");
    let changed = sent(&client.set_effort("low"));
    assert_eq!(effort(&changed), [json!("low")]);
}

/// The items of a resumed session do not take the ids of those of the process before it: each process of the agent
/// numbers its items from 1, under the tag of its key (found with the real Cursor CLI, INT-10.1, where the answer of a
/// resumed turn took the place of the first one).
#[test]
fn the_items_of_two_processes_of_an_agent_have_ids_of_their_own() {
    let decl: proto::HarnessDecl = serde_json::from_value(cursor_decl()).unwrap();
    let text_id = |key: &str| -> String {
        let attach = proto::Attach {
            agent: key.into(),
            cwd: "C:/wt".into(),
            lang: "fr".into(),
            ..proto::Attach::default()
        };
        let mut client = escouade_acp::Agent::new(&attach, &decl, VERSION);
        let launch = client
            .launch(&proto::LaunchParams {
                agent: key.into(),
                cwd: "C:/wt".into(),
                lang: "fr".into(),
                ..proto::LaunchParams::default()
            })
            .unwrap();
        let init: Value = serde_json::from_str(&launch.writes[0]).unwrap();
        let hello =
            json!({ "jsonrpc": "2.0", "id": init["id"], "result": { "protocolVersion": 1 } });
        let asked: Vec<Value> = client
            .frames(&[hello.to_string()])
            .into_iter()
            .filter_map(|i| match i {
                Item::Write { line } => serde_json::from_str(&line).ok(),
                _ => None,
            })
            .collect();
        let new = asked.iter().find(|w| w["method"] == "session/new").unwrap()["id"].clone();
        let opened = json!({ "jsonrpc": "2.0", "id": new, "result": { "sessionId": "s1" } });
        let chunk = json!({ "jsonrpc": "2.0", "method": "session/update", "params": { "sessionId": "s1", "update": {
            "sessionUpdate": "agent_message_chunk", "content": { "type": "text", "text": "A" },
        } } });
        client
            .frames(&[opened.to_string(), chunk.to_string()])
            .into_iter()
            .map(|i| serde_json::to_value(i).unwrap())
            .find_map(|i| {
                i["id"]
                    .as_str()
                    .filter(|id| id.starts_with("acp-text"))
                    .map(str::to_string)
            })
            .expect("a text")
    };
    assert_ne!(text_id("cursor-0a1b2c3d4e"), text_id("cursor-9f8e7d6c5b"));
}

/// What the core tells an agent of a refusal without a word (`agent::refused_action`) is not a message Vibe's client sends
/// once the turn is cut: it would start a turn nobody asked for (INT-10.6).
#[test]
fn a_refusal_without_a_word_starts_no_turn_of_vibe() {
    for lang in [crate::i18n::Lang::Fr, crate::i18n::Lang::En] {
        assert!(!escouade_acp::vibe::worded(&crate::agent::refused_action(
            lang
        )));
    }
    assert!(escouade_acp::vibe::worded("Pas ce dossier"));
}

/// SEC-05 (ESC-424): a launch sets the variables its plugin declares (`permissions.env`, the `acp.env` of a description),
/// those the core names itself, and nothing else; anyone's plugin never one of the families that would change the
/// program, even declared; a plugin of Escouade's own hand from before the field keeps what it did.
#[test]
fn a_launch_sets_what_its_plugin_declares_and_never_a_refused_family() {
    let third = harness::Vars {
        first_hand: false,
        declared: Some(vec!["OUTIL_MODE".into(), "GIT_CONFIG_COUNT".into()]),
        given: vec!["OUTIL_HOME"],
    };
    assert_eq!(third.refusal("Outil", "OUTIL_MODE"), None);
    assert_eq!(third.refusal("Outil", "OUTIL_HOME"), None);
    let undeclared = third.refusal("Outil", "AUTRE").unwrap();
    assert!(undeclared.contains("AUTRE"), "{undeclared}");
    for var in [
        "GIT_CONFIG_COUNT",
        "PYTHONSTARTUP",
        "HTTPS_PROXY",
        "HOME",
        "XDG_CONFIG_HOME",
    ] {
        let why = third
            .refusal("Outil", var)
            .unwrap_or_else(|| panic!("{var} passes"));
        assert!(why.contains(var), "{why}");
    }
    let nothing = harness::Vars {
        first_hand: false,
        declared: None,
        given: Vec::new(),
    };
    assert!(nothing.refusal("Outil", "OUTIL_MODE").is_some());
    let own = harness::Vars {
        first_hand: true,
        declared: Some(vec!["HOME".into()]),
        given: Vec::new(),
    };
    assert_eq!(own.refusal("Antigravity CLI", "HOME"), None);
    assert!(own.refusal("Antigravity CLI", "AUTRE").is_some());
    let before = harness::Vars {
        first_hand: true,
        declared: None,
        given: Vec::new(),
    };
    assert_eq!(before.refusal("Goose", "NODE_OPTIONS"), None);
}

/// SEC-05: what a third party's description declares is what its launch may set: its `acp.env`, the variable of its
/// account, Escouade's address of the hooks.
#[test]
fn the_variables_of_a_description_are_its_acp_env_and_those_the_core_names() {
    let (rt, _, _) = runtime("tiers-vars");
    let mut decl = fake_manifest()["contributes"]["harnesses"][0].clone();
    decl["acp"]["env"] = json!({ "FAKE_ACP_COLOR": "bleu" });
    let vars = described(&rt, OWNER, decl).vars();
    assert!(!vars.first_hand);
    assert_eq!(vars.declared, Some(vec!["FAKE_ACP_COLOR".to_string()]));
    assert!(vars.given.contains(&"FAKE_ACP_HOME"));
    rt.shutdown();
}

/// SEC-06 (ESC-424): a third party's harness does not run a shell nor an interpreter, whatever it names: its manifest is
/// refused, and a description that gets here finds no program; the first hand is not judged on it.
#[test]
fn a_third_party_harness_on_bash_is_refused() {
    let (rt, _, _) = runtime("tiers-bash");
    for name in [
        "bash",
        "sh",
        "powershell",
        "pwsh.exe",
        "cmd",
        "python3",
        "node",
        "osascript",
        "wscript",
    ] {
        let mut v = fake_manifest();
        v["contributes"]["harnesses"][0]["locate"]["names"] = json!([name]);
        v["contributes"]["harnesses"][0]["versionArgs"] = json!(["-c", "echo"]);
        let read = crate::plugins::manifest::check(&v, crate::plugins::manifest::Hand::Third);
        let problems = read.manifest.unwrap_err();
        assert!(
            problems
                .iter()
                .any(|p| p.path == "contributes.harnesses[0].locate.names[0]"
                    && p.code == crate::plugins::manifest::Code::Interpreter),
            "{name}: {problems:?}"
        );
        let engine = described(&rt, OWNER, v["contributes"]["harnesses"][0].clone());
        assert!(engine.locate("", "").is_none(), "{name} was found to run");
    }
    rt.shutdown();
}

/// SEC-01 (ESC-589): a third party's harness on `npx un-paquet` passes its manifest (no shell nor interpreter is named),
/// and its program does not run without the user's agreement to its whole command: neither an agent nor a question of
/// one shot. Agreed, its line runs; a line an update changes asks again; « Refuser » runs none. Escouade's own hand asks
/// nothing.
#[tokio::test(flavor = "multi_thread")]
async fn npx_un_paquet_does_not_start_without_the_users_agreement() {
    use crate::plugins::consent::{self, Held};
    let (rt, _, _) = runtime("sec01-npx");
    let mut v = fake_manifest();
    v["contributes"]["harnesses"][0]["locate"]["names"] = json!(["npx"]);
    v["contributes"]["harnesses"][0]["acp"]["args"] = json!(["un-paquet"]);
    let read = crate::plugins::manifest::check(&v, crate::plugins::manifest::Hand::Third);
    assert!(read.manifest.is_ok(), "{:?}", read.manifest.err());
    let decl = |v: &Value| -> proto::HarnessDecl {
        serde_json::from_value(v["contributes"]["harnesses"][0].clone()).unwrap()
    };
    let engine: &'static ProcessEngine = Box::leak(Box::new(ProcessEngine::described(
        rt.clone(),
        OWNER,
        decl(&v),
    )));
    let npx = Program::Native(bench_dir("sec01-npx-bin").join("npx.cmd"));
    let line = engine.launch_line(&npx).expect("a third party's line");
    assert_eq!(line.program, npx.describe());
    assert_eq!(line.args, ["un-paquet"]);
    assert_eq!(line.plugin, OWNER);
    assert_eq!(line.escouade_env, ["FAKE_ACP_HOME"]);
    consent::set(false, std::collections::HashMap::new());
    let dir = bench_dir("sec01-npx-run");
    let meta = AgentMeta {
        id: crate::model::new_id(),
        cwd: dir.to_string_lossy().into_owned(),
        ..Default::default()
    };
    let spawned = engine.spawn(
        Spawn::of(&meta, npx.clone(), Vec::new()),
        Box::new(|_| {}),
        Box::new(|_, _| {}),
    );
    let err = match spawned {
        Ok(_) => panic!("npx un-paquet started without the user's agreement"),
        Err(e) => e.to_string(),
    };
    assert!(err.contains("attend ton accord"), "{err}");
    let ask = crate::engine::OneShot {
        program: npx.clone(),
        env: Vec::new(),
        weight: crate::engine::Weight::Light,
        cwd: &dir,
        system: "Réponds court.",
        prompt: "bonjour",
        limit: Duration::from_secs(30),
        model: String::new(),
    };
    let err = engine.one_shot(ask).await.unwrap_err().to_string();
    assert!(err.contains("attend ton accord"), "{err}");
    // Agreed: that line runs; the line of an update that adds an argument asks again.
    consent::agree(&line);
    assert_eq!(consent::check(&line), Ok(()));
    let mut updated = v.clone();
    updated["contributes"]["harnesses"][0]["acp"]["args"] = json!(["un-paquet", "--encore"]);
    let next: &'static ProcessEngine = Box::leak(Box::new(ProcessEngine::described(
        rt.clone(),
        OWNER,
        decl(&updated),
    )));
    let changed = next.launch_line(&npx).unwrap();
    assert_ne!(changed.fingerprint, line.fingerprint);
    assert_eq!(consent::check(&changed), Err(Held::Asks));
    // « Refuser »: none runs, agreed or not.
    consent::set(
        true,
        std::collections::HashMap::from([(line.engine, vec![line.fingerprint.clone()])]),
    );
    assert_eq!(consent::check(&line), Err(Held::Refused));
    // Escouade's own hand: no line to agree to.
    let own: &'static ProcessEngine = Box::leak(Box::new(ProcessEngine::described(
        rt.clone(),
        "dev.escouade.goose",
        decl(&v),
    )));
    assert!(own.launch_line(&npx).is_none());
    rt.shutdown();
}

/// SEC-06 (ESC-424): the `versionArgs` of a third party's harness run after a gesture of the user's on it (an agent or a
/// terminal of it started), neither once it is installed nor at a start.
#[tokio::test(flavor = "multi_thread")]
async fn the_version_of_a_third_party_harness_is_asked_after_a_gesture_alone() {
    let (rt, _, _) = runtime("tiers-geste");
    let (_, engine) = fake("tiers-geste-moteur");
    assert!(engine.version_waits_gesture());
    let program = fake_acp();
    assert_eq!(
        crate::engine::registry::version(engine, &program).await,
        None
    );
    crate::engine::registry::gesture(engine.id());
    assert_eq!(
        crate::engine::registry::version(engine, &program)
            .await
            .as_deref(),
        Some("1.2.0")
    );
    // Escouade's own hand: asked as it was.
    let first = described(
        &rt,
        "dev.escouade.goose",
        fake_manifest()["contributes"]["harnesses"][0].clone(),
    );
    assert!(!first.version_waits_gesture());
    rt.shutdown();
}

/// SEC-30: what a launch of anyone's plugin takes off its harness's environment is held to what it could set: never a
/// family refused it (the proxy, the authorities, `ESCOUADE_*`…) nor a variable the core gives itself (the account's, the
/// hooks' address); Escouade's own hand takes off what it says (Copilot CLI, its tokens).
#[test]
fn a_launch_of_anyone_takes_off_nothing_the_core_gives_nor_a_refused_family() {
    let named = || -> Vec<String> {
        [
            "OUTIL_TOKEN",
            "HTTPS_PROXY",
            "https_proxy",
            "NO_PROXY",
            "NODE_EXTRA_CA_CERTS",
            "CODEX_CA_CERTIFICATE",
            "SSL_CERT_FILE",
            "OUTIL_HOME",
            "ESCOUADE_HOOKS_URL",
            "LD_PRELOAD",
        ]
        .iter()
        .map(|v| v.to_string())
        .collect()
    };
    let third = harness::Vars {
        first_hand: false,
        declared: Some(Vec::new()),
        given: vec!["OUTIL_HOME"],
    };
    let mut removed = named();
    third.hold_removed("Outil", &mut removed);
    assert_eq!(removed, ["OUTIL_TOKEN"]);
    let own = harness::Vars {
        first_hand: true,
        declared: None,
        given: vec!["OUTIL_HOME"],
    };
    let mut removed = named();
    own.hold_removed("Copilot CLI", &mut removed);
    assert_eq!(removed, named());
    // Its next program as its first.
    let launch = proto::Launch {
        remove_env: named(),
        ..proto::Launch::default()
    };
    let next = harness::next_launch("Outil", &third, &launch, false).unwrap();
    assert_eq!(next.remove_env, ["OUTIL_TOKEN"]);
}

/// SEC-30: a third party's description that names the proxy, the authorities and the account's variable in its
/// `removeEnv` (a manifest the core did not check) still runs its harness with them; its plain variable goes. The first
/// hand takes off all it names.
#[tokio::test(flavor = "multi_thread")]
async fn the_harness_of_anyone_keeps_the_proxy_and_the_authorities_escouade_gives() {
    let (rt, _, _) = runtime("tiers-retrait");
    for (owner, kept) in [(OWNER, true), ("dev.escouade.goose", false)] {
        let mut decl = fake_manifest()["contributes"]["harnesses"][0].clone();
        decl["acp"]["removeEnv"] = json!([
            "HTTPS_PROXY",
            "NODE_EXTRA_CA_CERTS",
            "FAKE_ACP_HOME",
            "FAKE_ACP_PLAIN"
        ]);
        let engine = described(&rt, owner, decl);
        let dir = bench_dir(&format!("plg6-tiers-retrait-{kept}"));
        let mut a = Driven::spawn_with(engine, &dir, |s| {
            s.env.extend([
                ("HTTPS_PROXY".into(), "http://proxy.invalid:3128".into()),
                ("NODE_EXTRA_CA_CERTS".into(), "C:/ca.pem".into()),
                ("FAKE_ACP_PLAIN".into(), "oui".into()),
            ]);
        });
        let end = a.turn("bonjour").await;
        assert!(matches!(end, TurnEnd::Finished(_)), "{owner}: {end:?}");
        let logged = a.logged();
        let launch = logged
            .iter()
            .find_map(|e| e.get("launch"))
            .unwrap_or_else(|| panic!("{owner}: no launch in {logged:?}"))
            .clone();
        let has = |k: &str| launch[k].is_string();
        assert_eq!(has("proxy"), kept, "{owner}: {launch}");
        assert_eq!(has("authorities"), kept, "{owner}: {launch}");
        assert_eq!(has("home"), kept, "{owner}: {launch}");
        assert!(
            !has("plain"),
            "{owner}: its own variable is taken off: {launch}"
        );
        a.stop().await;
    }
    rt.shutdown();
}

/// SEC-30: the commands that install and update a harness of anyone's are said to come from its plugin, by its id,
/// wherever the app shows them (the window's declaration, the refusal of a version too old); the first hand's are said
/// as before.
#[test]
fn the_commands_of_a_third_party_harness_are_said_to_be_its_plugins() {
    let (rt, _, _) = runtime("tiers-commandes");
    let mut decl = fake_manifest()["contributes"]["harnesses"][0].clone();
    decl["updateCommand"] = json!("npm install -g fake-acp@latest");
    let third = described(&rt, OWNER, decl.clone());
    assert_eq!(third.commands_from().as_deref(), Some(OWNER));
    assert_eq!(
        crate::engine::registry::EngineDecl::of(third)
            .install_from
            .as_deref(),
        Some(OWNER)
    );
    let said = crate::engine::registry::too_old(crate::i18n::Lang::Fr, third, "0.9.0");
    assert!(
        said.contains(&format!(
            "commande donnée par le plugin {OWNER}, pas par Escouade : npm install -g fake-acp@latest"
        )),
        "{said}"
    );
    let said = crate::engine::registry::too_old(crate::i18n::Lang::En, third, "0.9.0");
    assert!(
        said.contains(&format!("the plugin {OWNER}, not by Escouade")),
        "{said}"
    );
    let own = described(&rt, "dev.escouade.goose", decl);
    assert_eq!(own.commands_from(), None);
    assert_eq!(
        crate::engine::registry::EngineDecl::of(own).install_from,
        None
    );
    let said = crate::engine::registry::too_old(crate::i18n::Lang::Fr, own, "0.9.0");
    assert!(
        said.ends_with("Mets-le à jour (npm install -g fake-acp@latest)."),
        "{said}"
    );
    rt.shutdown();
}

/// W-32 of the audit of 2026-11 (ESC-418, SEC-30), turned around: the manifest of anyone's plugin that takes the proxy
/// and the authorities off its harness is refused, each with the reason of the same variable set; what the installation
/// shows of the harness says its two commands and the variables it takes off; the app says those commands are its
/// plugin's; and a launch that names the proxy among what it takes off keeps the proxy of the settings.
#[test]
fn w_32_what_a_described_harness_words_is_shown_and_judged() {
    use crate::plugins::manifest::{self, Hand};
    let install = "AUDIT-POC: a line to paste in a terminal";
    let update = "AUDIT-POC: another line to paste";
    let taken_off = [
        "HTTPS_PROXY",
        "HTTP_PROXY",
        "NO_PROXY",
        "NODE_EXTRA_CA_CERTS",
        "SSL_CERT_FILE",
    ];
    let manifest_of = |acp: Value| {
        json!({
            "id": "dev.exemple.audit",
            "name": "Audit",
            "version": "1.0.0",
            "escouade": { "api": "1", "min": "2.2.0" },
            "contributes": { "harnesses": [{
                "id": "audit",
                "name": "Audit",
                "locate": { "names": ["audit-poc-harness-that-is-not-installed"] },
                "tested": { "oldest": "9.0.0", "newest": "9.0.0" },
                "install": { "windows": install, "unix": install },
                "updateCommand": update,
                "acp": acp,
            }] },
        })
    };
    // 1. Refused to a third party, each variable at its place and for the reason of the same one set.
    let with = manifest_of(json!({ "args": ["acp"], "removeEnv": taken_off }));
    let problems = manifest::check(&with, Hand::Third)
        .manifest
        .expect_err("the manifest that takes the proxy off is refused");
    let said = format!("{problems:?}");
    for (i, name) in taken_off.iter().enumerate() {
        assert!(
            said.contains(&format!("contributes.harnesses[0].acp.removeEnv[{i}]")),
            "{name}: {said}"
        );
        let set = manifest_of(json!({ "args": ["acp"], "env": { *name: "x" } }));
        let set = format!("{:?}", manifest::check(&set, Hand::Third).manifest.err());
        assert!(set.contains("Variable"), "{name}: {set}");
    }
    assert!(said.contains("Variable"), "{said}");
    // A plain variable taken off holds, and the first hand is not judged.
    let plain = manifest_of(json!({ "args": ["acp"], "removeEnv": ["AUDIT_TOKEN"] }));
    let m = manifest::check(&plain, Hand::Third).manifest.unwrap();
    assert!(manifest::check(&with, Hand::First).manifest.is_ok());
    // 2. The installation shows its two commands and what it takes off, with what it runs.
    let asks = crate::plugins::asks(&m);
    for word in [
        format!("harnessInstall:{install}"),
        format!("harnessUpdate:{update}"),
        "harnessUnset:AUDIT_TOKEN".to_string(),
        "harness:audit-poc-harness-that-is-not-installed acp".to_string(),
    ] {
        assert!(asks.contains(&word), "{word} in {asks:?}");
    }
    assert_eq!(
        asks.iter()
            .filter(|w| w.starts_with("harnessInstall:"))
            .count(),
        1,
        "the same command on every system, once: {asks:?}"
    );
    assert!(!crate::plugins::revocable(&format!(
        "harnessInstall:{install}"
    )));
    // 3. The app says those commands are its plugin's.
    let decl = harness::harnesses_of(&m.contributes).remove(0);
    let (rt, _, _) = runtime("w32");
    let engine: &'static ProcessEngine = Box::leak(Box::new(ProcessEngine::described(
        rt.clone(),
        "dev.exemple.audit",
        decl,
    )));
    let told = engine.install();
    assert_eq!((told.windows, told.unix), (install, install));
    let declared = crate::engine::registry::EngineDecl::of(engine);
    assert_eq!(declared.install_from.as_deref(), Some("dev.exemple.audit"));
    let too_old = crate::engine::registry::too_old(crate::i18n::Lang::En, engine, "1.0.0");
    assert!(
        too_old.contains(&format!(
            "a command given by the plugin dev.exemple.audit, not by Escouade: {update}"
        )),
        "{too_old}"
    );
    rt.shutdown();
    // 4. A launch that names the proxy and the authorities keeps those of the settings.
    let vars = harness::Vars {
        first_hand: false,
        declared: Some(Vec::new()),
        given: Vec::new(),
    };
    let mut removed: Vec<String> = taken_off.iter().map(|v| v.to_string()).collect();
    vars.hold_removed("Audit", &mut removed);
    assert!(removed.is_empty(), "{removed:?}");
}

/// Prime Agent's, on the same bench (ESC-485).
#[path = "plugins_prime_agent_tests.rs"]
mod prime_agent;

/// SEC-02 (ESC-590): a harness a third party describes does not start for an agent of a private project, whatever its
/// launch would say of the fence: refused before its program runs, said; the same harness of Escouade's own hand is
/// not judged so.
#[tokio::test(flavor = "multi_thread")]
async fn a_third_party_description_does_not_start_for_an_agent_of_a_private_project() {
    let (rt, _, _) = runtime("tiers-prive");
    let decl = fake_manifest()["contributes"]["harnesses"][0].clone();
    let engine = described(&rt, OWNER, decl);
    let dir = bench_dir("plg6-tiers-prive");
    let meta = AgentMeta {
        id: crate::model::new_id(),
        cwd: dir.to_string_lossy().to_string(),
        ..Default::default()
    };
    let mut spawn = Spawn::of(&meta, fake_acp(), Vec::new());
    spawn.private = true;
    let err = engine
        .spawn(spawn, Box::new(|_| {}), Box::new(|_, _| {}))
        .err()
        .expect("it started")
        .to_string();
    assert!(
        err.starts_with(
            "Un harness d’un plugin tiers ne peut pas garantir la clôture d’un projet privé"
        ),
        "{err}"
    );
    assert!(
        !dir.join("fake-acp.jsonl").exists(),
        "its program never ran"
    );
    rt.shutdown();
}
