//! Gemini CLI's translation, on the streams of the study of Gemini CLI 0.66.0
//! (tests/fixtures/gemini/) and on its fake (tests/fixtures/fake-gemini.mjs): the turn, its
//! permissions, its question, its plan, its modes, its cancel, its resume, the fence (ESC-274).

use super::*;
use crate::engine::gemini::{bench, GEMINI};
use crate::engine::{Engine, Spawn, Start};
use crate::model::{AgentMeta, EngineId};
use std::time::Duration;
use tokio::sync::mpsc;

fn rt(cwd: &str) -> AgentRt {
    let dir = crate::paths::test_dir(&format!("esc274-gemini-{}", new_id()));
    AgentRt::new(
        AgentMeta {
            id: new_id(),
            engine: EngineId::GEMINI,
            cwd: cwd.into(),
            mode: "default".into(),
            ..Default::default()
        },
        &dir,
    )
}

/// The stream `file` as it was recorded, through the translation.
fn replayed(file: &str, cwd: &str) -> AgentRt {
    let mut rt = rt(cwd);
    for line in bench::stream(file) {
        rt.gemini_frame(&line, &mut Effects::default());
    }
    rt
}

const ESSAI: &str = "C:\\tmp\\esc273\\proj2";

#[test]
fn a_recorded_write_asks_with_its_diff_and_ends_with_its_tokens() {
    let mut rt = replayed("03-permission-write.jsonl", ESSAI);
    let items = rt.conv.items();
    let perm = items.iter().find(|i| i["kind"] == "permission").unwrap();
    assert_eq!(perm["toolName"], "Write", "{perm}");
    assert!(perm["patch"].to_string().contains("hello"), "{perm}");
    let w = items.iter().find(|i| i["kind"] == "tool").unwrap();
    assert_eq!(w["status"], "ok", "{items:#?}");
    let turn = items.last().unwrap();
    assert_eq!(turn["kind"], "turn");
    assert_eq!(turn["isError"], false);
    assert!(turn["tokens"].as_u64().unwrap() > 0, "{turn}");
    assert_eq!(rt.meta.cost, 0.0);
}

#[test]
fn a_recorded_command_is_bash_and_a_rejected_write_fails() {
    let mut rt = replayed("05-permission-shell.jsonl", ESSAI);
    let items = rt.conv.items();
    let perm = items.iter().find(|i| i["kind"] == "permission").unwrap();
    assert_eq!(perm["toolName"], "Bash");
    assert_eq!(perm["input"]["command"], "node --version");
    let mut rt = replayed("04-permission-reject.jsonl", ESSAI);
    let items = rt.conv.items();
    let perm = items.iter().find(|i| i["kind"] == "permission").unwrap();
    assert_eq!(perm["decision"], "deny");
    let w = items.iter().find(|i| i["kind"] == "tool").unwrap();
    assert_eq!(w["status"], "error", "{w}");
}

#[test]
fn a_recorded_mcp_tool_takes_claude_codes_name() {
    let mut rt = replayed("12-mcp-session-new.jsonl", ESSAI);
    let items = rt.conv.items();
    let tool = items.iter().find(|i| i["kind"] == "tool").unwrap();
    assert_eq!(tool["name"], "mcp__escouade__get_board", "{tool}");
    let perm = items.iter().find(|i| i["kind"] == "permission").unwrap();
    assert!(perm["description"]
        .as_str()
        .unwrap_or_default()
        .contains("escouade"));
}

#[test]
fn the_recorded_plan_is_a_card_and_its_mode_changes_are_left_out_of_the_thread() {
    let mut rt = replayed("09-plan-exit-approval.jsonl", ESSAI);
    let items = rt.conv.items();
    let card = items
        .iter()
        .find(|i| i["toolName"] == "ExitPlanMode")
        .unwrap();
    assert!(card["input"]["plan"].as_str().unwrap().contains("plan.md"));
    assert_eq!(card["decision"], "allow");
    // Neither « [MODE_UPDATE] » nor the call of exit_plan_mode in the thread.
    assert!(
        !items
            .iter()
            .any(|i| i.to_string().contains("[MODE_UPDATE]")),
        "{items:#?}"
    );
    assert!(!items
        .iter()
        .any(|i| i["kind"] == "tool" && i["id"].as_str().unwrap_or("").starts_with("exit_plan")));
    assert_eq!(rt.meta.mode, "default");
    let rt = replayed("08-plan-enter.jsonl", ESSAI);
    assert_eq!(rt.meta.mode, "plan");
}

#[test]
fn a_recorded_load_leaves_its_replay_out_and_a_subagent_is_a_task() {
    let mut rt = replayed("11-load-replay.jsonl", ESSAI);
    let items = rt.conv.items();
    assert!(
        !items
            .iter()
            .any(|i| i.to_string().contains("session_context")),
        "{items:#?}"
    );
    assert!(!items.iter().any(|i| i["kind"] == "tool"), "{items:#?}");
    assert_eq!(
        rt.meta.session_id.as_deref(),
        Some("e93b98be-8a38-49e3-b150-412b345be625")
    );
    let mut rt = replayed("18-subagent-yolo.jsonl", ESSAI);
    let items = rt.conv.items();
    assert!(
        items
            .iter()
            .any(|i| i["kind"] == "tool" && i["name"] == "Task"),
        "{items:#?}"
    );
}

#[test]
fn a_recorded_rate_limit_ends_the_turn_limited_and_a_policy_refusal_is_an_error() {
    let mut rt = replayed("13-rate-limit-429.jsonl", ESSAI);
    let turn = rt.conv.items().into_iter().last().unwrap();
    assert_eq!(turn["kind"], "turn");
    assert_eq!(turn["limited"], true, "{turn}");
    let mut rt = replayed("16-policy-fence-deny.jsonl", ESSAI);
    let items = rt.conv.items();
    let read = items.iter().find(|i| i["kind"] == "tool").unwrap();
    assert_eq!(read["name"], "Read");
    assert_eq!(read["status"], "error");
    assert!(read["result"].to_string().contains("denied by policy"));
}

/// What the process says, as the core passes it on.
enum Heard {
    Frame(Value),
    Exit,
}

/// An agent of Gemini on the fake, in `dir`, its frames translated as the core does.
struct Live {
    rt: AgentRt,
    heard: mpsc::UnboundedReceiver<Heard>,
    log: std::path::PathBuf,
    home: std::path::PathBuf,
}

impl Live {
    async fn start(dir: &std::path::Path, meta: AgentMeta, fence: crate::fence::Fence) -> Live {
        let program = GEMINI.locate(&bench::fake_gemini(), "").unwrap();
        let escouade = crate::engine::Escouade::Granted {
            url: "http://127.0.0.1:9/mcp".into(),
            token: "s3cr3t-jeton".into(),
            file: None,
        };
        Live::start_with(dir, meta, fence, program, escouade, Vec::new()).await
    }

    async fn start_with(
        dir: &std::path::Path,
        meta: AgentMeta,
        fence: crate::fence::Fence,
        program: crate::engine::Program,
        escouade: crate::engine::Escouade,
        more: Vec<(String, String)>,
    ) -> Live {
        let home = dir.join("gemini-home");
        let log = dir.join("fake-gemini.jsonl");
        let mut env = vec![
            (
                "GEMINI_CLI_HOME".into(),
                home.to_string_lossy().into_owned(),
            ),
            ("FAKE_GEMINI_LOG".into(), log.to_string_lossy().into_owned()),
        ];
        env.extend(more);
        let mut spawn = Spawn::of(&meta, program, env);
        spawn.fence = fence;
        spawn.escouade = escouade;
        let (tx, heard) = mpsc::unbounded_channel();
        let exit = tx.clone();
        let proc = GEMINI
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
        let mut rt = AgentRt::new(meta, &dir.join("conv"));
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
                GEMINI.translate(&mut self.rt, &f, &mut Effects::default());
            }
        }
    }

    fn send(&mut self, text: &str) {
        let m = GEMINI.message(text, &[]).unwrap();
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
            assert!(started.elapsed() < Duration::from_secs(120), "no {what}");
            match tokio::time::timeout(Duration::from_millis(200), self.heard.recv()).await {
                Ok(Some(Heard::Frame(f))) => {
                    GEMINI.translate(&mut self.rt, &f, &mut Effects::default())
                }
                Ok(Some(Heard::Exit)) | Ok(None) => panic!("the fake left: no {what}"),
                Err(_) => {}
            }
        }
    }

    fn turns(&mut self) -> Vec<Value> {
        self.rt
            .conv
            .items()
            .into_iter()
            .filter(|i| i["kind"] == "turn")
            .collect()
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
        engine: EngineId::GEMINI,
        cwd: cwd.to_string_lossy().into_owned(),
        model: "gemini-3.8-flash".into(),
        mode: mode.into(),
        ..Default::default()
    }
}

#[tokio::test]
async fn a_turn_on_the_fake_has_its_model_mode_home_and_server_without_its_token_in_the_arguments()
{
    let dir = crate::paths::test_dir("esc274-gemini-live-turn");
    let mut live = Live::start(&dir, meta(&dir, "acceptEdits"), Default::default()).await;
    live.send("modèle");
    live.until("its reply", |items| {
        items.iter().any(|i| i["kind"] == "turn")
    })
    .await;
    assert_eq!(
        live.texts(),
        ["Je tourne sur gemini-3.8-flash, mode autoEdit."]
    );
    // Its tokens, no cost.
    assert_eq!(live.turns()[0]["tokens"], 1220);
    let log = live.launched();
    // Escouade's server by the session, its token in a header; never on the command line.
    assert!(log.contains("\"argv\":[\"--acp\"]"), "{log}");
    assert!(log.contains("Bearer s3cr3t-jeton"));
    assert!(!log
        .lines()
        .any(|l| l.contains("\"launch\"") && l.contains("s3cr3t")));
    // Its own home, trusted, its settings there; never the user's.
    assert!(log.contains("\"trust\":true"), "{log}");
    let settings = live.home.join(".gemini").join("settings.json");
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(settings).unwrap()).unwrap();
    assert_eq!(doc["general"]["enableAutoUpdate"], false);
    assert!(doc["security"]["auth"]["selectedType"].is_string());
    // What the first message told it: its questions in a block.
    assert!(log.contains("escouade_question"));
    // Its change of mode is left out of the thread.
    live.send("confiance");
    live.until("its reply", |items| {
        items.iter().filter(|i| i["kind"] == "turn").count() == 2
    })
    .await;
    assert!(live.texts().iter().any(|t| t == "Dossier de confiance."));
    assert!(!live.texts().iter().any(|t| t.contains("MODE_UPDATE")));
    live.stop().await;
}

#[tokio::test]
async fn a_permission_refused_cuts_the_turn_and_always_holds_for_the_session() {
    let dir = crate::paths::test_dir("esc274-gemini-live-perm");
    let mut live = Live::start(&dir, meta(&dir, "default"), Default::default()).await;
    live.send("commande");
    live.until("its permission", |items| {
        items.iter().any(|i| i["kind"] == "permission")
    })
    .await;
    let perm = live
        .rt
        .conv
        .items()
        .into_iter()
        .find(|i| i["kind"] == "permission")
        .unwrap();
    assert_eq!(perm["toolName"], "Bash");
    assert_eq!(perm["input"]["command"], "npm test");
    let id = perm["id"].as_str().unwrap().to_string();
    live.rt
        .answer_permission_by(&id, "deny", None, None, &mut Effects::default())
        .unwrap();
    live.until("the turn cut", |items| {
        items.iter().any(|i| i["kind"] == "turn")
    })
    .await;
    assert_eq!(live.turns()[0]["interrupted"], true);
    assert!(live.launched().contains("session/cancel"));
    live.send("commande");
    live.until("its permission", |items| {
        items.iter().filter(|i| i["kind"] == "permission").count() == 2
    })
    .await;
    let second = live
        .rt
        .conv
        .items()
        .into_iter()
        .filter(|i| i["kind"] == "permission")
        .nth(1)
        .unwrap();
    let id = second["id"].as_str().unwrap().to_string();
    live.rt
        .answer_permission_by(&id, "always", None, None, &mut Effects::default())
        .unwrap();
    live.until("its end", |items| {
        items.iter().filter(|i| i["kind"] == "turn").count() == 2
    })
    .await;
    assert!(live.launched().contains("\"optionId\":\"proceed_always\""));
    live.send("commande");
    live.until("its end", |items| {
        items.iter().filter(|i| i["kind"] == "turn").count() == 3
    })
    .await;
    let asked = live
        .rt
        .conv
        .items()
        .iter()
        .filter(|i| i["kind"] == "permission")
        .count();
    assert_eq!(asked, 2);
    live.stop().await;
}

#[tokio::test]
async fn a_question_block_is_a_card_whose_answers_go_on_in_the_same_turn() {
    let dir = crate::paths::test_dir("esc274-gemini-live-question");
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
    assert_eq!(
        q["questions"][0]["question"],
        "Quelle couleur préfères-tu ?"
    );
    assert_eq!(q["questions"][0]["options"][1]["label"], "bleu");
    // The block is not in the thread; the turn waits.
    assert!(!live.texts().iter().any(|t| t.contains("escouade_question")));
    assert!(live.turns().is_empty());
    assert_eq!(live.rt.meta.status, AgentStatus::Waiting);
    let id = q["id"].as_str().unwrap().to_string();
    live.rt
        .answer_question_by(
            &id,
            json!({ "Quelle couleur préfères-tu ?": "bleu" }),
            None,
            &mut Effects::default(),
        )
        .unwrap();
    live.until("its end", |items| items.iter().any(|i| i["kind"] == "turn"))
        .await;
    let items = live.rt.conv.items();
    assert!(
        live.texts().iter().any(|t| t == "Tu as choisi bleu."),
        "{items:#?}
{}",
        live.launched()
    );
    assert_eq!(live.turns().len(), 1);
    // Both prompts' tokens in the turn.
    assert_eq!(live.turns()[0]["tokens"], 2440);
    live.stop().await;
}

#[tokio::test]
async fn in_its_mode_plan_the_plan_is_read_from_its_file_and_always_carries_it_out_accepting_edits()
{
    let dir = crate::paths::test_dir("esc274-gemini-live-plan");
    let mut live = Live::start(&dir, meta(&dir, "plan"), Default::default()).await;
    assert!(live.launched().contains("\"modeId\":\"plan\""));
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
        .contains("2. Écrire son test."));
    assert_eq!(live.rt.meta.status, AgentStatus::Waiting);
    let id = card["id"].as_str().unwrap().to_string();
    live.rt
        .answer_permission_by(&id, "always", None, None, &mut Effects::default())
        .unwrap();
    live.until("its end", |items| items.iter().any(|i| i["kind"] == "turn"))
        .await;
    live.drain().await;
    assert_eq!(live.rt.meta.mode, "acceptEdits");
    assert!(live.launched().contains("\"modeId\":\"autoEdit\""));
    assert!(live
        .texts()
        .iter()
        .any(|t| t == "Plan approuvé, je le mets en œuvre."));
    live.stop().await;
}

#[tokio::test]
async fn a_session_is_resumed_by_a_new_process_its_replay_left_out_and_a_copy_starts_anew() {
    let dir = crate::paths::test_dir("esc274-gemini-live-resume");
    let m = meta(&dir, "default");
    let mut live = Live::start(&dir, m.clone(), Default::default()).await;
    live.send("bonjour");
    live.until("its end", |items| items.iter().any(|i| i["kind"] == "turn"))
        .await;
    let session = live.rt.meta.session_id.clone().unwrap();
    live.stop().await;
    let resumed = AgentMeta {
        session_id: Some(session.clone()),
        ..m.clone()
    };
    assert_eq!(
        Start::of(&resumed),
        Start::Resume {
            session: session.clone()
        }
    );
    let mut again = Live::start(&dir, resumed, Default::default()).await;
    assert!(again.launched().contains("session/load"));
    assert_eq!(again.rt.meta.session_id.as_deref(), Some(session.as_str()));
    // Its history replayed is not told again.
    let before = again.rt.conv.items();
    assert_eq!(before.len(), 3, "{before:#?}");
    again.send("bonjour encore");
    again
        .until("its end", |items| {
            items.iter().filter(|i| i["kind"] == "turn").count() == 2
        })
        .await;
    assert_eq!(
        again.texts(),
        ["Gemini a reçu : bonjour", "Gemini a reçu : bonjour encore"]
    );
    again.stop().await;
    // No `session/fork`: a copy starts a session of its own.
    let copy = AgentMeta {
        id: new_id(),
        fork_of: Some(session.clone()),
        ..m
    };
    let mut forked = Live::start(&dir, copy, Default::default()).await;
    assert_ne!(forked.rt.meta.session_id.as_deref(), Some(session.as_str()));
    forked.stop().await;
}

#[tokio::test]
async fn an_interrupted_turn_ends_cancelled_and_the_next_one_goes() {
    let dir = crate::paths::test_dir("esc274-gemini-live-cancel");
    let mut live = Live::start(&dir, meta(&dir, "default"), Default::default()).await;
    live.send("lent");
    live.until("its start", |items| {
        items.iter().any(|i| i["kind"] == "text")
    })
    .await;
    live.rt.interrupted = true;
    let proc = live.rt.proc.clone().unwrap();
    proc.interrupt(false, Duration::from_secs(5)).await.unwrap();
    live.until("its end", |items| items.iter().any(|i| i["kind"] == "turn"))
        .await;
    assert_eq!(live.turns()[0]["interrupted"], true);
    live.send("bonjour");
    live.until("the next", |items| {
        items.iter().filter(|i| i["kind"] == "turn").count() == 2
    })
    .await;
    live.stop().await;
}

/// « Projet privé »: a file of the fence refused by its rules, in yolo too, never read.
#[tokio::test]
async fn a_fenced_file_is_refused_by_its_rules_even_in_yolo() {
    let dir = crate::paths::test_dir("esc274-gemini-live-fence");
    let m = meta(&dir, "bypassPermissions");
    let secret = dir.join("Prive");
    std::fs::create_dir_all(&secret).unwrap();
    std::fs::write(secret.join("secret.txt"), "MOT-DE-PASSE\n").unwrap();
    let fence = crate::fence::Fence::new(vec![secret.clone()]);
    let mut live = Live::start(&dir, m, fence).await;
    let log = live.launched();
    assert!(log.contains("--policy"), "{log}");
    assert!(log.contains("\"modeId\":\"yolo\""), "{log}");
    live.send(&format!("lis {}", secret.join("secret.txt").display()));
    live.until("its end", |items| items.iter().any(|i| i["kind"] == "turn"))
        .await;
    let items = live.rt.conv.items();
    let read = items.iter().find(|i| i["kind"] == "tool").unwrap();
    assert_eq!(read["status"], "error", "{items:#?}");
    assert!(!items.iter().any(|i| i.to_string().contains("MOT-DE-PASSE")));
    // Another file reads.
    std::fs::write(dir.join("repo").join("ok.txt"), "lisible\n").unwrap();
    live.send("lis ok.txt");
    live.until("its end", |items| {
        items.iter().filter(|i| i["kind"] == "turn").count() == 2
    })
    .await;
    assert!(live.texts().iter().any(|t| t == "Le fichier dit : lisible"));
    live.stop().await;
    // The rules, in the agent's own home.
    assert!(live.home.join(crate::engine::gemini::POLICY_FILE).is_file());
}

/// The files Gemini CLI itself writes in `~/.gemini` (not Antigravity CLI's `antigravity-cli/`),
/// each by its path, size and time: read for their metadata only, never their content.
fn stamps(
    dir: &std::path::Path,
) -> std::collections::BTreeMap<String, (u64, std::time::SystemTime)> {
    let mut out = std::collections::BTreeMap::new();
    let mut todo = vec![dir.to_path_buf()];
    while let Some(d) = todo.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            let rel = p.strip_prefix(dir).unwrap().to_string_lossy().into_owned();
            if rel.starts_with("antigravity-cli") {
                continue;
            }
            let Ok(md) = e.metadata() else { continue };
            if md.is_dir() {
                todo.push(p);
            } else {
                out.insert(
                    rel,
                    (md.len(), md.modified().unwrap_or(std::time::UNIX_EPOCH)),
                );
            }
        }
    }
    out
}

/// ESC-274, by hand, the day an account is there: a short real run of the machine's Gemini CLI
/// (`ESCOUADE_TEST_REAL_GEMINI`, its `gemini.cmd` or `gemini`), on its sign-in (its keychain or
/// `GEMINI_API_KEY`, never read), in a `GEMINI_CLI_HOME` of its own: a reply, a command whose
/// permission is allowed from its card, then, in yolo, a file of a private project its rules refuse.
/// The files of the user's `~/.gemini` (shared with Antigravity CLI, whose own are left out) are
/// compared by their size and time before and after, never read. Never run in this ticket (no
/// account).
#[tokio::test]
#[ignore = "real Gemini CLI: ESCOUADE_TEST_REAL_GEMINI=<gemini>"]
async fn real_gemini() {
    let program = std::env::var("ESCOUADE_TEST_REAL_GEMINI").expect("ESCOUADE_TEST_REAL_GEMINI");
    let program = GEMINI.locate(&program, "").expect("its gemini");
    let user = dirs::home_dir().unwrap().join(".gemini");
    let before = stamps(&user);
    let dir = crate::paths::test_dir("esc274-real-gemini");
    let m = AgentMeta {
        model: crate::engine::gemini::DEFAULT.into(),
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
        Vec::new(),
    )
    .await;
    live.send("Reply with the single word: ok.");
    live.until("its reply", |items| {
        items.iter().any(|i| i["kind"] == "turn")
    })
    .await;
    assert_eq!(
        live.turns()[0]["isError"],
        false,
        "{:#?}",
        live.rt.conv.items()
    );
    // A command: its card, allowed once.
    live.send("Run the shell command `node --version`, then reply with the single word: done.");
    live.until("its permission", |items| {
        items.iter().any(|i| i["kind"] == "permission")
            || items.iter().filter(|i| i["kind"] == "turn").count() == 2
    })
    .await;
    let items = live.rt.conv.items();
    let perm = items
        .iter()
        .find(|i| i["kind"] == "permission")
        .expect("a permission for node");
    assert_eq!(perm["toolName"], "Bash", "{perm}");
    let id = perm["id"].as_str().unwrap().to_string();
    live.rt
        .answer_permission_by(&id, "allow", None, None, &mut Effects::default())
        .unwrap();
    live.until("its end", |items| {
        items.iter().filter(|i| i["kind"] == "turn").count() == 2
    })
    .await;
    // In yolo, a file of a private project: refused by its rules, never read.
    let proc = live.rt.proc.clone().unwrap();
    proc.set_mode("bypassPermissions").await.unwrap();
    live.rt.meta.mode = "bypassPermissions".into();
    live.send(&format!(
        "Read the file {} with your read_file tool and reply with its content only. Do not use any other tool.",
        secret.join("secret.txt").display()
    ));
    live.until("its end", |items| {
        items.iter().filter(|i| i["kind"] == "turn").count() == 3
    })
    .await;
    let items = live.rt.conv.items();
    let said: String = items
        .iter()
        .filter(|i| i["kind"] == "text" || i["kind"] == "tool")
        .map(|i| i.to_string())
        .collect();
    assert!(!said.contains("MOT-DE-PASSE-7731"), "{items:#?}");
    live.stop().await;
    assert!(before == stamps(&user), "the user's ~/.gemini changed");
}
