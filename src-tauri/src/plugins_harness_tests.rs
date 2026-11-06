//! Tests of the contract of the harnesses in process (INT-9, ESC-221; `docs/PLUGINS.md` §4.5 and §4.6), on the
//! demonstration plugin `dev.escouade.minimal`, whose harness « Minimal » is the minimal fake engine of the bench made a
//! plugin: the conformance bench played through `ProcessEngine` as through the harnesses written in the core, the
//! launch described by the plugin and checked by the core, a crash of the plugin in the middle of a turn (once, then
//! for good), the one-shot question, the terminal view's launch and its hooks both ways, the memory declared, and the
//! measure against INT-0.

use crate::agent::{AgentRt, Effects};
use crate::board::TurnEnd;
use crate::engine::bench::{bench_dir, run, Scenario, Target};
use crate::engine::{Engine, Escouade, Process, Program, Spawn};
use crate::integrations::secrets::MemoryStore;
use crate::model::{AgentMeta, AgentStatus};
use crate::paths::DataDir;
use crate::plugins::builtin;
use crate::plugins::harness::{self, check_launch, ProcessEngine};
use crate::plugins::runtime::{Limits, PhaseName, Runtime, Stop};
use crate::plugins::Plugins;
use escouade_plugin_protocol::engine::{self as proto, EnvVar, FileToWrite, HookReply};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

const ID: &str = "dev.escouade.minimal";
const VERSION: &str = "2.2.0";

/// Where the build put its programs (`escouade-plugins` among them): the folder of the tests' own, `deps/`, is beside
/// it.
fn app_dir() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    let dir = exe.parent().unwrap().parent().unwrap().to_path_buf();
    assert!(
        dir.join(builtin::program_name()).is_file(),
        "{} is not built beside the tests (cargo build --bin escouade-plugins)",
        dir.join(builtin::program_name()).display()
    );
    dir
}

fn limits(restarts: usize) -> Limits {
    Limits {
        initialize: Duration::from_secs(20),
        shutdown: Duration::from_secs(2),
        restarts,
        ..Limits::default()
    }
}

/// The runtime of a data folder of its own, the demonstration plugin in it, and its harness.
fn minimal(name: &str, restarts: usize) -> (Arc<Runtime>, &'static ProcessEngine) {
    let data = DataDir::new(bench_dir(&format!("int9-{name}")).join("data"));
    let plugins = Plugins::load_in(data.clone(), VERSION, app_dir());
    let rt = Runtime::new(
        plugins.clone(),
        Arc::new(MemoryStore::default()),
        &data,
        VERSION,
        limits(restarts),
    );
    let m = plugins
        .get(ID, crate::i18n::Lang::En)
        .and_then(|v| v.manifest)
        .expect("the demonstration plugin");
    let decl = harness::harnesses_of(&m.contributes)
        .into_iter()
        .next()
        .expect("its harness declared");
    let engine: &'static ProcessEngine = Box::leak(Box::new(ProcessEngine::new(
        rt.clone(),
        ID,
        decl,
        &app_dir(),
    )));
    (rt, engine)
}

/// The harness « Minimal » in process, on the bench.
struct InProcess(&'static ProcessEngine);

impl Target for InProcess {
    fn engine(&self) -> &'static dyn Engine {
        self.0
    }

    fn program(&self) -> Program {
        self.0.locate("", "").expect("its program, the app's own")
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
                Scenario::PlanMode | Scenario::Limit | Scenario::Fork | Scenario::Quota => {
                    return None
                }
            }
            .to_string(),
        )
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_minimal_harness_in_process_passes_the_bench_as_the_one_written_in_the_core() {
    let (rt, engine) = minimal("bench", 3);
    let dir = bench_dir("int9-bench-run");
    let played = run(&InProcess(engine), &dir).await;
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
        ],
        "the scenarios of the in-process fake (`engine/minimal/`)"
    );
    assert_eq!(rt.status(ID).restarts, 0);
    rt.shutdown();
}

/// What a process says, as the engine's callbacks pass it on.
enum Heard {
    Frame(Value),
    Exit(String),
}

/// An agent of the harness, driven as the core drives it.
struct Agent {
    engine: &'static ProcessEngine,
    rt: AgentRt,
    heard: mpsc::UnboundedReceiver<Heard>,
    ends: Vec<TurnEnd>,
    exit: Option<String>,
}

fn meta(dir: &Path) -> AgentMeta {
    AgentMeta {
        id: crate::model::new_id(),
        cwd: dir.to_string_lossy().to_string(),
        model: "grand".into(),
        mode: "default".into(),
        ..Default::default()
    }
}

impl Agent {
    fn spawn_with(engine: &'static ProcessEngine, dir: &Path, f: impl FnOnce(&mut Spawn)) -> Agent {
        let meta = meta(dir);
        let mut spawn = Spawn::of(&meta, engine.locate("", "").unwrap(), Vec::new());
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
        Agent {
            engine,
            rt,
            heard,
            ends: Vec::new(),
            exit: None,
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

    /// Translates what the process says until `pred` holds.
    async fn until(&mut self, what: &str, pred: impl Fn(&mut Agent) -> bool) {
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
                Ok(Some(Heard::Exit(said))) => self.exit = Some(said),
                _ => panic!(
                    "no {what}; the conversation: {:#}",
                    Value::Array(self.rt.conv.items())
                ),
            }
        }
    }

    fn items(&mut self) -> Vec<Value> {
        self.rt.conv.items()
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

/// The plugin crashes in the middle of a turn: its harness goes on, the frames read meanwhile wait, the plugin starts
/// again and is given the agent and the batch again; the conversation says so, and the turn ends as it would have,
/// the call left under way closed by its end.
#[tokio::test(flavor = "multi_thread")]
async fn a_crash_of_the_plugin_in_the_middle_of_a_turn_restarts_it_and_the_turn_goes_on() {
    let (rt, engine) = minimal("crash", 3);
    let dir = bench_dir("int9-crash-run");
    let mut a = Agent::spawn_with(engine, &dir, |_| {});
    a.proc().initialize().await.unwrap();
    let harness_pids = a.proc().pids();
    a.send("plantage");
    a.until("the end of the turn", |a| !a.ends.is_empty()).await;
    assert!(
        matches!(&a.ends[0], TurnEnd::Finished(t) if t.contains("Après le plantage")),
        "{:?}",
        a.ends
    );
    let items = a.items();
    let texts: Vec<&str> = items
        .iter()
        .filter(|i| i["kind"] == "text")
        .filter_map(|i| i["text"].as_str())
        .collect();
    assert!(texts.contains(&"Avant le plantage."), "{texts:?}");
    assert!(texts.contains(&"Après le plantage."), "{texts:?}");
    assert_eq!(
        texts.iter().filter(|t| **t == "Avant le plantage.").count(),
        1,
        "nothing given twice"
    );
    assert!(
        items
            .iter()
            .any(|i| i["kind"] == "notice" && i["text"].as_str().unwrap().contains("a redémarré")),
        "{items:#?}"
    );
    let tool = items.iter().find(|i| i["kind"] == "tool").unwrap();
    assert_ne!(
        tool["status"], "running",
        "closed by the turn's end: {tool}"
    );
    assert_eq!(a.rt.meta.status, AgentStatus::Done);
    assert_eq!(rt.status(ID).restarts, 1);
    assert!(a.proc().is_alive(), "the harness outlived its plugin");
    // The same harness (its console's host may have joined its job since).
    assert!(
        a.proc().pids().contains(&harness_pids[0]),
        "the same harness"
    );
    // And it keeps going, its plugin started again.
    a.send("bonjour");
    a.until("the next turn", |a| a.ends.len() == 2).await;
    a.stop().await;
    rt.shutdown();
}

/// The plugin crashes once more than it may: it is stopped, and so is the agent's harness, the conversation and the
/// end of its process saying why; the session is kept for the next start.
#[tokio::test(flavor = "multi_thread")]
async fn a_plugin_stopped_after_its_crashes_stops_its_agents_and_says_why() {
    let (rt, engine) = minimal("stopped", 1);
    let dir = bench_dir("int9-stopped-run");
    let mut a = Agent::spawn_with(engine, &dir, |_| {});
    a.proc().initialize().await.unwrap();
    // A first turn makes the session known: in the turn that crashes, its `start` may come in the same batch as the
    // crash, which the plugin never translates (under macOS, every time).
    a.send("bonjour");
    a.until("the first turn", |a| a.ends.len() == 1).await;
    let session = a.rt.meta.session_id.clone();
    assert!(session.is_some(), "the session of the first turn");
    a.send("plantage-toujours");
    a.until("the end of the process", |a| a.exit.is_some())
        .await;
    let said = a.exit.clone().unwrap();
    assert!(said.contains("plantages"), "{said}");
    assert!(
        matches!(rt.status(ID).stop, Some(Stop::Crashed { .. })),
        "{:?}",
        rt.status(ID)
    );
    let items = a.items();
    assert!(
        items
            .iter()
            .any(|i| i["kind"] == "notice" && i["level"] == "error"),
        "{items:#?}"
    );
    assert_eq!(a.rt.meta.session_id, session, "its session is kept");
    assert!(!a.proc().is_alive());
    rt.shutdown();
}

/// A launch of a private project's agent, given Escouade's server, in a discussion (read-only): the plugin spells the
/// fence and read-only, the token in a variable, and the core lets it start.
#[tokio::test(flavor = "multi_thread")]
async fn a_fenced_read_only_launch_with_escouade_s_server_starts_once_checked() {
    let (rt, engine) = minimal("fenced", 3);
    let dir = bench_dir("int9-fenced-run");
    let private = bench_dir("int9-fenced-private");
    let mut a = Agent::spawn_with(engine, &dir, |s| {
        s.fence = crate::fence::Fence::new(vec![private.clone()]);
        s.read_only = true;
        s.escouade = Escouade::Granted {
            url: "http://127.0.0.1:1/mcp".into(),
            token: "jeton-secret-de-l-agent".into(),
            file: None,
        };
    });
    a.proc().initialize().await.unwrap();
    a.send("bonjour");
    a.until("the end of the turn", |a| !a.ends.is_empty()).await;
    a.stop().await;
    rt.shutdown();
}

fn launch(args: &[&str]) -> proto::Launch {
    proto::Launch {
        args: args.iter().map(|a| a.to_string()).collect(),
        fence_applied: true,
        ..proto::Launch::default()
    }
}

#[test]
fn the_core_refuses_a_launch_with_the_token_among_its_arguments() {
    let ok = launch(&["minimal-harness"]);
    assert!(check_launch("Minimal", true, Some("tok-123"), &[], false, None, &ok).is_ok());
    let bad = launch(&["minimal-harness", "--token=tok-123"]);
    let why = check_launch("Minimal", true, Some("tok-123"), &[], false, None, &bad).unwrap_err();
    assert!(why.contains("jeton"), "{why}");
    let mut in_env = ok.clone();
    in_env.env.push(EnvVar {
        name: "T".into(),
        value: "tok-123".into(),
    });
    assert!(
        check_launch("Minimal", true, Some("tok-123"), &[], false, None, &in_env).is_ok(),
        "a variable may hold it"
    );
}

#[test]
fn the_core_refuses_a_launch_that_does_not_apply_the_fence() {
    let fence = vec![
        "C:/projets/prive".to_string(),
        "C:/projets/autre".to_string(),
    ];
    let mut not_said = launch(&["minimal-harness"]);
    not_said.fence_applied = false;
    assert!(check_launch("Minimal", true, None, &fence, false, None, &not_said).is_err());
    let one = launch(&["minimal-harness", "--deny", "C:/projets/prive"]);
    let why = check_launch("Minimal", true, None, &fence, false, None, &one).unwrap_err();
    assert!(why.contains("clôture"), "a refused path left out: {why}");
    let home = if cfg!(windows) {
        r"C:\agents\a1"
    } else {
        "/agents/a1"
    };
    let mut all = launch(&["minimal-harness", "--deny", "C:/projets/prive"]);
    all.files.push(FileToWrite {
        path: Path::new(home)
            .join("settings.json")
            .to_string_lossy()
            .into_owned(),
        text: r#"{"deny":["C:/projets/autre"]}"#.into(),
    });
    assert!(
        check_launch("Minimal", true, None, &fence, false, Some(home), &all).is_ok(),
        "spelled in its arguments and its files"
    );
}

/// The word of a plugin that it holds the fence itself, on the files its harness reads and writes through it
/// (`fence_by_client`), is taken from a plugin the app gives only (the generic ACP plugin): a plugin of anyone else's
/// that says it keeps the rule, the fence applied and each of its paths in the launch.
#[test]
fn only_a_plugin_the_app_gives_may_hold_the_fence_itself() {
    let fence = vec!["C:/projets/prive".to_string()];
    let mut by_client = launch(&["agent", "acp"]);
    by_client.fence_applied = false;
    by_client.fence_by_client = true;
    assert!(check_launch("ACP", true, None, &fence, false, None, &by_client).is_ok());
    let why = check_launch("Tiers", false, None, &fence, false, None, &by_client).unwrap_err();
    assert!(why.contains("clôture"), "{why}");
    // Spelled in its launch, as any plugin's, it goes.
    let mut spelled = launch(&["agent", "--deny", "C:/projets/prive"]);
    spelled.fence_by_client = true;
    assert!(check_launch("Tiers", false, None, &fence, false, None, &spelled).is_ok());
}

#[test]
fn the_core_refuses_a_launch_that_does_not_hold_read_only() {
    let plain = launch(&["minimal-harness"]);
    let why = check_launch("Minimal", true, None, &[], true, None, &plain).unwrap_err();
    assert!(why.contains("lecture seule"), "{why}");
    let mut held = plain.clone();
    held.read_only = true;
    assert!(check_launch("Minimal", true, None, &[], true, None, &held).is_ok());
}

#[test]
fn the_core_refuses_a_file_written_outside_the_agent_s_own_folder() {
    let home = if cfg!(windows) {
        "C:\\agents\\a1"
    } else {
        "/agents/a1"
    };
    let file = |path: &str| {
        let mut l = launch(&[]);
        l.files.push(FileToWrite {
            path: path.into(),
            text: "{}".into(),
        });
        l
    };
    let inside = Path::new(home).join("config.json");
    assert!(check_launch(
        "M",
        true,
        None,
        &[],
        false,
        Some(home),
        &file(&inside.to_string_lossy())
    )
    .is_ok());
    let above = Path::new(home).join("..").join("a2").join("config.json");
    assert!(check_launch(
        "M",
        true,
        None,
        &[],
        false,
        Some(home),
        &file(&above.to_string_lossy())
    )
    .is_err());
    assert!(check_launch(
        "M",
        true,
        None,
        &[],
        false,
        None,
        &file(&inside.to_string_lossy())
    )
    .is_err());
    assert!(check_launch(
        "M",
        true,
        None,
        &[],
        false,
        Some(home),
        &file("config.json")
    )
    .is_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_one_shot_question_is_described_by_the_plugin_run_by_the_core_and_read_by_the_plugin() {
    let (rt, engine) = minimal("oneshot", 3);
    let dir = bench_dir("int9-oneshot-run");
    let answer = engine
        .one_shot(crate::engine::OneShot {
            program: engine.locate("", "").unwrap(),
            env: Vec::new(),
            weight: crate::engine::Weight::Light,
            cwd: &dir,
            system: "Nomme",
            prompt: "ce ticket",
            limit: Duration::from_secs(30),
            model: String::new(),
        })
        .await
        .unwrap();
    assert_eq!(answer, "Réponse courte : Nomme ce ticket");
    rt.shutdown();
}

/// The terminal view: its launch is described by the plugin and checked by the core as an agent's; its hooks go both
/// ways, the core's answer (context for the model, a refusal) put in the harness's form by the plugin.
#[tokio::test(flavor = "multi_thread")]
async fn a_terminal_view_is_launched_from_the_plugin_s_description_and_its_hooks_go_both_ways() {
    let (rt, engine) = minimal("terminal", 3);
    let params = proto::TerminalParams {
        agent: "a1".into(),
        cwd: "C:/projets/demo".into(),
        session: Some("min-1".into()),
        hook_url: "http://127.0.0.1:1/hook".into(),
        hook_token_var: "ESCOUADE_HOOK_TOKEN".into(),
        fence: vec!["C:/projets/prive".into()],
        read_only: true,
        escouade: Some(proto::Grant {
            url: "http://127.0.0.1:1/mcp".into(),
            token: "jeton".into(),
        }),
        ..proto::TerminalParams::default()
    };
    let l = engine.terminal_launch(params).await.unwrap();
    assert_eq!(
        l.args[..4],
        ["minimal-harness", "--tui", "--session", "min-1"]
    );
    assert!(l.read_only && l.fence_applied);
    assert_eq!(l.trust, ["Minimal lit ce dossier."]);

    let reply = HookReply {
        context: Some("Le projet suit docs/SPEC.md".into()),
        deny: None,
    };
    let a = engine
        .terminal_hook(
            "a1",
            json!({ "event": "PreToolUse", "tool": "Grep" }),
            Some(reply),
        )
        .await
        .unwrap();
    assert_eq!(
        a.answer,
        Some(json!({ "additionalContext": "Le projet suit docs/SPEC.md" }))
    );
    let deny = HookReply {
        context: None,
        deny: Some("hors du projet".into()),
    };
    let a = engine
        .terminal_hook("a1", json!({ "event": "PreToolUse" }), Some(deny))
        .await
        .unwrap();
    assert_eq!(
        a.answer,
        Some(json!({ "decision": "block", "reason": "hors du projet" }))
    );

    // A Stop ends the turn of the agent, through the same items as its frames.
    let a = engine
        .terminal_hook("a1", json!({ "event": "Stop" }), None)
        .await
        .unwrap();
    let mut agent = AgentRt::new(
        meta(&bench_dir("int9-terminal-conv")),
        &bench_dir("int9-terminal-conv").join("conv"),
    );
    let mut fx = Effects::default();
    let items: Vec<Value> = a
        .items
        .iter()
        .map(|i| serde_json::to_value(i).unwrap())
        .collect();
    agent.apply_items(&items, &mut fx);
    assert!(matches!(fx.turn_end, Some(TurnEnd::Finished(_))));
    rt.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn what_the_harness_remembers_and_what_it_can_do_are_declared_without_starting_the_plugin() {
    let (rt, engine) = minimal("declared", 3);
    let dir = engine
        .memory_dir(Path::new("/comptes/a"), "C:\\projets\\démo")
        .unwrap();
    assert_eq!(
        dir,
        PathBuf::from("/comptes/a/projects/C--projets-d-mo/memory")
    );
    let caps = engine.caps();
    assert!(caps.read_only, "a discussion may run on it");
    assert!(!caps.accounts_by_key);
    assert_eq!(engine.models().len(), 2);
    assert_eq!(engine.name(), "Minimal");
    assert_eq!(rt.status(ID).phase, PhaseName::Idle, "nothing started");
    rt.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_harness_of_a_plugin_is_registered_from_its_manifest() {
    let data = DataDir::new(bench_dir("int9-register").join("data"));
    let plugins = Plugins::load_in(data.clone(), VERSION, app_dir());
    let rt = Runtime::new(
        plugins.clone(),
        Arc::new(MemoryStore::default()),
        &data,
        VERSION,
        limits(3),
    );
    let m = plugins
        .get(ID, crate::i18n::Lang::En)
        .and_then(|v| v.manifest)
        .unwrap();
    let ids = harness::register(&rt, ID, &m.contributes, &app_dir());
    assert_eq!(ids, [crate::model::EngineId::new("minimal")]);
    let found = crate::engine::registry::find(ids[0]).expect("in the registry");
    assert_eq!(found.name(), "Minimal");
    assert!(crate::engine::written(ids[0]));
    rt.shutdown();
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    let i = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[i]
}

/// The stream of a harness through its plugin, measured against INT-0 (`docs/PLUGINS.md` §4.6, point 1): the delay
/// from the line its harness wrote to the items received by the core, at once (`flux`) and at a pace (`rythme`), the
/// lines per second, and the plugin's memory. Run by hand: `cargo nextest run --run-ignored only
/// int9_measure --no-capture`.
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn int9_measure_the_stream_through_the_plugin() {
    let (rt, engine) = minimal("measure", 3);
    let dir = bench_dir("int9-measure-run");
    for (prompt, n) in [("flux 50000", 50_000usize), ("rythme 2000", 2_000)] {
        let mut a = Agent::spawn_with(engine, &dir, |_| {});
        a.proc().initialize().await.unwrap();
        let started = Instant::now();
        a.send(prompt);
        let mut delays = Vec::with_capacity(n);
        let mut last = Instant::now();
        while delays.len() < n {
            match tokio::time::timeout(Duration::from_secs(60), a.heard.recv()).await {
                Ok(Some(Heard::Frame(frame))) => {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_micros() as f64;
                    for item in frame["items"].as_array().into_iter().flatten() {
                        if let Some(t) = item["text"].as_str().and_then(|t| t.parse::<f64>().ok()) {
                            delays.push((now - t) / 1000.0);
                        }
                    }
                    last = Instant::now();
                }
                other => panic!("{}", matches!(other, Ok(Some(Heard::Exit(_))))),
            }
        }
        let secs = last.duration_since(started).as_secs_f64();
        let memory = rt.usage(ID).map(|u| u.memory >> 20).unwrap_or_default();
        delays.sort_by(|a, b| a.partial_cmp(b).unwrap());
        println!(
            "INT-9 {prompt}: {n} lines in {secs:.2} s ({:.0} lines/s), delay median {:.2} ms, p99 {:.2} ms, max {:.2} ms; plugin {memory} MB",
            n as f64 / secs,
            percentile(&delays, 0.5),
            percentile(&delays, 0.99),
            delays[delays.len() - 1],
        );
        a.stop().await;
    }
    rt.shutdown();
}

/// INT-10.8: the fence of a launch in a file of JSON, its paths written with their `\` doubled, is read as said; a relaunch goes
/// through the same check: one that drops a path of the fence, or read-only, is refused.
#[test]
fn a_fence_in_a_file_of_json_is_read_as_said_and_a_file_outside_the_agents_folder_refused() {
    let (private, home) = if cfg!(windows) {
        ("C:\\prive\\", "C:\\agent")
    } else {
        ("/prive/", "/agent")
    };
    let fence = vec![private.to_string()];
    let file = |path: String, text: String| proto::FileToWrite { path, text };
    let inside = format!("{home}{}settings.json", std::path::MAIN_SEPARATOR);
    let rules = serde_json::json!({ "deny": [format!("read_file({})", private.trim_end_matches(['/', '\\']))] });
    let ok = proto::Launch {
        fence_applied: true,
        files: vec![file(inside.clone(), rules.to_string())],
        ..proto::Launch::default()
    };
    check_launch("Agy", true, None, &fence, false, Some(home), &ok).unwrap();
    let forgot = proto::Launch {
        fence_applied: true,
        files: vec![file(inside.clone(), "{}".into())],
        ..proto::Launch::default()
    };
    assert!(check_launch("Agy", true, None, &fence, false, Some(home), &forgot).is_err());
    assert!(check_launch("Agy", true, None, &[], true, Some(home), &ok).is_err());
    let outside = proto::Launch {
        files: vec![file(
            format!("{home}-autre{}x", std::path::MAIN_SEPARATOR),
            "{}".into(),
        )],
        ..proto::Launch::default()
    };
    assert!(check_launch("Agy", true, None, &[], false, Some(home), &outside).is_err());
}

#[test]
fn an_agent_on_the_default_account_has_the_folder_its_harness_looks_in() {
    let home = std::path::Path::new("/home/moi");
    for (var, folder) in [
        ("CODEX_HOME", ".codex"),
        ("COPILOT_HOME", ".copilot"),
        ("CURSOR_CONFIG_DIR", ".cursor"),
        ("GEMINI_CLI_HOME", ".gemini"),
    ] {
        assert_eq!(
            crate::plugins::harness::default_home_in(var, home),
            Some(home.join(folder))
        );
    }
    assert_eq!(
        crate::plugins::harness::default_home_in("AUTRE", home),
        None
    );
}
