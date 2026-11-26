//! The gate of `escouade-server` in the core (RP-1): an agent's processes get it in place of the space's proxy with a
//! token of their own, `GIT_SSH_COMMAND` and the repository's `core.sshCommand` through `escouade-server via`; the
//! project's terminals, launches and the core's `git` get it with a token of the project; without the gate (the
//! desktop), nothing changes.

use crate::core_tests::{git, harness};
use crate::privatenet::Kind;
use escouade_gate::NoConnectors;
use std::sync::Arc;

/// A turn of the agent `id`; its notices said when it does not start.
async fn say(h: &crate::core_tests::Harness, id: &str) {
    if let Err(e) = h.core.send_message(id, "Bonjour".into(), vec![]).await {
        let notices: Vec<_> = h
            .items(id)
            .into_iter()
            .filter(|i| i["kind"] == "notice")
            .collect();
        panic!("{e:#}: {notices:#?}");
    }
    h.wait("turn end", |h| {
        h.items(id).iter().any(|i| i["kind"] == "turn") && !h.agent(id).status.is_active()
    })
    .await;
}

fn get<'a>(env: &'a [(String, String)], k: &str) -> Option<&'a str> {
    env.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str())
}

/// The token of a proxy's address `http://<token>@127.0.0.1:P`.
fn token_of(url: &str) -> String {
    url.trim_start_matches("http://")
        .trim_start_matches("socks5h://")
        .split('@')
        .next()
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn an_agent_on_a_server_with_the_gate_goes_out_by_it_with_a_token_of_its_own() {
    let h = harness("gate-agent");
    let (p, r) = h.project(false).await;
    // A first agent, as the network's own test starts one before it sets the proxy.
    let first = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    say(&h, &first).await;
    h.core.settings.write().proxy_url = "http://proxy.corp:3128".into();
    // The desktop, or a server without the gate: the proxy set, as before.
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    say(&h, &a).await;
    let before = h.launch_log(&r).last().cloned().unwrap();
    assert_eq!(before["proxy"], "http://proxy.corp:3128");
    assert!(before["gitSsh"].is_null());
    let gate = h.core.start_gate(Arc::new(NoConnectors)).await.unwrap();
    let port = gate.addr().port();
    let b = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    say(&h, &b).await;
    let seen = h.launch_log(&r).last().cloned().unwrap();
    let proxy = seen["proxy"].as_str().unwrap().to_string();
    assert!(
        proxy.starts_with("http://") && proxy.ends_with(&format!("@127.0.0.1:{port}")),
        "{proxy}"
    );
    assert!(!proxy.contains("proxy.corp"));
    let token = token_of(&proxy);
    assert_eq!(
        seen["allProxy"].as_str().unwrap(),
        format!("socks5h://{token}@127.0.0.1:{port}")
    );
    assert_eq!(seen["noProxy"], "127.0.0.1,localhost");
    let ssh = seen["gitSsh"].as_str().unwrap();
    assert!(
        ssh.starts_with("ssh -o ProxyCommand=") && ssh.ends_with(" via %h %p\""),
        "{ssh}"
    );
    // The token says the agent and its project.
    let holder = gate.holder(&token).unwrap();
    assert_eq!(
        (holder.who.as_str(), holder.project.as_str()),
        (b.as_str(), p.id.as_str())
    );
    assert_eq!(holder.project_name, "demo");
    // The repository's ssh command goes through the gate, for a git run without its variables.
    let written = git(&r, &["config", "--get", "core.sshCommand"]);
    assert!(escouade_gate::env::is_ours(&written), "{written}");
    // Its token is its own, and worth nothing once it is gone.
    let mut other = Vec::new();
    h.core
        .gate_agent_env(&a, &p.id, &r.to_string_lossy(), &mut other);
    assert_ne!(token_of(get(&other, "HTTPS_PROXY").unwrap()), token);
    h.core.delete_agent(&b, false).await.unwrap();
    assert!(gate.holder(&token).is_none());
    // No token ever goes to the log of the agent's conversation, the journal nor the state.
    let state = std::fs::read_to_string(h.core.data.root().join("state.json")).unwrap_or_default();
    assert!(!state.contains(&token));
}

#[tokio::test]
async fn the_project_s_terminals_launches_and_git_get_the_gate_and_the_desktop_keeps_its_proxy() {
    let h = harness("gate-project");
    let (p, r) = h.project(false).await;
    let dir = r.to_string_lossy().into_owned();
    h.core.settings.write().proxy_url = "http://proxy.corp:3128".into();
    let set = h.core.settings.read().proxy_env();
    // Without the gate: nothing changes.
    let mut env = set.clone();
    assert!(!h
        .core
        .gate_project_env(Kind::Terminal, &p.id, &dir, &mut env));
    assert_eq!(env, set);
    assert!(h.core.gate_git_env(&dir).is_empty());
    let gate = h.core.start_gate(Arc::new(NoConnectors)).await.unwrap();
    // A terminal, even with « le proxy aussi pour les terminaux » off (nothing set before).
    let mut terminal = Vec::new();
    assert!(h
        .core
        .gate_project_env(Kind::Terminal, &p.id, &dir, &mut terminal));
    let t = token_of(get(&terminal, "HTTPS_PROXY").unwrap());
    assert_eq!(gate.holder(&t).unwrap().who, "terminal");
    assert!(get(&terminal, "GIT_SSH_COMMAND")
        .unwrap()
        .contains(" via %h %p"));
    // A launch, its proxy set replaced.
    let mut launch = set.clone();
    assert!(h
        .core
        .gate_project_env(Kind::Launch, &p.id, &dir, &mut launch));
    assert!(launch.iter().all(|(_, v)| !v.contains("proxy.corp")));
    let l = token_of(get(&launch, "http_proxy").unwrap());
    assert_eq!(gate.holder(&l).unwrap().who, "launch");
    // The core's git over the network, in the project and in a folder under it.
    let git_env = h.core.gate_git_env(&r.join("src").to_string_lossy());
    let g = token_of(get(&git_env, "HTTPS_PROXY").unwrap());
    let holder = gate.holder(&g).unwrap();
    assert_eq!(
        (holder.who.as_str(), holder.project.as_str()),
        ("git", p.id.as_str())
    );
    // Out of any project: none.
    assert!(h
        .core
        .gate_git_env(&std::env::temp_dir().to_string_lossy())
        .is_empty());
    // The space's proxy changed: the gate follows (a token given stays good).
    h.core.settings.write().proxy_url = String::new();
    h.core.gate.follow(&h.core.settings.read());
    assert!(gate.holder(&g).is_some());
    h.core.gate.stop();
    assert!(!h.core.gate.is_on());
    let mut after = set.clone();
    assert!(!h
        .core
        .gate_project_env(Kind::Terminal, &p.id, &dir, &mut after));
}

#[tokio::test]
async fn the_lists_are_read_again_when_their_file_changes() {
    let h = harness("gate-lists");
    let (p, _) = h.project(false).await;
    let gate = h.core.start_gate(Arc::new(NoConnectors)).await.unwrap();
    assert!(!h.core.gate.reload_if_changed());
    let file = h.core.data.root().join(escouade_gate::lists::FILE);
    std::fs::write(
        &file,
        format!(
            r#"{{"projects":{{"{}":{{"hosts":[{{"host":"gitlab.acme.internal","port":22,"connector":"acme"}}]}}}}}}"#,
            p.id
        ),
    )
    .unwrap();
    assert!(h.core.gate.reload_if_changed());
    // Its connector is not paired (no connector before RP-4): refused, and said so, never out.
    let holder = escouade_gate::Holder {
        who: "a".into(),
        project: p.id.clone(),
        project_name: "demo".into(),
    };
    let lists = escouade_gate::Lists::read(&file).unwrap();
    let t = escouade_gate::Target::parse("gitlab.acme.internal:22").unwrap();
    assert!(matches!(
        escouade_gate::decide(&lists, &[], &holder, &t),
        escouade_gate::Decision::Refused(escouade_gate::Reason::UnknownConnector { .. })
    ));
    drop(gate);
}

#[test]
fn escouade_server_via_and_gate_are_read_from_the_command_line() {
    use crate::server;
    assert_eq!(server::main(vec!["via".into()]), 2);
    assert_eq!(server::main(vec!["via".into(), "h".into(), "x".into()]), 2);
    let on = server::parse_with(&["--gate".to_string()], |_| None).unwrap();
    assert!(on.gate);
    let by_var = server::parse_with(&[], |k| (k == server::GATE_VAR).then(|| "1".into())).unwrap();
    assert!(by_var.gate);
    let off = server::parse_with(&[], |_| None).unwrap();
    assert!(!off.gate);
    assert!(server::help().contains("via"));
}
