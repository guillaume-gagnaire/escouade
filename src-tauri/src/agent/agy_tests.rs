//! Antigravity CLI's translation, on the streams of the real `agy` 1.4.9 (tests/fixtures/agy/) and on
//! its fake (tests/fixtures/fake-agy.mjs): the turn, its permissions, its question, its plan, its
//! interruption, its resume, the fence (ESC-270).

use super::*;
use crate::engine::agy::{bench, AGY};
use crate::engine::{Engine, Spawn, Start};
use crate::model::{AgentMeta, EngineId};
use std::time::Duration;
use tokio::sync::mpsc;

fn rt(cwd: &str) -> AgentRt {
    let dir = crate::paths::test_dir(&format!("esc270-agy-{}", new_id()));
    AgentRt::new(
        AgentMeta {
            id: new_id(),
            engine: EngineId::AGY,
            cwd: cwd.into(),
            mode: "default".into(),
            ..Default::default()
        },
        &dir,
    )
}

/// The stream `file` of tests/fixtures/agy/ as it was recorded, through the translation.
fn replayed(file: &str) -> AgentRt {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/fixtures/agy")
        .join(file);
    let mut rt = rt("C:\\tmp\\esc269\\work");
    for line in std::fs::read_to_string(path).unwrap().lines() {
        let f: Value = serde_json::from_str(line).unwrap();
        rt.agy_frame(&f, &mut Effects::default());
    }
    rt
}

#[test]
fn a_recorded_turn_is_its_text_its_tokens_and_no_cost() {
    let mut rt = replayed("01-simple-turn.jsonl");
    let items = rt.conv.items();
    let text = items.iter().find(|i| i["kind"] == "text").unwrap();
    assert_eq!(text["text"], "pong");
    let turn = items.last().unwrap();
    assert_eq!(turn["kind"], "turn");
    assert_eq!(turn["isError"], false);
    assert_eq!(turn["tokens"], 6735);
    assert_eq!(turn["cost"], 0.0);
    assert_eq!(
        rt.meta.session_id.as_deref(),
        Some("9f9031e6-987b-430c-a020-a22779883a82")
    );
}

#[test]
fn a_recorded_refusal_fails_its_call_and_tools_take_claude_codes_names() {
    let mut rt = replayed("03-permission-denied-print.jsonl");
    let items = rt.conv.items();
    let bash = items.iter().find(|i| i["kind"] == "tool").unwrap();
    assert_eq!(bash["name"], "Bash");
    assert_eq!(bash["input"]["command"], "echo hi");
    assert_eq!(bash["status"], "error");
    let mut rt = replayed("08-mcp-allowed-project-config.jsonl");
    let items = rt.conv.items();
    let names: Vec<&str> = items
        .iter()
        .filter(|i| i["kind"] == "tool")
        .filter_map(|i| i["name"].as_str())
        .collect();
    assert_eq!(names, ["Read", "mcp__proj__esc_ping"], "{items:#?}");
    let mut rt = replayed("12-subagent-tasks.jsonl");
    assert!(rt
        .conv
        .items()
        .iter()
        .any(|i| i["kind"] == "tool" && i["name"] == "Task"));
    let plan = rt.meta.plan.clone().expect("its plan");
    assert!(!plan.agents.is_empty());
}

/// What the process says, as the core passes it on.
enum Heard {
    Frame(Value),
    Exit,
}

/// An agent of Antigravity on the fake, in `dir`, its frames translated as the core does.
struct Live {
    rt: AgentRt,
    heard: mpsc::UnboundedReceiver<Heard>,
    log: std::path::PathBuf,
    home: std::path::PathBuf,
}

impl Live {
    async fn start(dir: &std::path::Path, meta: AgentMeta, fence: crate::fence::Fence) -> Live {
        let program = AGY.locate(&bench::fake_agy(), "").unwrap();
        let escouade = crate::engine::Escouade::Granted {
            url: "http://127.0.0.1:9/mcp".into(),
            token: "s3cr3t-jeton".into(),
            file: None,
        };
        Live::start_with(dir, meta, fence, program, escouade).await
    }

    async fn start_with(
        dir: &std::path::Path,
        meta: AgentMeta,
        fence: crate::fence::Fence,
        program: crate::engine::Program,
        escouade: crate::engine::Escouade,
    ) -> Live {
        let home = dir.join("agy-home");
        let log = dir.join("fake-agy.jsonl");
        let env = vec![
            (
                crate::engine::agy::HOME_VAR.into(),
                home.to_string_lossy().into_owned(),
            ),
            ("FAKE_AGY_LOG".into(), log.to_string_lossy().into_owned()),
        ];
        let mut spawn = Spawn::of(&meta, program, env);
        spawn.fence = fence;
        spawn.escouade = escouade;
        let (tx, heard) = mpsc::unbounded_channel();
        let exit = tx.clone();
        let proc = AGY
            .spawn(
                spawn,
                Box::new(move |f| {
                    let _ = tx.send(Heard::Frame(f));
                }),
                Box::new(move |_, _| {
                    let _ = exit.send(Heard::Exit);
                }),
            )
            .unwrap();
        let mut rt = AgentRt::new(meta, &dir.join(format!("conv-{}", new_id())));
        rt.attach(proc.clone());
        proc.initialize().await.unwrap();
        let mut live = Live {
            rt,
            heard,
            log,
            home,
        };
        live.drain().await;
        live
    }

    async fn drain(&mut self) {
        while let Ok(Some(h)) =
            tokio::time::timeout(Duration::from_millis(50), self.heard.recv()).await
        {
            if let Heard::Frame(f) = h {
                AGY.translate(&mut self.rt, &f, &mut Effects::default());
            }
        }
    }

    fn send(&mut self, text: &str) {
        let m = AGY.message(text, &[]).unwrap();
        let uid = uuid::Uuid::new_v4().to_string();
        self.rt
            .proc
            .clone()
            .unwrap()
            .send_message(&uid, &m)
            .unwrap();
        self.rt
            .push_user(&uid, text, 0, &[], &mut Effects::default());
    }

    /// Until `pred` holds of the conversation, frames translated meanwhile.
    async fn until(&mut self, what: &str, pred: impl Fn(&[Value]) -> bool) {
        let started = std::time::Instant::now();
        while !pred(&self.rt.conv.items()) {
            assert!(
                started.elapsed() < Duration::from_secs(30),
                "no {what}: {:#?}
{}
{}",
                self.rt.conv.items(),
                self.launched(),
                std::fs::read_to_string(self.home.join(".gemini/antigravity-cli/settings.json"))
                    .unwrap_or_default()
            );
            match tokio::time::timeout(Duration::from_millis(200), self.heard.recv()).await {
                Ok(Some(Heard::Frame(f))) => {
                    AGY.translate(&mut self.rt, &f, &mut Effects::default())
                }
                Ok(Some(Heard::Exit)) | Ok(None) => panic!("the fake left: no {what}"),
                Err(_) => {}
            }
        }
    }

    async fn turns_done(&mut self, n: usize) {
        self.until("the end of the turn", |items| {
            items.iter().filter(|i| i["kind"] == "turn").count() >= n
        })
        .await;
    }

    fn texts(&mut self) -> Vec<String> {
        self.rt
            .conv
            .items()
            .into_iter()
            .filter(|i| i["kind"] == "text")
            .filter_map(|i| i["text"].as_str().map(str::to_string))
            .collect()
    }

    fn launched(&self) -> String {
        std::fs::read_to_string(&self.log).unwrap_or_default()
    }

    /// The arguments of each launch of its program, in order.
    fn launches(&self) -> Vec<Vec<String>> {
        self.launched()
            .lines()
            .filter_map(|l| serde_json::from_str::<Value>(l).ok())
            .filter_map(|v| serde_json::from_value(v["launch"]["argv"].clone()).ok())
            .collect()
    }

    fn answer(&mut self, kind: &str, decision: &str) {
        let id = self
            .rt
            .conv
            .items()
            .into_iter()
            .rev()
            .find(|i| i["kind"] == kind && i["decision"].is_null())
            .and_then(|i| i["id"].as_str().map(str::to_string))
            .expect("a card");
        self.rt
            .answer_permission_by(&id, decision, None, None, &mut Effects::default())
            .unwrap();
    }

    async fn stop(&mut self) {
        let proc = self.rt.proc.clone().unwrap();
        proc.kill();
        proc.wait_gone(Duration::from_secs(5)).await;
    }
}

fn meta(dir: &std::path::Path, mode: &str) -> AgentMeta {
    let cwd = dir.join("repo");
    std::fs::create_dir_all(&cwd).unwrap();
    AgentMeta {
        id: new_id(),
        engine: EngineId::AGY,
        cwd: cwd.to_string_lossy().into_owned(),
        model: "gemini-3.8-flash".into(),
        effort: "high".into(),
        mode: mode.into(),
        ..Default::default()
    }
}

#[tokio::test]
async fn a_turn_on_the_fake_has_its_model_effort_mode_home_and_server_without_its_token_in_the_arguments(
) {
    let dir = crate::paths::test_dir("esc270-agy-live-turn");
    let mut live = Live::start(&dir, meta(&dir, "acceptEdits"), Default::default()).await;
    live.send("modèle");
    live.turns_done(1).await;
    assert_eq!(
        live.texts(),
        ["Modèle : gemini-3.8-flash, effort : high, mode : accept-edits"]
    );
    let log = live.launched();
    // Escouade's server in its own MCP config, its token there; never on the command line.
    assert!(!log
        .lines()
        .any(|l| l.contains("\"launch\"") && l.contains("s3cr3t")));
    let mcp = std::fs::read_to_string(live.home.join(".gemini/config/mcp_config.json")).unwrap();
    assert!(mcp.contains("Bearer s3cr3t-jeton"), "{mcp}");
    // Its own home, never the user's; no key of the machine.
    let home = live.home.to_string_lossy().replace('\\', "\\\\");
    assert!(log.contains(&format!("\"home\":\"{home}\"")), "{log}");
    assert!(log.contains("\"geminiKey\":null"), "{log}");
    // The instructions told once, before its first message.
    assert!(log.contains("ask_question"), "{log}");
    // Its tokens, no cost.
    let turn = live
        .rt
        .conv
        .items()
        .into_iter()
        .find(|i| i["kind"] == "turn")
        .unwrap();
    assert_eq!(turn["tokens"], 7025);
    // A model changed: its program starts again on the conversation before the next message.
    let proc = live.rt.proc.clone().unwrap();
    proc.set_model("claude-sonnet-4-6").await.unwrap();
    live.send("modèle");
    live.turns_done(2).await;
    assert_eq!(
        live.texts()[1],
        "Modèle : claude-sonnet-4-6, effort : aucun, mode : accept-edits"
    );
    let launches = live.launches();
    assert_eq!(launches.len(), 2);
    assert!(launches[1].contains(&"--conversation".to_string()));
    // Its server is taken out of its config once it stops: its token with it.
    live.stop().await;
    let mcp = std::fs::read_to_string(live.home.join(".gemini/config/mcp_config.json")).unwrap();
    assert!(!mcp.contains("s3cr3t"), "{mcp}");
}

#[tokio::test]
async fn a_command_refused_asks_once_the_turn_is_over_then_runs_and_always_holds() {
    let dir = crate::paths::test_dir("esc270-agy-live-perm");
    let mut live = Live::start(&dir, meta(&dir, "default"), Default::default()).await;
    live.send("commande");
    live.until("its permission", |items| {
        items.iter().any(|i| i["kind"] == "permission")
    })
    .await;
    let items = live.rt.conv.items();
    let perm = items.iter().find(|i| i["kind"] == "permission").unwrap();
    assert_eq!(perm["toolName"], "Bash");
    assert_eq!(perm["input"]["command"], "npm test");
    assert_eq!(live.rt.meta.status, AgentStatus::Waiting);
    // From its card, as from the list.
    live.answer("permission", "always");
    live.turns_done(1).await;
    let items = live.rt.conv.items();
    assert!(items
        .iter()
        .any(|i| i["kind"] == "tool" && i["name"] == "Bash" && i["status"] == "ok"));
    assert_eq!(live.texts().last().unwrap(), "Fait.");
    // One turn of Escouade for both of `agy`'s.
    assert_eq!(items.iter().filter(|i| i["kind"] == "turn").count(), 1);
    let rules =
        std::fs::read_to_string(live.home.join(".gemini/antigravity-cli/settings.json")).unwrap();
    assert!(rules.contains("command(npm test)"), "{rules}");
    // « Toujours »: no card the next time.
    live.send("commande");
    live.turns_done(2).await;
    let items = live.rt.conv.items();
    assert_eq!(
        items.iter().filter(|i| i["kind"] == "permission").count(),
        1
    );
    assert_eq!(live.texts().last().unwrap(), "Tests passés.");
    live.stop().await;
}

#[tokio::test]
async fn a_refusal_ends_the_turn_and_once_is_taken_back_after() {
    let dir = crate::paths::test_dir("esc270-agy-live-deny");
    let mut live = Live::start(&dir, meta(&dir, "default"), Default::default()).await;
    live.send("commande");
    live.until("its permission", |items| {
        items.iter().any(|i| i["kind"] == "permission")
    })
    .await;
    live.answer("permission", "deny");
    live.turns_done(1).await;
    assert!(!live
        .rt
        .conv
        .items()
        .iter()
        .any(|i| i["kind"] == "tool" && i["status"] == "ok"));
    // Allowed once: the rule goes once the turn is over.
    live.send("commande");
    live.until("its permission", |items| {
        items.iter().filter(|i| i["kind"] == "permission").count() == 2
    })
    .await;
    live.answer("permission", "allow");
    live.turns_done(2).await;
    let rules =
        std::fs::read_to_string(live.home.join(".gemini/antigravity-cli/settings.json")).unwrap();
    assert!(!rules.contains("command(npm test)"), "{rules}");
    live.stop().await;
}

#[tokio::test]
async fn a_write_in_the_folder_goes_through_with_its_diff_and_one_outside_asks() {
    let dir = crate::paths::test_dir("esc270-agy-live-write");
    let mut live = Live::start(&dir, meta(&dir, "default"), Default::default()).await;
    live.send("fichier");
    live.turns_done(1).await;
    let items = live.rt.conv.items();
    let w = items.iter().find(|i| i["kind"] == "tool").unwrap();
    assert_eq!(w["name"], "Write", "{w}");
    assert_eq!(w["status"], "ok");
    assert!(w["result"]["patch"].to_string().contains("+bonjour"), "{w}");
    assert!(!live.rt.meta.touched_files.is_empty());
    live.send("fichier dehors");
    live.until("its permission", |items| {
        items.iter().any(|i| i["kind"] == "permission")
    })
    .await;
    let items = live.rt.conv.items();
    let perm = items.iter().find(|i| i["kind"] == "permission").unwrap();
    assert_eq!(perm["toolName"], "Write");
    assert!(perm["patch"].to_string().contains("bonjour"), "{perm}");
    assert!(
        perm["description"]
            .as_str()
            .unwrap_or_default()
            .contains("dossier")
            || perm["description"]
                .as_str()
                .unwrap_or_default()
                .contains("folder")
    );
    live.answer("permission", "allow");
    live.turns_done(2).await;
    assert!(dir.join("dehors.txt").is_file());
    live.stop().await;
}

#[tokio::test]
async fn a_question_skipped_by_agy_is_asked_then_answered() {
    let dir = crate::paths::test_dir("esc270-agy-live-question");
    let mut live = Live::start(&dir, meta(&dir, "default"), Default::default()).await;
    live.send("question");
    live.until("its question", |items| {
        items.iter().any(|i| i["kind"] == "question")
    })
    .await;
    let q = live
        .rt
        .conv
        .items()
        .into_iter()
        .find(|i| i["kind"] == "question")
        .unwrap();
    assert_eq!(q["questions"][0]["question"], "Bleu ou rouge ?");
    assert_eq!(q["questions"][0]["options"][0]["label"], "Bleu");
    let id = q["id"].as_str().unwrap().to_string();
    live.rt
        .answer_question_by(
            &id,
            json!({ "Bleu ou rouge ?": "Rouge" }),
            None,
            &mut Effects::default(),
        )
        .unwrap();
    live.turns_done(1).await;
    assert_eq!(live.texts().last().unwrap(), "Réponse reçue : Rouge");
    live.stop().await;
}

#[tokio::test]
async fn in_the_mode_plan_its_plan_is_approved_then_carried_out() {
    let dir = crate::paths::test_dir("esc270-agy-live-plan");
    let mut live = Live::start(&dir, meta(&dir, "plan"), Default::default()).await;
    live.send("propose un plan");
    live.until("its plan", |items| {
        items
            .iter()
            .any(|i| i["kind"] == "permission" && i["toolName"] == "ExitPlanMode")
    })
    .await;
    let card = live
        .rt
        .conv
        .items()
        .into_iter()
        .find(|i| i["toolName"] == "ExitPlanMode")
        .unwrap();
    assert!(card["input"]["plan"]
        .as_str()
        .unwrap()
        .contains("Écrire notes.txt"));
    live.answer("permission", "allow");
    live.turns_done(1).await;
    assert_eq!(live.texts().last().unwrap(), "Plan mis en œuvre.");
    assert_eq!(live.rt.meta.mode, "default");
    let launches = live.launches();
    assert!(launches[0].contains(&"plan".to_string()));
    assert!(!launches.last().unwrap().contains(&"plan".to_string()));
    live.stop().await;
}

#[tokio::test]
async fn a_turn_stopped_kills_its_program_and_the_next_resumes_the_conversation() {
    let dir = crate::paths::test_dir("esc270-agy-live-stop");
    let mut live = Live::start(&dir, meta(&dir, "default"), Default::default()).await;
    live.send("lent");
    live.until("its first words", |items| {
        items.iter().any(|i| i["kind"] == "text")
    })
    .await;
    live.rt.interrupted = true;
    let proc = live.rt.proc.clone().unwrap();
    proc.interrupt(false, Duration::from_secs(5)).await.unwrap();
    live.turns_done(1).await;
    let turn = live
        .rt
        .conv
        .items()
        .into_iter()
        .find(|i| i["kind"] == "turn")
        .unwrap();
    assert_eq!(turn["interrupted"], true);
    assert!(proc.is_alive());
    live.send("bonjour");
    live.turns_done(2).await;
    let session = live.rt.meta.session_id.clone().unwrap();
    let launches = live.launches();
    assert_eq!(launches.len(), 2);
    assert!(launches[1]
        .join(" ")
        .contains(&format!("--conversation {session}")));
    live.stop().await;
}

#[tokio::test]
async fn a_conversation_is_resumed_by_a_new_process_and_one_gone_starts_anew() {
    let dir = crate::paths::test_dir("esc270-agy-live-resume");
    let m = meta(&dir, "default");
    let mut live = Live::start(&dir, m.clone(), Default::default()).await;
    live.send("pamplemousse");
    live.turns_done(1).await;
    let session = live.rt.meta.session_id.clone().unwrap();
    live.stop().await;
    let mut again = m.clone();
    again.session_id = Some(session.clone());
    assert_eq!(
        Start::of(&again),
        Start::Resume {
            session: session.clone()
        }
    );
    let mut live = Live::start(&dir, again.clone(), Default::default()).await;
    live.send("tu te souviens ?");
    live.turns_done(1).await;
    assert_eq!(live.rt.meta.session_id.as_deref(), Some(session.as_str()));
    assert_eq!(live.texts().last().unwrap(), "Tu as dit : pamplemousse");
    live.stop().await;
    // A conversation gone: a new one, said.
    let mut gone = m;
    gone.session_id = Some("00000000-0000-0000-0000-000000000000".into());
    let mut live = Live::start(&dir.join("other"), gone, Default::default()).await;
    assert_ne!(
        live.rt.meta.session_id.as_deref(),
        Some("00000000-0000-0000-0000-000000000000")
    );
    assert!(live.rt.conv.items().iter().any(|i| i["kind"] == "notice"
        && i["text"]
            .as_str()
            .unwrap_or_default()
            .contains("Antigravity")));
    live.stop().await;
}

#[tokio::test]
async fn a_fenced_path_is_refused_by_its_rules_even_in_bypass_without_a_card() {
    let dir = crate::paths::test_dir("esc270-agy-live-fence");
    let m = meta(&dir, "bypassPermissions");
    let outside = std::path::Path::new(&m.cwd)
        .parent()
        .unwrap()
        .join("dehors.txt");
    let fence = crate::fence::Fence::new(vec![outside.clone()]);
    let mut live = Live::start(&dir, m, fence).await;
    live.send("fichier dehors");
    live.turns_done(1).await;
    let items = live.rt.conv.items();
    assert!(
        !items.iter().any(|i| i["kind"] == "permission"),
        "{items:#?}"
    );
    let w = items.iter().find(|i| i["kind"] == "tool").unwrap();
    assert_eq!(w["status"], "error");
    assert!(!outside.exists());
    let rules =
        std::fs::read_to_string(live.home.join(".gemini/antigravity-cli/settings.json")).unwrap();
    assert!(
        rules.contains("\"deny\"") && rules.contains("dehors.txt"),
        "{rules}"
    );
    // Its commands are allowed in Bypass by its rules.
    live.send("commande");
    live.turns_done(2).await;
    assert_eq!(live.texts().last().unwrap(), "Tests passés.");
    live.stop().await;
}

/// The size and date of each file of `dir` but those of `skip` (the files `agy` writes of its own
/// running session): never what they hold (its sign-in files are never read).
fn stamps(
    dir: &std::path::Path,
    skip: &str,
) -> std::collections::BTreeMap<String, (u64, Option<std::time::SystemTime>)> {
    let mut out = std::collections::BTreeMap::new();
    let mut todo = vec![dir.to_path_buf()];
    while let Some(d) = todo.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            let rel = p.strip_prefix(dir).unwrap().to_string_lossy().into_owned();
            if rel.starts_with(skip) {
                continue;
            }
            if p.is_dir() {
                todo.push(p);
            } else if let Ok(m) = e.metadata() {
                out.insert(rel, (m.len(), m.modified().ok()));
            }
        }
    }
    out
}

/// ESC-270, by hand: a short real run of the machine's Antigravity CLI (`ESCOUADE_TEST_REAL_AGY`, its
/// `agy`), on its Google account (in the keychain, never read), in a home of its own: a reply, a
/// command whose permission is allowed from its card once the turn is over, then, in Bypass, a file
/// of a private project its rules refuse. The user's `~/.gemini` but `antigravity-cli` (where a
/// session of the user's may be writing its logs) is the same, by size and date, before and after.
/// About 4 turns of Gemini Flash (Low), ~30 000 input tokens of its weekly quota.
#[tokio::test]
#[ignore = "real Antigravity CLI: ESCOUADE_TEST_REAL_AGY=<agy>"]
async fn real_agy() {
    let program = std::env::var("ESCOUADE_TEST_REAL_AGY").expect("ESCOUADE_TEST_REAL_AGY");
    let program = AGY.locate(&program, "").expect("its agy");
    let user = dirs::home_dir().unwrap().join(".gemini");
    let before = stamps(&user, "antigravity-cli");
    let dir = crate::paths::test_dir("esc270-real-agy");
    let m = AgentMeta {
        model: "gemini-3.8-flash".into(),
        effort: "low".into(),
        ..meta(&dir, "default")
    };
    let secret = dir.join("prive");
    std::fs::create_dir_all(&secret).unwrap();
    std::fs::write(secret.join("secret.txt"), "MOT-DE-PASSE-7731\n").unwrap();
    let fence = crate::fence::Fence::new(vec![secret.clone()]);
    let mut live = Live::start_with(
        &dir,
        m,
        fence,
        program,
        crate::engine::Escouade::Denied { file: None },
    )
    .await;
    // A command: refused by `agy`, asked, allowed once, run.
    live.send(
        "Run the command `echo escouade-ok` with run_command, then reply with the single word: done.",
    );
    live.until("its permission", |items| {
        items
            .iter()
            .any(|i| i["kind"] == "permission" || i["kind"] == "turn")
    })
    .await;
    let items = live.rt.conv.items();
    let perm = items
        .iter()
        .find(|i| i["kind"] == "permission")
        .unwrap_or_else(|| panic!("a permission for echo: {items:#?}"));
    assert_eq!(perm["toolName"], "Bash", "{perm}");
    live.answer("permission", "allow");
    live.turns_done(1).await;
    let items = live.rt.conv.items();
    assert!(
        items
            .iter()
            .any(|i| i["kind"] == "tool" && i["name"] == "Bash" && i["status"] == "ok"),
        "{items:#?}"
    );
    let turn = items.iter().find(|i| i["kind"] == "turn").unwrap();
    assert_eq!(turn["isError"], false, "{items:#?}");
    assert!(turn["tokens"].as_u64().unwrap_or(0) > 0);
    // In Bypass, a file of a private project: refused by its rules, never read.
    let proc = live.rt.proc.clone().unwrap();
    proc.set_mode("bypassPermissions").await.unwrap();
    live.rt.meta.mode = "bypassPermissions".into();
    let file = secret.join("secret.txt");
    live.send(&format!(
        "Read the file {} with read_file and reply with its content only. Use no other tool.",
        file.display()
    ));
    live.turns_done(2).await;
    let items = live.rt.conv.items();
    let said: String = items
        .iter()
        .filter(|i| i["kind"] == "text" || i["kind"] == "tool")
        .map(|i| i.to_string())
        .collect();
    assert!(!said.contains("MOT-DE-PASSE-7731"), "{items:#?}");
    live.stop().await;
    assert!(
        before == stamps(&user, "antigravity-cli"),
        "the user's ~/.gemini changed"
    );
}
