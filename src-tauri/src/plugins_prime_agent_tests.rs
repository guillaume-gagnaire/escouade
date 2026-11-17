//! Prime Agent as the manifest of its plugin describes it (ESC-485, plugins/prime-agent/): a harness described over
//! ACP whose one mode is Bypass, on a fake that speaks as Prime Agent 0.10.3 did in the study's sandbox
//! (tests/fixtures/fake-prime-agent.mjs), and the streams recorded there (tests/fixtures/prime-agent/). A child of
//! `plugins_acp_tests`, whose bench it drives.

use super::*;

/// Prime Agent as the manifest of its plugin declares it.
fn prime_decl() -> Value {
    let v: Value = serde_json::from_str(include_str!(
        "../../plugins/prime-agent/escouade-plugin.json"
    ))
    .unwrap();
    v["contributes"]["harnesses"][0].clone()
}

/// The fake Prime Agent, by the name its manifest finds: its `.cmd` under Windows.
fn fake_prime() -> Program {
    let name = if cfg!(windows) {
        "prime-agent.cmd"
    } else {
        "prime-agent"
    };
    Program::Native(fixtures().join("prime-agent-bin").join(name))
}

/// The harness of `dev.escouade.prime-agent`, from its manifest alone.
fn prime(name: &str) -> (Arc<Runtime>, &'static ProcessEngine) {
    let (rt, _, _) = runtime(name);
    let engine = described(&rt, "dev.escouade.prime-agent", prime_decl());
    (rt, engine)
}

/// An agent of Prime Agent in `dir`, in the mode it has, on the session `session`; its daemon's pid in `dir/daemon.pid`.
fn prime_agent(
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
    let log = dir.join("fake-prime-agent.jsonl");
    let env = vec![
        (
            "FAKE_PRIME_AGENT_LOG".into(),
            log.to_string_lossy().into_owned(),
        ),
        (
            "FAKE_PRIME_AGENT_DAEMON".into(),
            dir.join("daemon.pid").to_string_lossy().into_owned(),
        ),
    ];
    let mut spawn = Spawn::of(&meta, fake_prime(), env);
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

/// What the client told the fake, by method.
fn told(a: &Driven) -> Vec<Value> {
    a.logged()
        .into_iter()
        .filter(|e| e["dir"] == "c2s")
        .map(|e| e["msg"].clone())
        .collect()
}

/// Whether the process `pid` runs.
fn alive(pid: u32) -> bool {
    #[cfg(windows)]
    {
        let out = std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).contains(&pid.to_string())
    }
    #[cfg(unix)]
    {
        // SAFETY: signal 0 only checks that the process exists.
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
}

/// Prime Agent as its manifest describes it, on a fake that speaks as 0.10.3: launched `--mode acp --no-session`, it
/// answers; its one tool shows as Python with its code and its output; a turn under way stops; no mode is set (it has
/// none), nor a model (its default is its own); it was never asked a permission nor a file; and its session is not
/// taken up by a new process (`loadSession: false`): the agent starts again on a new one.
#[tokio::test(flavor = "multi_thread")]
async fn prime_agent_answers_runs_its_cell_stops_and_starts_a_new_session_after_a_restart() {
    let (rt, engine) = prime("prime-agent");
    assert_eq!(engine.id().as_str(), "prime-agent");
    let dir = bench_dir("esc485-prime-run");
    let mut a = prime_agent(engine, &dir, None, |_| {});
    a.proc().initialize().await.unwrap();
    let end = a.turn("bonjour").await;
    assert!(
        matches!(&end, TurnEnd::Finished(t) if t.contains("Prime Agent a reçu : bonjour")),
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
    // A turn under way stops.
    let n = a.ends.len();
    a.send("long");
    a.until("the reply under way", |d| {
        d.rt.conv.items().iter().any(|i| {
            i["kind"] == "text"
                && i["text"]
                    .as_str()
                    .is_some_and(|t| t.contains("La commande tourne"))
        })
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
    // Asked nothing, set nothing: no mode, no model.
    let said: Vec<String> = told(&a)
        .iter()
        .filter_map(|m| m["method"].as_str().map(str::to_string))
        .collect();
    for never in [
        "session/set_mode",
        "session/set_model",
        "session/set_config_option",
    ] {
        assert!(!said.iter().any(|m| m == never), "{never}: {said:?}");
    }
    // It never asked the client: no request of its own was answered.
    assert!(
        !a.logged().iter().any(|e| e["dir"] == "s2c"
            && e["msg"]["method"].is_string()
            && e["msg"]["id"].is_number()),
        "it asked nothing"
    );
    let launch = a
        .logged()
        .into_iter()
        .find_map(|e| e.get("launch").cloned())
        .unwrap();
    assert_eq!(launch["argv"], json!(["--mode", "acp", "--no-session"]));
    // Started in the agent's folder: Prime Agent works in its process's, whatever `session/new` asks.
    assert_eq!(
        std::fs::canonicalize(launch["cwd"].as_str().unwrap()).unwrap(),
        std::fs::canonicalize(&dir).unwrap()
    );
    let session = a.rt.meta.session_id.clone().expect("its session");

    // Its first message carried the client's instructions (here the protocol of questions in text).
    // The first prompt its last process was told (the log is the folder's, both processes in it).
    let first = |d: &Driven| {
        let logged = d.logged();
        let from = logged
            .iter()
            .rposition(|e| e.get("launch").is_some())
            .unwrap_or(0);
        logged[from..]
            .iter()
            .find(|e| e["dir"] == "c2s" && e["msg"]["method"] == "session/prompt")
            .map(|e| e["msg"]["params"]["prompt"].to_string())
            .unwrap_or_default()
    };
    assert!(first(&a).contains("escouade_question"), "{}", first(&a));
    a.stop().await;

    // A new process does not take its session up: it says it loads none (`loadSession: false`), the client asks none
    // and opens a new one (ESC-491): said, the new session given the instructions again, and what was done.
    let mut b = prime_agent(engine, &dir, Some(session.clone()), |s| {
        s.recap = Some("# Reprise\n\nLe test calc passe.".into())
    });
    b.proc().initialize().await.unwrap();
    let end = b.turn("encore là ?").await;
    assert!(
        matches!(&end, TurnEnd::Finished(t) if t.contains("Prime Agent a reçu : encore là ?")),
        "{end:?}"
    );
    assert!(
        !told(&b).iter().any(|m| m["method"] == "session/load"),
        "it says it loads none"
    );
    assert_ne!(b.rt.meta.session_id.as_deref(), Some(session.as_str()));
    let items = b.rt.conv.items();
    assert!(
        items.iter().any(|i| i["kind"] == "notice"
            && i["text"].as_str().is_some_and(|t| t.starts_with(
                "Prime Agent ne reprend pas sa session : l’agent repart d’une session neuve."
            ) && t.contains("un résumé de ce qu’il a fait"))),
        "said: {items:#?}"
    );
    assert!(
        first(&b).contains("escouade_question") && first(&b).contains("Le test calc passe."),
        "given again: {}",
        first(&b)
    );
    b.stop().await;
    kill_daemon(&dir);
    rt.shutdown();
}

/// Escouade's MCP server goes to its session in `session/new`, the token in a header: never on its command line.
#[tokio::test(flavor = "multi_thread")]
async fn escouade_s_mcp_server_goes_to_prime_agent_s_session_its_token_in_a_header() {
    let (rt, engine) = prime("prime-agent-mcp");
    let dir = bench_dir("esc485-prime-mcp");
    let token = "s3cr3t-esc485-token";
    let mut a = prime_agent(engine, &dir, None, |s| {
        s.escouade = Escouade::Granted {
            url: "http://127.0.0.1:4110/mcp".into(),
            token: token.into(),
            file: None,
        }
    });
    a.proc().initialize().await.unwrap();
    let end = a.turn("serveurs").await;
    assert!(
        matches!(&end, TurnEnd::Finished(t) if t.contains("escouade (http) Authorization")),
        "{end:?}"
    );
    let launch = a
        .logged()
        .into_iter()
        .find_map(|e| e.get("launch").cloned())
        .unwrap();
    assert!(!launch.to_string().contains(token), "{launch}");
    a.stop().await;
    kill_daemon(&dir);
    rt.shutdown();
}

/// No discussion of the Docs tab on Prime Agent: it has no mode that changes nothing, and it does not start.
#[tokio::test(flavor = "multi_thread")]
async fn a_discussion_of_prime_agent_does_not_start() {
    let (rt, engine) = prime("prime-agent-readonly");
    let dir = bench_dir("esc485-prime-readonly");
    let a = prime_agent(engine, &dir, None, |s| s.read_only = true);
    let err = a.proc().initialize().await.unwrap_err().to_string();
    assert!(
        err.contains("Prime Agent n’a pas de mode qui ne change rien"),
        "{err}"
    );
    assert!(
        !dir.join("fake-prime-agent.jsonl").exists(),
        "its program never ran"
    );
    rt.shutdown();
}

/// Nor beside a private project: it reads and writes by itself, the fence would not hold.
#[tokio::test(flavor = "multi_thread")]
async fn an_agent_of_prime_agent_does_not_start_beside_a_private_project() {
    let (rt, engine) = prime("prime-agent-fence");
    let dir = bench_dir("esc485-prime-fence");
    let fence = crate::fence::Fence::new(vec![bench_dir("esc485-prime-fence-other")]);
    let a = prime_agent(engine, &dir, None, |s| s.fence = fence);
    let err = a.proc().initialize().await.unwrap_err().to_string();
    assert!(
        err.contains("la clôture des projets privés ne tiendrait pas"),
        "{err}"
    );
    assert!(!dir.join("fake-prime-agent.jsonl").exists());
    rt.shutdown();
}

/// The pid of the fake's daemon in `dir`, if it started one.
fn daemon_of(dir: &Path) -> Option<u32> {
    std::fs::read_to_string(dir.join("daemon.pid"))
        .ok()
        .and_then(|p| p.trim().parse().ok())
}

/// Stops what the fake's daemon left running (Unix: it outlives the agent).
fn kill_daemon(dir: &Path) {
    if let Some(pid) = daemon_of(dir) {
        #[cfg(unix)]
        // SAFETY: a plain signal to the fake's own daemon.
        unsafe {
            libc::kill(pid as i32, libc::SIGKILL);
        }
        #[cfg(windows)]
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/PID", &pid.to_string()])
            .output();
    }
}

/// Its daemon, as the study found it and the fake plays it (a process `--mode acp` starts detached, once for the user):
/// what Escouade leaves behind once the agent is gone. Kept as it is today, so that a change shows (ESC-485, the
/// finding; the fix is the app's to make, docs/PLUGINS.md and the plugin's Troubleshooting tab say which):
/// - on Unix, it leaves the agent's process group in a session of its own: Escouade's stop does not reach it, it stays;
/// - on Windows, no Job lets it go: it dies with the Job of the agent that started it, and with it the sessions of the
///   other agents that used it.
#[tokio::test(flavor = "multi_thread")]
async fn its_daemon_outlives_its_agent_on_unix_and_dies_with_its_job_on_windows() {
    let (rt, engine) = prime("prime-agent-daemon");
    let dir = bench_dir("esc485-prime-daemon");
    let _ = std::fs::remove_file(dir.join("daemon.pid"));
    let mut a = prime_agent(engine, &dir, None, |_| {});
    a.proc().initialize().await.unwrap();
    a.turn("bonjour").await;
    let daemon = daemon_of(&dir).expect("its daemon started");
    assert!(alive(daemon), "its daemon runs with the agent");
    // A second agent, in another folder, finds it: one daemon for all.
    let other = bench_dir("esc485-prime-daemon-other");
    std::fs::copy(dir.join("daemon.pid"), other.join("daemon.pid")).unwrap();
    let mut b = prime_agent(engine, &other, None, |_| {});
    b.proc().initialize().await.unwrap();
    b.turn("bonjour").await;
    assert_eq!(daemon_of(&other), Some(daemon), "the same daemon");
    b.stop().await;
    // The agent that started it stops, as the core stops it: its input closed, its process and its Job gone.
    let p = a.rt.detach().unwrap();
    p.close_input();
    if !p.wait_exit(Duration::from_secs(10)).await {
        p.kill();
    }
    p.kill();
    drop(p);
    let mut gone = false;
    for _ in 0..50 {
        if !alive(daemon) {
            gone = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    kill_daemon(&dir);
    rt.shutdown();
    if cfg!(windows) {
        assert!(gone, "on Windows the daemon is in the agent's Job");
    } else {
        assert!(!gone, "on Unix the daemon left the agent's group");
    }
}

/// The streams of Prime Agent 0.10.3 recorded in the study's sandbox (tests/fixtures/prime-agent/: a turn of text, a fix
/// in six cells on a local model, two calls of an MCP server from the kernel and a turn cancelled), through the client
/// of its manifest: each cell a tool « Python » with its code and its output, its `session_info_update`s ignored.
#[test]
fn the_recorded_streams_of_prime_agent_make_python_cells_with_their_code() {
    let decl = prime_decl();
    for (case, cells, last) in [
        ("hello", 0, "bonjour"),
        ("full-turn", 6, "calc.py a été corrigé"),
        ("mcp", 6, "encore là"),
    ] {
        let flux =
            std::fs::read_to_string(fixtures().join("prime-agent").join(format!("{case}.jsonl")))
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
        let conv = crate::paths::test_dir(&format!("esc485-replay-{case}"));
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
        // Its notifications of its own say nothing in the conversation.
        assert!(
            !items.iter().any(|i| i["kind"] == "notice"
                && i["text"]
                    .as_str()
                    .is_some_and(|t| t.contains("session_info"))),
            "{case}: {items:#?}"
        );
    }
}

/// The gap ESC-485 showed, closed by ESC-489 (the version its manifest asks for in `escouade.min`): a harness whose one
/// mode is Bypass gets its new agents in « Demander » (`default`), a mode it does not have; the mode judged is the one
/// it runs in, Bypass. Without « Autoriser le mode Bypass » an agent of Prime Agent does not start, and the refusal says
/// why; with it, it runs in Bypass.
#[test]
fn an_agent_of_prime_agent_runs_in_bypass_and_only_when_it_is_allowed() {
    let (_rt, engine) = prime("prime-agent-gap");
    crate::engine::registry::register(engine);
    let modes: Vec<&str> = engine.caps().modes.iter().map(|m| m.id).collect();
    assert_eq!(modes, [crate::bypass::BYPASS]);
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
        err.contains("Prime Agent ne tourne qu’en Bypass")
            && err.contains("Autoriser le mode Bypass"),
        "{err}"
    );
    assert_eq!(
        crate::modes::judge(engine, "default", true, lang).unwrap(),
        crate::bypass::BYPASS
    );
    crate::engine::registry::unregister(engine.id());
}
