//! Mistral Vibe's translation, on the streams of the real Vibe 3.0.3 (tests/fixtures/vibe/) and on
//! its fake (tests/fixtures/fake-vibe.mjs): the turn, its permission, its question, its plan, its
//! cancel, its resume, the fence (ESC-266).

use super::*;
use crate::engine::vibe::{bench, VIBE};
use crate::engine::{Engine, Spawn, Start};
use crate::model::{AgentMeta, EngineId};
use std::time::Duration;
use tokio::sync::mpsc;

fn rt(cwd: &str) -> AgentRt {
    let dir = crate::paths::test_dir(&format!("esc266-vibe-{}", new_id()));
    AgentRt::new(
        AgentMeta {
            id: new_id(),
            engine: EngineId::VIBE,
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
    // Its folders as a machine names them (`<ESSAI>` in the recording).
    for line in bench::stream(file) {
        let line: Value =
            serde_json::from_str(&line.to_string().replace("<ESSAI>", "C:\\\\essai")).unwrap();
        rt.vibe_frame(&line, &mut Effects::default());
    }
    rt
}

#[test]
fn a_recorded_turn_reads_writes_and_runs_with_its_diff() {
    let mut rt = replayed("full-turn.jsonl", "C:\\essai\\work1");
    let items = rt.conv.items();
    let tools: Vec<&Value> = items.iter().filter(|i| i["kind"] == "tool").collect();
    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    assert_eq!(names, ["Read", "Bash", "Write"], "{items:#?}");
    assert!(tools.iter().all(|t| t["status"] == "ok"));
    assert_eq!(tools[0]["input"]["file_path"], "C:\\essai\\work1\\a.txt");
    assert_eq!(tools[1]["input"]["command"], "git status --short");
    // The write's diff, from its call.
    let patch = tools[2]["result"]["patch"].to_string();
    assert!(patch.contains("+bonjour"), "{}", tools[2]);
    // Its permission, named by its call alone, has the call's file and diff.
    let perm = items.iter().find(|i| i["kind"] == "permission").unwrap();
    assert_eq!(perm["toolName"], "Write");
    assert_eq!(perm["input"]["file_path"], "C:\\essai\\work1\\b.txt");
    assert_eq!(perm["decision"], "allow");
    assert!(perm["patch"].to_string().contains("bonjour"));
    // Reasoning, then the reply, then the turn.
    assert!(items.iter().any(|i| i["kind"] == "thinking"));
    let text = items.iter().find(|i| i["kind"] == "text").unwrap();
    assert_eq!(text["text"], "done");
    let turn = items.last().unwrap();
    assert_eq!(turn["kind"], "turn");
    assert_eq!(turn["isError"], false);
    assert_eq!(
        rt.meta.session_id.as_deref(),
        Some("6fef71c6-4e62-2f63-6499-5f91b842880e")
    );
}

#[test]
fn a_recorded_question_is_a_card_answered_by_its_choice() {
    let mut rt = replayed("question-elicitation.jsonl", "C:\\essai\\work1");
    let items = rt.conv.items();
    let q = items.iter().find(|i| i["kind"] == "question").unwrap();
    assert_eq!(q["questions"][0]["question"], "blue or red?");
    assert_eq!(q["questions"][0]["options"][1]["label"], "red");
    assert_eq!(q["answers"], json!({ "blue or red?": "red" }));
    // Its call is not shown beside its card.
    assert!(!items.iter().any(|i| i["kind"] == "tool"), "{items:#?}");
}

#[test]
fn a_recorded_task_list_and_subagent_go_to_the_plan() {
    let mut rt = replayed("todo-subagent.jsonl", "C:\\essai\\work1");
    let items = rt.conv.items();
    let task = items
        .iter()
        .find(|i| i["kind"] == "tool" && i["name"] == "Task")
        .expect("the subagent's call");
    assert_eq!(task["input"]["description"], "Count the lines of a.txt");
    // Its task list and its search of tools are no calls.
    assert!(!items.iter().any(|i| i["kind"] == "tool"
        && matches!(
            i["name"].as_str(),
            Some("vibe.todo" | "search_tool_functions")
        )));
    let plan = rt.meta.plan.clone().expect("its plan");
    assert!(!plan.agents.is_empty());
}

#[test]
fn an_mcp_tool_of_vibe_takes_claude_codes_name() {
    let n = normalized(
        &json!({
            "toolCallId": "effect-1", "kind": "other", "status": "in_progress",
            "rawInput": { "x": 1 }, "_meta": { "tool_name": "mcp_escouade.get_board" },
        }),
        "C:\\repo",
    )
    .unwrap();
    let (name, input) = cursor::tool_of(&n, "C:\\repo");
    assert_eq!(name, "mcp__escouade__get_board");
    assert_eq!(input, json!({ "x": 1 }));
    // Its own tools shown otherwise, or not at all.
    for tool in ["vibe.todo", "ui.ask_user_question", "search_tool_functions"] {
        assert!(normalized(&json!({ "_meta": { "tool_name": tool } }), "C:\\repo").is_none());
    }
    // Its files named from the folder.
    let w = normalized(
        &json!({ "kind": "edit", "content": [{ "type": "diff", "path": "b.txt", "newText": "x" }],
                 "_meta": { "tool_name": "write_file" } }),
        "C:\\repo",
    )
    .unwrap();
    assert_eq!(w["content"][0]["path"], "C:\\repo\\b.txt");
}

/// What the process says, as the core passes it on.
enum Heard {
    Frame(Value),
    Exit,
}

/// An agent of Vibe on the fake, in `dir`, its frames translated as the core does.
struct Live {
    rt: AgentRt,
    heard: mpsc::UnboundedReceiver<Heard>,
    log: std::path::PathBuf,
    home: std::path::PathBuf,
}

impl Live {
    async fn start(dir: &std::path::Path, meta: AgentMeta, fence: crate::fence::Fence) -> Live {
        let program = VIBE.locate(&bench::fake_vibe(), "").unwrap();
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
        let home = dir.join("vibe-home");
        let log = dir.join("fake-vibe.jsonl");
        let env = vec![
            ("VIBE_HOME".into(), home.to_string_lossy().into_owned()),
            ("FAKE_VIBE_LOG".into(), log.to_string_lossy().into_owned()),
        ];
        let mut spawn = Spawn::of(&meta, program, env);
        spawn.fence = fence;
        spawn.escouade = escouade;
        let (tx, heard) = mpsc::unbounded_channel();
        let exit = tx.clone();
        let proc = VIBE
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
                VIBE.translate(&mut self.rt, &f, &mut Effects::default());
            }
        }
    }

    fn send(&mut self, text: &str) {
        let m = VIBE.message(text, &[]).unwrap();
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
                    VIBE.translate(&mut self.rt, &f, &mut Effects::default())
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
        engine: EngineId::VIBE,
        cwd: cwd.to_string_lossy().into_owned(),
        model: "devstral-2".into(),
        effort: "low".into(),
        mode: mode.into(),
        ..Default::default()
    }
}

#[tokio::test]
async fn a_turn_on_the_fake_has_its_model_thinking_agent_and_server_without_its_token_in_the_arguments(
) {
    let dir = crate::paths::test_dir("esc266-vibe-live-turn");
    let mut live = Live::start(&dir, meta(&dir, "acceptEdits"), Default::default()).await;
    live.send("modèle");
    live.until("its reply", |items| {
        items.iter().any(|i| i["kind"] == "turn")
    })
    .await;
    let text = live
        .rt
        .conv
        .items()
        .into_iter()
        .find(|i| i["kind"] == "text")
        .unwrap();
    assert_eq!(
        text["text"],
        "Je tourne sur devstral-2, réflexion off, agent accept-edits."
    );
    let log = live.launched();
    // Escouade's server by the session, its token in a header; never on the command line.
    assert!(log.contains("\"argv\":[]"), "{log}");
    assert!(log.contains("Bearer s3cr3t-jeton"));
    assert!(!log
        .lines()
        .any(|l| l.contains("\"launch\"") && l.contains("s3cr3t")));
    // Its own home, no telemetry nor update, the folder trusted there.
    assert!(log.contains("\"telemetry\":\"false\""), "{log}");
    assert!(live.home.join("trusted_folders.toml").is_file());
    live.send("confiance");
    live.until("its reply", |items| {
        items.iter().filter(|i| i["kind"] == "turn").count() == 2
    })
    .await;
    assert!(live
        .rt
        .conv
        .items()
        .iter()
        .any(|i| i["kind"] == "text" && i["text"] == "Dossier de confiance."));
    live.stop().await;
}

#[tokio::test]
async fn a_permission_refused_cuts_the_turn_and_always_holds_for_the_agent() {
    let dir = crate::paths::test_dir("esc266-vibe-live-perm");
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
    // The card has the command its call told.
    assert_eq!(perm["toolName"], "Bash");
    assert_eq!(perm["input"]["command"], "npm test");
    assert_eq!(perm["alwaysScope"], "agent");
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
    // « Toujours »: kept in the agent's own VIBE_HOME, asked no more.
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
    let config = std::fs::read_to_string(live.home.join("config.toml")).unwrap();
    assert!(config.contains("\"npm\""), "{config}");
    live.send("commande");
    live.until("its end", |items| {
        items.iter().filter(|i| i["kind"] == "turn").count() == 3
    })
    .await;
    assert_eq!(
        live.rt
            .conv
            .items()
            .iter()
            .filter(|i| i["kind"] == "permission")
            .count(),
        2
    );
    live.stop().await;
}

#[tokio::test]
async fn a_write_in_the_folder_goes_through_and_one_outside_asks_with_its_diff() {
    let dir = crate::paths::test_dir("esc266-vibe-live-write");
    let mut live = Live::start(&dir, meta(&dir, "default"), Default::default()).await;
    live.send("fichier");
    live.until("its end", |items| items.iter().any(|i| i["kind"] == "turn"))
        .await;
    let items = live.rt.conv.items();
    let w = items
        .iter()
        .find(|i| i["kind"] == "tool" && i["name"] == "Write")
        .unwrap();
    assert_eq!(w["status"], "ok");
    assert!(w["result"]["patch"].to_string().contains("note de Vibe"));
    assert!(!items.iter().any(|i| i["kind"] == "permission"));
    assert!(!live.rt.meta.touched_files.is_empty());
    live.send("fichier dehors");
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
    assert!(perm["patch"].to_string().contains("note de Vibe"), "{perm}");
    assert!(perm["description"]
        .as_str()
        .unwrap_or_default()
        .contains("dossier"));
    live.stop().await;
}

#[tokio::test]
async fn a_question_is_answered_from_its_card() {
    let dir = crate::paths::test_dir("esc266-vibe-live-question");
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
    assert!(live
        .rt
        .conv
        .items()
        .iter()
        .any(|i| i["kind"] == "text" && i["text"] == "Tu as choisi bleu."));
    live.stop().await;
}

#[tokio::test]
async fn in_the_mode_plan_its_last_words_are_a_plan_to_approve_then_carried_out() {
    let dir = crate::paths::test_dir("esc266-vibe-live-plan");
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
        .contains("2. Écrire son test."));
    // Its tasks in the plan; the turn waits.
    assert_eq!(live.rt.meta.status, AgentStatus::Waiting);
    assert!(live.turns().is_empty());
    let log = live.launched();
    assert!(log.contains("escouade_plan_mode"));
    assert!(log.contains("\"modeId\":\"ask\""));
    let id = card["id"].as_str().unwrap().to_string();
    live.rt
        .answer_permission_by(&id, "always", None, None, &mut Effects::default())
        .unwrap();
    live.until("its end", |items| items.iter().any(|i| i["kind"] == "turn"))
        .await;
    assert_eq!(live.rt.meta.mode, "acceptEdits");
    assert!(live.launched().contains("\"modeId\":\"accept-edits\""));
    assert!(live
        .rt
        .conv
        .items()
        .iter()
        .any(|i| i["kind"] == "text" && i["text"] == "Plan approuvé, je le mets en œuvre."));
    assert_eq!(live.turns().len(), 1);
    live.stop().await;
}

#[tokio::test]
async fn a_session_is_resumed_and_copied_by_a_new_process() {
    let dir = crate::paths::test_dir("esc266-vibe-live-resume");
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
    again.stop().await;
    let copy = AgentMeta {
        id: new_id(),
        fork_of: Some(session.clone()),
        ..m
    };
    let mut forked = Live::start(&dir, copy, Default::default()).await;
    let own = forked.rt.meta.session_id.clone().unwrap();
    assert_ne!(own, session);
    assert!(forked.launched().contains("session/fork"));
    forked.stop().await;
}

/// « Projet privé »: a permission naming a path of the fence is refused at once, never a card.
#[tokio::test]
async fn a_write_naming_a_fenced_path_is_refused_without_a_card() {
    let dir = crate::paths::test_dir("esc266-vibe-live-fence");
    // In Bypass too: Vibe kept in its agent `ask`, its `auto-approve` letting all through.
    let m = meta(&dir, "bypassPermissions");
    let outside = std::path::Path::new(&m.cwd)
        .parent()
        .unwrap()
        .join("vibe-dehors.txt");
    let fence = crate::fence::Fence::new(vec![outside]);
    let mut live = Live::start(&dir, m, fence).await;
    live.send("fichier dehors");
    live.until("its end", |items| items.iter().any(|i| i["kind"] == "turn"))
        .await;
    let items = live.rt.conv.items();
    assert!(
        !items.iter().any(|i| i["kind"] == "permission"),
        "{items:#?}"
    );
    let w = items.iter().find(|i| i["kind"] == "tool").unwrap();
    assert_eq!(w["status"], "error");
    assert!(!dir.join("vibe-dehors.txt").exists());
    let log = live.launched();
    assert!(log.contains("\"modeId\":\"ask\""), "{log}");
    assert!(!log.contains("auto-approve\"}"), "{log}");
    // Its file tools refused the fence in its own config.
    let config = std::fs::read_to_string(live.home.join("config.toml")).unwrap();
    assert!(
        config.contains("[tools.read_file]") && config.contains("denylist"),
        "{config}"
    );
    live.stop().await;
}

/// Every file of `dir` with what it holds, by its path from `dir`: the user's `~/.vibe` before and
/// after a real run.
fn snapshot(dir: &std::path::Path) -> std::collections::BTreeMap<String, Vec<u8>> {
    let mut out = std::collections::BTreeMap::new();
    let mut todo = vec![dir.to_path_buf()];
    while let Some(d) = todo.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                todo.push(p);
            } else if let Ok(bytes) = std::fs::read(&p) {
                let rel = p.strip_prefix(dir).unwrap().to_string_lossy().into_owned();
                out.insert(rel, bytes);
            }
        }
    }
    out
}

/// ESC-266, by hand: a short real run of the machine's Mistral Vibe (`ESCOUADE_TEST_REAL_VIBE`, its
/// `vibe-acp`), on its account (its key stays in the keychain, never read), in a `VIBE_HOME` of its
/// own: a command whose permission is allowed from its card, then, in Bypass, a file of a private
/// project its fence refuses. The user's `~/.vibe` is compared byte for byte before and after.
/// About 0.03 $ on Mistral Medium 3.5.
#[tokio::test]
#[ignore = "real Mistral Vibe: ESCOUADE_TEST_REAL_VIBE=<vibe-acp>"]
async fn real_vibe() {
    let program = std::env::var("ESCOUADE_TEST_REAL_VIBE").expect("ESCOUADE_TEST_REAL_VIBE");
    let program = VIBE.locate(&program, "").expect("its vibe-acp");
    let user = dirs::home_dir().unwrap().join(".vibe");
    let before = snapshot(&user);
    let dir = crate::paths::test_dir("esc266-real-vibe");
    let m = meta(&dir, "default");
    let m = AgentMeta {
        model: crate::engine::vibe::DEFAULT.into(),
        effort: "low".into(),
        ..m
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
    // A command off its list: its card, allowed once.
    live.send("Run the shell command `node --version` with your bash tool, then reply with the single word: done.");
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
        .expect("a permission for node");
    assert_eq!(perm["toolName"], "Bash", "{perm}");
    assert!(perm["input"]["command"]
        .as_str()
        .unwrap_or_default()
        .contains("node"));
    let id = perm["id"].as_str().unwrap().to_string();
    live.rt
        .answer_permission_by(&id, "allow", None, None, &mut Effects::default())
        .unwrap();
    live.until("its end", |items| items.iter().any(|i| i["kind"] == "turn"))
        .await;
    let items = live.rt.conv.items();
    assert!(items
        .iter()
        .any(|i| i["kind"] == "tool" && i["name"] == "Bash" && i["status"] == "ok"));
    assert_eq!(live.turns()[0]["isError"], false, "{items:#?}");
    // In Bypass, a file of a private project: refused by its config, never read.
    let proc = live.rt.proc.clone().unwrap();
    proc.set_mode("bypassPermissions").await.unwrap();
    live.rt.meta.mode = "bypassPermissions".into();
    let file = secret.join("secret.txt");
    live.send(&format!(
        "Read the file {} with your read_file tool and reply with its content only. Do not use any other tool.",
        file.display()
    ));
    live.until("its end", |items| {
        items.iter().filter(|i| i["kind"] == "turn").count() == 2
    })
    .await;
    let items = live.rt.conv.items();
    let said: String = items
        .iter()
        .filter(|i| i["kind"] == "text" || i["kind"] == "tool")
        .map(|i| i.to_string())
        .collect();
    assert!(!said.contains("MOT-DE-PASSE-7731"), "{items:#?}");
    assert!(
        items
            .iter()
            .any(|i| i["kind"] == "tool" && i["name"] == "Read" && i["status"] == "error"),
        "{items:#?}"
    );
    live.stop().await;
    assert!(before == snapshot(&user), "the user's ~/.vibe changed");
}
