//! Tests of the facts' bus (PLG-5, ESC-177): each fact emitted by the core where its state changes,
//! on the fake harnesses (tests/fixtures/fake-claude, the bench's Minimal), and received by the
//! fake plugin (tests/fixtures/fake-plugin.mjs, `--keep-facts`); a private project's facts kept
//! from a plugin not turned on for it; a webhook received by a fake HTTP server; a command run on a
//! fact once agreed to; a full queue that drops its oldest facts and says it in the journal.

use crate::core_tests::{agent_on, harness, pro_out_of_quota_and_team, repo, turn_over, Harness};
use crate::facts::{About, Body, Bus, Recorder, NAMES};
use crate::i18n::Lang;
use crate::integrations::secrets::{MemoryStore, SecretStore};
use crate::model::*;
use crate::paths::DataDir;
use crate::plugins::events::Dispatch;
use crate::plugins::journal::Kind;
use crate::plugins::runtime::{secret_name, Limits, PhaseName, Runtime};
use crate::plugins::Plugins;
use crate::tickets_tests::{conflicting, draft, github_remote, reviewed};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const FAKE: &str = "dev.test.facts";

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures")
}

/// A folder holding a plugin of `manifest` (its `id`, `name`, `version` and `escouade` added), the
/// fake plugin's program beside it.
fn plugin_dir(base: &Path, folder: &str, mut manifest: Value) -> PathBuf {
    let dir = base.join(folder);
    std::fs::create_dir_all(&dir).unwrap();
    for f in ["fake-plugin.mjs", "fake-plugin.cmd", "fake-plugin"] {
        std::fs::copy(fixtures().join(f), dir.join(f)).unwrap();
    }
    manifest["name"] = json!(folder);
    manifest["version"] = json!("1.0.0");
    manifest["escouade"] = json!({ "api": "1", "min": "2.1.0" });
    std::fs::write(
        dir.join("escouade-plugin.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();
    dir
}

/// The fake plugin's program that keeps every fact it is told.
fn keeper(id: &str, events: &[&str]) -> Value {
    json!({
        "id": id,
        "run": { "command": "fake-plugin", "windows": "fake-plugin.cmd", "args": ["--keep-facts"] },
        "activation": ["event:*"],
        "permissions": { "events": events },
    })
}

/// What a test hears: every fact of the bus, and the fake plugin's.
struct Listening {
    rec: Arc<Recorder>,
}

/// The fake plugin, which asks for every fact, loaded in development in the harness's core.
fn listen(h: &Harness) -> Listening {
    // Else cmd.exe may no longer find node (see `trim_test_path`).
    #[cfg(windows)]
    crate::paths::trim_test_path();
    let dir = plugin_dir(&h.dir, "fake-plugin", keeper(FAKE, &["*"]));
    h.core.plugins.load_dev(&dir, Lang::En).unwrap();
    let rec = Arc::new(Recorder::default());
    h.core.facts.subscribe(rec.clone());
    Listening { rec }
}

/// The facts the plugin `id` was told, as it kept them.
async fn told(h: &Harness, id: &str) -> Vec<Value> {
    let state = h
        .core
        .plugin_runtime
        .call(id, "test/state", json!({}))
        .await
        .unwrap_or_else(|e| panic!("{e}: {:#?}", h.core.plugin_runtime.journal(id)));
    state["facts"].as_array().cloned().unwrap_or_default()
}

fn names(facts: &[Value]) -> BTreeSet<String> {
    facts
        .iter()
        .filter_map(|f| f["name"].as_str().map(str::to_string))
        .collect()
}

impl Listening {
    /// Waits until the bus said each of `wanted` and the fake plugin was told it; gives what the
    /// plugin was told.
    async fn heard(&self, h: &Harness, wanted: &[&str]) -> Vec<Value> {
        let started = Instant::now();
        loop {
            h.core.facts.flush();
            let said: BTreeSet<String> = self.rec.facts().iter().map(|f| f.name()).collect();
            let missing: Vec<&&str> = wanted.iter().filter(|n| !said.contains(**n)).collect();
            if missing.is_empty() {
                break;
            }
            assert!(
                started.elapsed() < Duration::from_secs(30),
                "never said: {missing:?} (said {said:?})"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        loop {
            let facts = told(h, FAKE).await;
            let got = names(&facts);
            let missing: Vec<&&str> = wanted.iter().filter(|n| !got.contains(**n)).collect();
            if missing.is_empty() {
                // As the core makes them, they hold to the protocol's schema the SDK is generated from.
                for f in &facts {
                    crate::plugins_protocol_tests::holds(f, "events/fact", "params");
                }
                return facts;
            }
            assert!(
                started.elapsed() < Duration::from_secs(30),
                "the fake plugin never got {missing:?} (got {got:?})"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }
}

/// The fact `name` about the ticket `ticket`.
fn one<'a>(facts: &'a [Value], name: &str, ticket: &str) -> &'a Value {
    facts
        .iter()
        .find(|f| f["name"] == name && f["ticket"]["id"] == ticket)
        .unwrap_or_else(|| panic!("no {name} of {ticket}"))
}

fn of<'a>(facts: &'a [Value], name: &str) -> Vec<&'a Value> {
    facts.iter().filter(|f| f["name"] == name).collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agents_facts_reach_the_plugin_with_their_names_never_the_conversation() {
    let h = harness("facts-agent");
    let l = listen(&h);
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.turn(&id, "Bonjour").await;
    h.core
        .send_message(&id, "permission".into(), vec![])
        .await
        .unwrap();
    h.wait("the permission", |h| {
        h.agent(&id).status == AgentStatus::Waiting
    })
    .await;
    let req = h
        .items(&id)
        .into_iter()
        .find(|i| i["kind"] == "permission")
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    h.core.answer_permission(&id, &req, "deny", None).unwrap();
    h.wait("the turn over", |h| !h.agent(&id).status.is_active())
        .await;
    h.core
        .send_message(&id, "question".into(), vec![])
        .await
        .unwrap();
    h.wait("the question", |h| {
        h.agent(&id).status == AgentStatus::Waiting
    })
    .await;
    let q = h
        .items(&id)
        .into_iter()
        .find(|i| i["kind"] == "question")
        .unwrap();
    let question = q["questions"][0]["question"].as_str().unwrap().to_string();
    h.core
        .answer_question(
            &id,
            q["id"].as_str().unwrap(),
            json!({ question.clone(): "SQLite" }),
        )
        .unwrap();
    h.wait("the turn over", |h| !h.agent(&id).status.is_active())
        .await;
    // Named after its first message (ESC-156), then archived: the last fact says its new name.
    h.wait("its name", |h| h.agent(&id).named).await;
    h.core.archive_agent(&id, true).await.unwrap();

    let facts = l
        .heard(
            &h,
            &[
                "project.added",
                "agent.created",
                "agent.renamed",
                "agent.status",
                "turn.ended",
                "request.permission",
                "request.question",
                "request.answered",
                "agent.stopped",
            ],
        )
        .await;
    let agent = h.agent(&id);
    let created = of(&facts, "agent.created")[0];
    assert_eq!(created["version"], 1);
    // Said with the name it had then; its naming after its first message is a fact of its own, and
    // the facts after it say the new name.
    let renamed = of(&facts, "agent.renamed")[0];
    assert_eq!(created["agent"]["id"], id.as_str());
    assert_eq!(created["agent"]["name"], renamed["data"]["from"]);
    assert_eq!(renamed["data"]["to"], agent.name.as_str());
    assert_eq!(renamed["agent"], json!({ "id": id, "name": agent.name }));
    let stopped = of(&facts, "agent.stopped")[0];
    assert_eq!(stopped["agent"], json!({ "id": id, "name": agent.name }));
    assert_eq!(
        created["project"],
        json!({ "id": p.id, "name": "demo", "private": false })
    );
    assert_eq!(created["data"]["harness"], "claude");
    let statuses: Vec<&Value> = of(&facts, "agent.status")
        .iter()
        .map(|f| &f["data"]["status"])
        .collect();
    assert!(statuses.contains(&&json!("running")), "{statuses:?}");
    assert!(statuses.contains(&&json!("waiting")), "{statuses:?}");
    assert!(statuses.contains(&&json!("done")), "{statuses:?}");
    let ended = of(&facts, "turn.ended");
    assert_eq!(ended[0]["data"], json!({ "end": "finished" }));
    let perm = of(&facts, "request.permission")[0];
    assert_eq!(perm["data"]["requestId"], req.as_str());
    assert!(perm["data"]["tool"].is_string());
    assert_eq!(
        of(&facts, "request.question")[0]["data"]["questions"],
        json!([question])
    );
    let answers: Vec<&Value> = of(&facts, "request.answered")
        .iter()
        .map(|f| &f["data"])
        .collect();
    assert!(
        answers.contains(&&json!({ "requestId": req, "kind": "permission", "decision": "deny" }))
    );
    assert!(answers.iter().any(|a| a["kind"] == "question"));
    assert_eq!(
        of(&facts, "agent.stopped")[0]["data"],
        json!({ "removed": false })
    );
    // The facts number in the order they came, and never carry what was said.
    let seqs: Vec<u64> = facts.iter().map(|f| f["seq"].as_u64().unwrap()).collect();
    assert!(seqs.windows(2).all(|w| w[0] < w[1]), "{seqs:?}");
    let reply = h
        .items(&id)
        .into_iter()
        .find(|i| i["kind"] == "text")
        .and_then(|i| i["text"].as_str().map(str::to_string))
        .unwrap_or_default();
    assert!(!reply.is_empty());
    let all = serde_json::to_string(&facts).unwrap();
    assert!(!all.contains(&reply), "a fact carried the conversation");
    assert!(
        !all.contains("Bonjour"),
        "a fact carried the user's message"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_tickets_facts_reach_the_plugin_from_its_creation_to_its_merge() {
    let h = harness("facts-ticket");
    let l = listen(&h);
    let (p, _) = h.project(false).await;
    h.set_board(&p.id, |s| {
        s.tests_first = true;
        s.test_command = "node -e \"process.exit(0)\"".into();
    });
    let (t, _) = reviewed(&h, &p.id, "Fichier [ok]").await;
    h.core.ticket_approve(&t.id).await.unwrap();
    assert_eq!(
        h.ticket(&t.id).column,
        Column::Done,
        "{:?}",
        h.ticket(&t.id).blocked
    );
    // Another one sent back to its agent.
    let (u, _) = reviewed(&h, &p.id, "Autre [ok]").await;
    h.core
        .ticket_reject(&u.id, "Encore un effort")
        .await
        .unwrap();

    let facts = l
        .heard(
            &h,
            &[
                "ticket.created",
                "ticket.column",
                "ticket.report",
                "worktree.created",
                "ticket.approved",
                "tests.result",
                "ticket.merged",
                "worktree.removed",
                "ticket.rejected",
            ],
        )
        .await;
    let about_t = |f: &&&Value| f["ticket"]["id"] == t.id.as_str();
    let c = one(&facts, "ticket.created", &t.id);
    assert_eq!(c["ticket"]["key"], "DEM-1");
    assert_eq!(c["ticket"]["title"], "Fichier [ok]");
    assert_eq!(c["project"]["name"], "demo");
    assert_eq!(c["data"], json!({ "column": "todo" }));
    let moves: Vec<(String, String)> = of(&facts, "ticket.column")
        .iter()
        .filter(about_t)
        .map(|f| {
            (
                f["data"]["from"].as_str().unwrap().to_string(),
                f["data"]["to"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    let mv = |a: &str, b: &str| (a.to_string(), b.to_string());
    assert_eq!(
        moves,
        [
            mv("todo", "doing"),
            mv("doing", "review"),
            mv("review", "done")
        ]
    );
    let report = one(&facts, "ticket.report", &t.id);
    let (met, total) = (&report["data"]["met"], &report["data"]["total"]);
    assert!(total.as_u64().unwrap() > 0 && met == total, "{report}");
    // The agent's facts say its ticket.
    assert!(of(&facts, "agent.created")
        .iter()
        .any(|f| f["ticket"]["key"] == "DEM-1"));
    assert_eq!(
        one(&facts, "tests.result", &t.id)["data"],
        json!({ "passed": true })
    );
    let merged = one(&facts, "ticket.merged", &t.id);
    assert_eq!(merged["data"]["target"], "main");
    assert_eq!(
        of(&facts, "ticket.rejected")[0]["data"],
        json!({ "comment": "Encore un effort" })
    );
    assert_eq!(of(&facts, "ticket.rejected")[0]["ticket"]["key"], "DEM-2");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_conflict_a_pull_request_and_a_projects_closing_are_facts() {
    let h = harness("facts-conflict-pr");
    let l = listen(&h);
    let (p, r) = h.project(false).await;
    let (t, _) = conflicting(&h, &p.id, &r).await;
    h.core.ticket_approve(&t.id).await.unwrap();
    assert!(h.ticket(&t.id).conflict);
    // A pull request, finished in the browser without `gh`.
    let (q, rq) = {
        let rq = repo(&h.dir.join("pr"));
        let q = h
            .core
            .create_project(
                &rq.to_string_lossy(),
                "prs",
                "oklch(0.7 0.1 200)",
                false,
                None,
                true,
            )
            .await
            .unwrap();
        (q, rq)
    };
    github_remote(&h, &rq);
    *h.core.gh.write() = None;
    h.set_board(&q.id, |s| s.action = "pr".into());
    let (u, _) = reviewed(&h, &q.id, "Fichier [ok]").await;
    h.core.ticket_approve(&u.id).await.unwrap();
    h.core.remove_project(&q.id, true).await.unwrap();

    let facts = l
        .heard(&h, &["ticket.conflict", "pr.opened", "project.closed"])
        .await;
    let conflict = of(&facts, "ticket.conflict")[0];
    assert_eq!(conflict["data"]["target"], "main");
    assert_eq!(conflict["data"]["files"], json!(["src/app.ts"]));
    let pr = of(&facts, "pr.opened")[0];
    assert!(pr["data"]["url"]
        .as_str()
        .unwrap()
        .starts_with("https://github.com/acme/demo/compare/main...ticket/"));
    assert_eq!(pr["ticket"]["id"], u.id.as_str());
    let closed = of(&facts, "project.closed")[0];
    assert_eq!(
        closed["project"],
        json!({ "id": q.id, "name": "prs", "private": false })
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_test_launch_the_docs_the_memory_the_decisions_and_the_start_are_facts() {
    let h = harness("facts-launch-docs");
    let l = listen(&h);
    let (p, repo_dir) = h.project(true).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    // A test launch of the agent: started, ready, stopped.
    h.core.track_test_run(&id, "facts-run", "web").unwrap();
    h.core.test_run_ready(&id, "http://localhost:4141");
    h.core.test_run_ended("facts-run");
    // The docs: a page written, a decision proposed then accepted, a page compared.
    std::fs::write(repo_dir.join("paiement.rs"), "const MAX: u32 = 3;").unwrap();
    let space = h.core.docs_space(&p.id, None).await.unwrap();
    space
        .spec_write("paiement/echeances", |p| {
            p.body = "Quatre échéances. [écart]".into()
        })
        .unwrap();
    let d = space
        .decision_propose(
            None,
            crate::docs::Decision {
                title: "Payer en trois fois".into(),
                ..Default::default()
            },
        )
        .unwrap();
    space.decision_accept(d.number, "Guillaume").unwrap();
    h.core
        .docs_compare(&p.id, Some(vec!["paiement/echeances".into()]))
        .await
        .unwrap();
    // The memory, through the MCP server's tools.
    let caller = crate::mcp::Caller::External;
    crate::mcp::memory::write(
        &h.core,
        &caller,
        Some(&p.id),
        crate::memory::Draft {
            name: "build".into(),
            description: "Comment construire".into(),
            body: "cargo build".into(),
            ..Default::default()
        },
    )
    .unwrap();
    crate::mcp::memory::forget(&h.core, &caller, Some(&p.id), "build").unwrap();
    // The app's start.
    h.core.tell_started();

    let facts = l
        .heard(
            &h,
            &[
                "launch.started",
                "launch.ready",
                "launch.stopped",
                "docs.written",
                "decision.proposed",
                "decision.accepted",
                "docs.gaps",
                "memory.written",
                "memory.forgotten",
                "app.started",
            ],
        )
        .await;
    let started = of(&facts, "launch.started")[0];
    assert_eq!(started["agent"]["id"], id.as_str());
    assert_eq!(started["data"], json!({ "run": "web" }));
    assert_eq!(
        of(&facts, "launch.ready")[0]["data"],
        json!({ "url": "http://localhost:4141" })
    );
    // A page compared tells its gaps, not a page written.
    let written: Vec<&Value> = of(&facts, "docs.written")
        .iter()
        .map(|f| &f["data"])
        .collect();
    assert_eq!(written, [&json!({ "page": "paiement/echeances" })]);
    assert_eq!(
        of(&facts, "docs.gaps")[0]["data"],
        json!({ "page": "paiement/echeances", "gaps": 1 })
    );
    assert_eq!(
        of(&facts, "decision.accepted")[0]["data"],
        json!({ "number": d.number, "title": "Payer en trois fois" })
    );
    assert_eq!(
        of(&facts, "memory.written")[0]["data"],
        json!({ "memory": "build", "new": true })
    );
    let app = of(&facts, "app.started")[0];
    assert_eq!(app["data"]["version"], env!("CARGO_PKG_VERSION"));
    assert!(app.get("project").is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_change_of_account_and_of_harness_are_facts() {
    let h = harness("facts-accounts");
    let l = listen(&h);
    let (p, _) = h.project(true).await;
    let _ = pro_out_of_quota_and_team(&h);
    let id = agent_on(&h, &p, "pro").await.id;
    turn_over(&h, &id, "Bonjour la limite").await;
    h.core.resume_on_account(&id, "equipe").await.unwrap();
    h.wait("its turn on the other account", |h| {
        !h.agent(&id).status.is_active()
    })
    .await;
    let facts = l.heard(&h, &["turn.ended", "account.switched"]).await;
    assert!(of(&facts, "turn.ended")
        .iter()
        .any(|f| f["data"] == json!({ "end": "limited" })));
    let switched = of(&facts, "account.switched")[0];
    assert_eq!(switched["agent"]["id"], id.as_str());
    assert_eq!(switched["data"]["from"], "Pro");
    assert!(switched["data"]["to"].is_string());

    // On another harness: the bench's Minimal.
    let mut s = h.core.settings.read().clone();
    s.accounts.push(Account {
        id: "relais".into(),
        name: "Relais".into(),
        engine: EngineId::Bench,
        config_dir: h.dir.join("relais").to_string_lossy().into(),
        ..Default::default()
    });
    h.core.save_settings(s).unwrap();
    h.core
        .continue_on_engine(&id, EngineId::Bench)
        .await
        .unwrap();
    let facts = l.heard(&h, &["agent.handover"]).await;
    assert_eq!(
        of(&facts, "agent.handover")[0]["data"],
        json!({ "from": "claude", "to": "bench" })
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_private_projects_facts_go_only_to_a_plugin_turned_on_for_it() {
    let h = harness("facts-private");
    #[cfg(windows)]
    crate::paths::trim_test_path();
    let rec = Arc::new(Recorder::default());
    h.core.facts.subscribe(rec.clone());
    let (open, _) = h.project(false).await;
    let r = repo(&h.dir.join("coffre"));
    let vault = h
        .core
        .create_project(
            &r.to_string_lossy(),
            "coffre-fort",
            "oklch(0.7 0.1 200)",
            false,
            None,
            true,
        )
        .await
        .unwrap();
    h.core
        .projects
        .write()
        .iter_mut()
        .find(|x| x.id == vault.id)
        .unwrap()
        .private = true;
    for (folder, id) in [("tous", "dev.test.all"), ("coffre", "dev.test.vault")] {
        let dir = plugin_dir(&h.dir, folder, keeper(id, &["*"]));
        h.core.plugins.load_dev(&dir, Lang::En).unwrap();
    }
    // Turned on for the private project by hand; the other one is not.
    h.core
        .plugins
        .choose("dev.test.vault", |c| {
            c.projects.insert(vault.id.clone(), true);
        })
        .unwrap();
    // And a webhook-only plugin, never turned on for it, whose server must see nothing of it.
    let (url, mut bodies) = fake_server().await;
    let dir = plugin_dir(
        &h.dir,
        "webhook",
        json!({
            "id": "dev.test.hook",
            "permissions": { "events": ["ticket.created"], "network": ["127.0.0.1"] },
            "contributes": { "webhooks": [{ "url": url, "events": ["ticket.created"] }] },
        }),
    );
    h.core.plugins.load_dev(&dir, Lang::En).unwrap();
    h.set_board(&vault.id, |s| s.autopilot = false);
    h.set_board(&open.id, |s| s.autopilot = false);
    let secret = h
        .core
        .ticket_create(&vault.id, draft("Le plan secret", &[], 5))
        .await
        .unwrap();
    // A fact of an agent gone, whose project no one knows: kept from every plugin.
    h.core
        .facts
        .emit(Body::LaunchStopped {}, h.core.about_agent("parti"));
    let public = h
        .core
        .ticket_create(&open.id, draft("Le plan public", &[], 5))
        .await
        .unwrap();
    // Both said on the bus, the private one marked so.
    let started = Instant::now();
    loop {
        h.core.facts.flush();
        if rec.named("ticket.created").len() == 2 {
            break;
        }
        assert!(started.elapsed() < Duration::from_secs(10));
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let privates: Vec<bool> = rec
        .named("ticket.created")
        .iter()
        .map(|f| f.private)
        .collect();
    assert_eq!(privates, [true, false]);
    // The plugin turned on for it gets both; the other one the public one alone.
    let until_told = |id: &'static str, n: usize| {
        let h = &h;
        async move {
            let started = Instant::now();
            loop {
                let facts = told(h, id).await;
                if of(&facts, "ticket.created").len() >= n {
                    return facts;
                }
                assert!(started.elapsed() < Duration::from_secs(30), "{id}");
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }
    };
    let vaults = until_told("dev.test.vault", 2).await;
    assert_eq!(of(&vaults, "ticket.created").len(), 2);
    let alls = until_told("dev.test.all", 1).await;
    // The webhook's server got the public one.
    let body = tokio::time::timeout(Duration::from_secs(20), bodies.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(body.contains(&public.id));
    tokio::time::sleep(Duration::from_millis(500)).await;
    let alls_after = told(&h, "dev.test.all").await;
    for facts in [&alls, &alls_after, &vaults] {
        assert!(
            of(facts, "launch.stopped").is_empty(),
            "a fact of no known project went out"
        );
    }
    for facts in [&alls, &alls_after] {
        let all = serde_json::to_string(facts).unwrap();
        assert!(
            !all.contains(&secret.id)
                && !all.contains("coffre-fort")
                && !all.contains("Le plan secret")
        );
    }
    assert!(
        bodies.try_recv().is_err(),
        "the webhook was called for the private project"
    );
}

/// A fake HTTP server: its address, and the bodies of the requests it answered (204).
async fn fake_server() -> (String, tokio::sync::mpsc::UnboundedReceiver<String>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    tokio::spawn(async move {
        while let Ok((mut s, _)) = listener.accept().await {
            let tx = tx.clone();
            tokio::spawn(async move {
                let mut buf = Vec::new();
                let mut chunk = [0u8; 4096];
                loop {
                    let n = s.read(&mut chunk).await.unwrap_or(0);
                    if n == 0 {
                        break;
                    }
                    buf.extend_from_slice(&chunk[..n]);
                    let text = String::from_utf8_lossy(&buf).to_string();
                    if let Some(end) = text.find("\r\n\r\n") {
                        let head = text[..end].to_lowercase();
                        let len = head
                            .lines()
                            .find_map(|l| l.strip_prefix("content-length:"))
                            .and_then(|v| v.trim().parse::<usize>().ok())
                            .unwrap_or(0);
                        if buf.len() >= end + 4 + len {
                            let _ = tx.send(text);
                            let _ = s
                                .write_all(b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                                .await;
                            break;
                        }
                    }
                }
            });
        }
    });
    (format!("http://{addr}/hook"), rx)
}

#[tokio::test(flavor = "multi_thread")]
async fn a_declared_webhook_posts_its_template_to_a_server_with_a_secret_address() {
    let h = harness("facts-webhook");
    let (url, mut bodies) = fake_server().await;
    let (p, _) = h.project(false).await;
    h.set_board(&p.id, |s| s.autopilot = false);
    let dir = plugin_dir(
        &h.dir,
        "discord",
        json!({
            "id": "dev.test.discord",
            "permissions": {
                "events": ["ticket.*", "agent.status"],
                "network": ["127.0.0.1"],
                "secrets": ["webhook"],
            },
            "contributes": { "webhooks": [{
                "url": "{{secrets.webhook}}",
                "events": ["ticket.created"],
                "when": { "data.column": "todo" },
                "headers": { "X-Fact": "{{name}}" },
                "body": { "content": "Nouveau ticket {{ticket.key}} : {{ticket.title}} ({{project.name}})" },
            }] },
        }),
    );
    h.core.plugins.load_dev(&dir, Lang::En).unwrap();
    h.core
        .secrets
        .set(&secret_name("dev.test.discord", "webhook"), &url)
        .unwrap();
    h.core
        .ticket_create(&p.id, draft("Écrire \"la doc\"", &[], 5))
        .await
        .unwrap();
    let got = tokio::time::timeout(Duration::from_secs(20), bodies.recv())
        .await
        .expect("the webhook in time")
        .unwrap();
    let (head, body) = got.split_once("\r\n\r\n").unwrap();
    assert!(head.starts_with("POST /hook "), "{head}");
    let head = head.to_lowercase();
    assert!(head.contains("content-type: application/json"), "{head}");
    assert!(head.contains("x-fact: ticket.created"), "{head}");
    assert_eq!(
        serde_json::from_str::<Value>(body).unwrap(),
        json!({ "content": "Nouveau ticket DEM-1 : Écrire \"la doc\" (demo)" })
    );
    // Its journal says it, never the address made of the secret.
    // Written once its answer is read, a moment after the server got it.
    let started = Instant::now();
    let journal = loop {
        let j = h.core.plugin_runtime.journal("dev.test.discord");
        if j.iter().any(|e| e.kind == Kind::Webhook) {
            break j;
        }
        assert!(started.elapsed() < Duration::from_secs(10), "{j:#?}");
        tokio::time::sleep(Duration::from_millis(20)).await;
    };
    let line = journal.iter().find(|e| e.kind == Kind::Webhook).unwrap();
    assert!(
        line.text.contains("204") && line.text.contains("ticket.created"),
        "{}",
        line.text
    );
    assert!(journal.iter().all(|e| !e.text.contains("/hook")));
    // A secret address to a host it did not declare is not called.
    h.core
        .secrets
        .set(
            &secret_name("dev.test.discord", "webhook"),
            "http://localhost:9/x",
        )
        .unwrap();
    h.core
        .ticket_create(&p.id, draft("Deux", &[], 5))
        .await
        .unwrap();
    let started = Instant::now();
    while !h
        .core
        .plugin_runtime
        .journal("dev.test.discord")
        .iter()
        .any(|e| e.kind == Kind::Refused && e.text.contains("localhost"))
    {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "{:#?}",
            h.core.plugin_runtime.journal("dev.test.discord")
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    // `events:ticket.*` taken back (Réglages › Plugins): those facts no longer reach it.
    h.core
        .secrets
        .set(&secret_name("dev.test.discord", "webhook"), &url)
        .unwrap();
    let take_back = |back: bool| {
        h.core
            .plugins
            .choose("dev.test.discord", |c| {
                c.revoked.retain(|w| w != "events:ticket.*");
                if back {
                    c.revoked.push("events:ticket.*".into());
                }
            })
            .unwrap()
    };
    assert!(take_back(true));
    h.core
        .ticket_create(&p.id, draft("Sans permission", &[], 5))
        .await
        .unwrap();
    // Handed out (or not) by the bus before the permission comes back.
    h.core.facts.flush();
    assert!(take_back(false));
    h.core
        .ticket_create(&p.id, draft("Rendue", &[], 5))
        .await
        .unwrap();
    // Its queue keeps the order: the next call is the one given back, none came before it.
    let got = tokio::time::timeout(Duration::from_secs(20), bodies.recv())
        .await
        .expect("the webhook in time")
        .unwrap();
    assert!(
        got.contains("Rendue") && !got.contains("Sans permission"),
        "{got}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_webhook_whose_host_a_fact_makes_goes_only_to_a_host_of_permissions_network() {
    let h = harness("facts-webhook-host");
    let (url, mut bodies) = fake_server().await;
    let (p, _) = h.project(false).await;
    h.set_board(&p.id, |s| s.autopilot = false);
    // Its address is the ticket's title: whoever writes a title would choose where the facts go.
    let dir = plugin_dir(
        &h.dir,
        "titled",
        json!({
            "id": "dev.test.titled",
            "permissions": { "events": ["ticket.created"], "network": ["127.0.0.1"] },
            "contributes": { "webhooks": [{ "url": "http://{{ticket.title}}", "events": ["ticket.created"] }] },
        }),
    );
    h.core.plugins.load_dev(&dir, Lang::En).unwrap();
    for title in ["evil.dev/x", "a@evil.dev", "127.0.0.1.evil.dev/x"] {
        h.core
            .ticket_create(&p.id, draft(title, &[], 5))
            .await
            .unwrap();
    }
    // A title that names a host it declared goes there, after the others in its queue.
    let good = url.trim_start_matches("http://").to_string();
    h.core
        .ticket_create(&p.id, draft(&good, &[], 5))
        .await
        .unwrap();
    let got = tokio::time::timeout(Duration::from_secs(20), bodies.recv())
        .await
        .expect("the declared host in time")
        .unwrap();
    assert!(got.starts_with("POST /hook "), "{got}");
    let refused: Vec<String> = h
        .core
        .plugin_runtime
        .journal("dev.test.titled")
        .into_iter()
        .filter(|e| e.kind == Kind::Refused)
        .map(|e| e.text)
        .collect();
    assert_eq!(refused.len(), 3, "{refused:#?}");
    for host in ["evil.dev", "evil.dev", "127.0.0.1.evil.dev"] {
        assert!(
            refused
                .iter()
                .any(|t| t.contains(&format!("to {host} is not sent"))),
            "{refused:#?}"
        );
    }
    assert!(
        bodies.try_recv().is_err(),
        "a webhook went to an undeclared host"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_command_runs_on_a_fact_only_once_agreed_to_as_it_is() {
    let h = harness("facts-command");
    #[cfg(windows)]
    crate::paths::trim_test_path();
    let (p, _) = h.project(false).await;
    h.set_board(&p.id, |s| s.autopilot = false);
    let out = h.dir.join("said.txt");
    let script = h.dir.join("say.mjs");
    std::fs::write(
        &script,
        format!(
            "import fs from 'node:fs';\nfs.appendFileSync({:?}, `${{process.env.ESCOUADE_FACT_NAME}} ${{process.env.ESCOUADE_TICKET}} ${{JSON.parse(process.env.ESCOUADE_FACT).ticket.title}}\\n`);\n",
            out.to_string_lossy()
        ),
    )
    .unwrap();
    let dir = plugin_dir(
        &h.dir,
        "say",
        json!({
            "id": "dev.test.say",
            "permissions": { "events": ["ticket.created"] },
            "contributes": { "commands": [{
                "id": "say",
                "events": ["ticket.created"],
                "run": format!("node \"{}\"", script.to_string_lossy().replace('\\', "/")),
            }] },
        }),
    );
    let view = h.core.plugins.load_dev(&dir, Lang::En).unwrap();
    assert!(!view.commands_approved);
    // Not agreed to: nothing runs, and the journal says why.
    h.core
        .ticket_create(&p.id, draft("Avant", &[], 5))
        .await
        .unwrap();
    let journal = || h.core.plugin_runtime.journal("dev.test.say");
    let started = Instant::now();
    while !journal().iter().any(|e| e.kind == Kind::Refused) {
        assert!(started.elapsed() < Duration::from_secs(10));
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(!out.exists());
    // Agreed to: it runs, the fact in its variables (a title runs nothing).
    assert!(h
        .core
        .plugins
        .approve_commands("dev.test.say", true)
        .unwrap());
    assert!(
        h.core
            .plugins
            .get("dev.test.say", Lang::En)
            .unwrap()
            .commands_approved
    );
    h.core
        .ticket_create(&p.id, draft("Après $(echo non) & `x`", &[], 5))
        .await
        .unwrap();
    let started = Instant::now();
    while !journal().iter().any(|e| e.kind == Kind::Command) {
        assert!(
            started.elapsed() < Duration::from_secs(60),
            "{:#?}",
            journal()
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let line = journal()
        .into_iter()
        .find(|e| e.kind == Kind::Command)
        .unwrap();
    assert!(line.text.contains("exit 0"), "{}", line.text);
    assert_eq!(
        std::fs::read_to_string(&out).unwrap(),
        "ticket.created DEM-2 Après $(echo non) & `x`\n"
    );
    // The agreement taken back (Réglages › Plugins): it runs no more, and says why.
    assert!(h
        .core
        .plugins
        .approve_commands("dev.test.say", false)
        .unwrap());
    assert!(
        !h.core
            .plugins
            .get("dev.test.say", Lang::En)
            .unwrap()
            .commands_approved
    );
    let refused = || journal().iter().filter(|e| e.kind == Kind::Refused).count();
    let before = refused();
    h.core
        .ticket_create(&p.id, draft("Repris", &[], 5))
        .await
        .unwrap();
    let started = Instant::now();
    while refused() == before {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "{:#?}",
            journal()
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(
        std::fs::read_to_string(&out).unwrap(),
        "ticket.created DEM-2 Après $(echo non) & `x`\n"
    );
    // Another command is another agreement.
    let mut m: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("escouade-plugin.json")).unwrap())
            .unwrap();
    m["contributes"]["commands"][0]["run"] = json!("node -e 1");
    std::fs::write(dir.join("escouade-plugin.json"), m.to_string()).unwrap();
    let view = h.core.plugins.load_dev(&dir, Lang::En).unwrap();
    assert!(!view.commands_approved);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_full_queue_drops_its_oldest_facts_and_says_it_in_the_journal() {
    #[cfg(windows)]
    crate::paths::trim_test_path();
    let base = std::env::temp_dir().join(format!("facts-queue-{}", uuid::Uuid::new_v4().simple()));
    let mut m = keeper(FAKE, &["ticket.rejected"]);
    m["run"]["args"] = json!(["--keep-facts", "--deaf"]);
    let dir = plugin_dir(&base, "deaf", m);
    let data = DataDir::new(base.join("data"));
    let plugins = Plugins::load(data.clone(), "2.1.1");
    plugins.load_dev(&dir, Lang::En).unwrap();
    let store: Arc<dyn SecretStore> = Arc::new(MemoryStore::default());
    let rt = Runtime::new(
        plugins.clone(),
        store.clone(),
        &data,
        "2.1.1",
        Limits {
            initialize: Duration::from_secs(20),
            shutdown: Duration::from_millis(300),
            ..Limits::default()
        },
    );
    let dispatch = Dispatch::new(plugins, rt.clone(), store, 4);
    let bus = Bus::new();
    bus.subscribe(dispatch);
    // Started by the first fact: while it starts, the facts wait in the queue of its start (PLG-2);
    // once it runs, they go to its input, which it no longer reads, then fill its queue of facts.
    let comment = "x".repeat(4000);
    let full = || {
        rt.journal(FAKE)
            .iter()
            .any(|e| e.kind == Kind::Dropped && e.text.contains("queue of facts is full (4)"))
    };
    // One at a time, at the pace of its queue: it fills only once its input is full.
    let mut sent = 0;
    while !full() {
        assert!(sent < 20_000, "never full: {:#?}", rt.journal(FAKE));
        bus.emit(
            Body::TicketRejected {
                comment: comment.clone(),
            },
            About::default(),
        );
        sent += 1;
        bus.flush();
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    // It ran, and stopped reading: what filled the queue is its input, not its start.
    assert_eq!(
        rt.status(FAKE).phase,
        PhaseName::Running,
        "{:#?}",
        rt.journal(FAKE)
    );
    assert!(
        !rt.journal(FAKE)
            .iter()
            .any(|e| e.kind == Kind::Dropped && e.text.contains("while it starts")),
        "{:#?}",
        rt.journal(FAKE)
    );
    let dropped = rt
        .journal(FAKE)
        .into_iter()
        .find(|e| e.kind == Kind::Dropped)
        .unwrap();
    // The oldest goes first: a low number, while the newest are kept.
    assert!(
        dropped.text.contains("ticket.rejected #"),
        "{}",
        dropped.text
    );
    rt.shutdown();
}

#[test]
fn every_fact_name_is_tested_here() {
    let text = include_str!("facts_tests.rs");
    for name in NAMES {
        assert!(
            text.contains(&format!("\"{name}\"")),
            "{name} is not tested"
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_tickets_docs_poured_on_the_docs_branch_are_a_fact() {
    let h = harness("facts-docs-filed");
    let l = listen(&h);
    let (p, _) = h.project(false).await;
    let p = crate::docs_tests::set_docs_branch(&h, &p.id, "docs");
    let (t, _) = reviewed(&h, &p.id, "Fichier [ok]").await;
    let agent = t.agent_id.clone().unwrap();
    let copy = h.core.docs_space(&p.id, Some(&agent)).await.unwrap();
    copy.spec_write("paiement/echeances", |p| {
        p.body = "Quatre échéances.".into()
    })
    .unwrap();
    h.core.ticket_approve(&t.id).await.unwrap();
    assert_eq!(h.ticket(&t.id).column, Column::Done);
    let facts = l.heard(&h, &["docs.filed"]).await;
    let filed = of(&facts, "docs.filed")[0];
    assert_eq!(filed["data"], json!({ "pages": 1 }));
    assert_eq!(filed["ticket"]["key"], t.key.as_str());
}
