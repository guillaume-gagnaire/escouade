//! The connectors' entry in the core (RP-4): the policy `servers.privateNetwork` at `false` opens neither the entry nor
//! the gate and refuses the pairing, then closes an entry open and cuts its channels; the keys in `secrets.json`, never
//! a code nor a key in `connectors.json`; a project's list within its connector's ceiling; the commands served to the
//! space. Against a fake connector made of the protocol's crate, on 127.0.0.1.

use crate::core_tests::harness;
use crate::privateentry::Asked;
use crate::server::secrets::FileStore;
use escouade_gate::entry::proto::{self, pairing, Ceiling, Keypair};
use escouade_gate::entry::State;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;

fn denied() -> crate::locks::Managed {
    crate::locks::Managed {
        values: [(
            escouade_policy::list::PRIVATE_NETWORK.to_string(),
            (Value::Bool(false), "test".to_string()),
        )]
        .into(),
        warnings: Vec::new(),
    }
}

fn asked() -> Asked {
    Asked {
        listen: Some("127.0.0.1:0".parse().unwrap()),
        ..Asked::default()
    }
}

async fn until(what: &str, mut ok: impl FnMut() -> bool) {
    for _ in 0..1000 {
        if ok() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("never: {what}");
}

async fn ws(
    addr: std::net::SocketAddr,
    path: &str,
) -> tokio_tungstenite::WebSocketStream<tokio::net::TcpStream> {
    let tcp = tokio::net::TcpStream::connect(addr).await.unwrap();
    tokio_tungstenite::client_async_with_config(
        format!("ws://{addr}{path}"),
        tcp,
        Some(proto::transport::ws_config()),
    )
    .await
    .unwrap()
    .0
}

/// A fake connector paired by `code` on the entry at `addr`, then connected with its ceiling.
async fn fake_connector(
    addr: std::net::SocketAddr,
    code: &str,
    rules: &[&str],
) -> proto::ConnectorChannel {
    let keys = Keypair::generate();
    let v = crate::privateentry::server_version();
    let paired = pairing::pair_server(
        ws(addr, "/connector/pair").await,
        &keys,
        code,
        v,
        Duration::from_secs(10),
    )
    .await
    .unwrap();
    let ceiling = Ceiling {
        rules: rules.iter().map(|r| r.to_string()).collect(),
    };
    proto::connect_server(
        ws(addr, "/connector").await,
        &keys,
        &paired.server,
        v,
        &ceiling,
        proto::Limits::default(),
    )
    .await
    .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn the_policy_at_false_opens_neither_the_entry_nor_the_gate_and_refuses_the_pairing() {
    let h = harness("privnet-policy");
    h.core.set_managed(denied());
    let store = Arc::new(FileStore::open(&h.core.data).unwrap());
    h.core
        .start_private_network(asked(), store.clone())
        .await
        .unwrap();
    assert!(!h.core.private_network.is_open(), "no entry");
    assert!(!h.core.gate.is_on(), "no gate");
    let err = h.core.pair_connector("Acme").unwrap_err().to_string();
    assert!(err.contains("servers.privateNetwork"), "{err}");
    let status = h.core.private_network_status();
    assert!(status.denied.unwrap().contains("servers.privateNetwork"));
    assert!(status.entry.is_none());

    // Lifted: the entry and the gate open.
    h.core.set_managed(crate::locks::Managed::default());
    until("open", || h.core.private_network.is_open()).await;
    assert!(h.core.gate.is_on());
    let addr = h.core.private_network.addr().unwrap();
    let (code, entry) = h.core.pair_connector("Acme").unwrap();
    assert!(entry.unwrap().ends_with("/connector"));
    let ch = fake_connector(addr, &code.text, &["gitlab.acme.internal:22"]).await;
    let registry = h.core.private_network.registry().unwrap();
    until("connected", || {
        registry
            .views()
            .first()
            .is_some_and(|v| v.state == State::Connected)
    })
    .await;

    // Denied again while it runs: the entry closed, its channel cut, the pairing refused.
    h.core.set_managed(denied());
    tokio::time::timeout(Duration::from_secs(10), ch.closed())
        .await
        .expect("its channel cut");
    until("closed", || !h.core.private_network.is_open()).await;
    let mut refused = false;
    for _ in 0..100 {
        if tokio::net::TcpStream::connect(addr).await.is_err() {
            refused = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(refused, "its port closed");
    assert!(h.core.pair_connector("Other").is_err());
    h.core.private_network.close();
    h.core.gate.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_keys_are_in_secrets_json_and_connectors_json_holds_neither_a_key_nor_a_code() {
    let h = harness("privnet-secrets");
    let store = Arc::new(FileStore::open(&h.core.data).unwrap());
    h.core
        .start_private_network(asked(), store.clone())
        .await
        .unwrap();
    let addr = h.core.private_network.addr().unwrap();
    let (code, _) = h.core.pair_connector("Acme").unwrap();
    let secrets = std::fs::read_to_string(h.core.data.root().join("secrets.json")).unwrap();
    assert!(secrets.contains("connector-pairing"));
    assert!(!secrets.contains(&code.text) && !secrets.contains(&code.text.replace('-', "")));
    let _ch = fake_connector(addr, &code.text, &["gitlab.acme.internal:22"]).await;
    let registry = h.core.private_network.registry().unwrap();
    until("connected", || {
        registry
            .views()
            .first()
            .is_some_and(|v| v.state == State::Connected)
    })
    .await;
    let id = registry.views()[0].id.clone();
    let secrets: Value =
        serde_json::from_slice(&std::fs::read(h.core.data.root().join("secrets.json")).unwrap())
            .unwrap();
    assert!(secrets[format!("connector-key-{id}")].is_string());
    assert!(secrets["connector-server-key"].is_string());
    assert!(
        secrets.get("connector-pairing").is_none(),
        "the code used, gone"
    );
    // Written just after the channel is held: waited for.
    let file = h.core.data.root().join("connectors.json");
    until("written", || {
        std::fs::read_to_string(&file).is_ok_and(|t| t.contains("connectedSince"))
    })
    .await;
    let listed = std::fs::read_to_string(&file).unwrap();
    let server_private = secrets["connector-server-key"].as_str().unwrap();
    let connector_public = secrets[format!("connector-key-{id}")].as_str().unwrap();
    assert!(!listed.contains(server_private) && !listed.contains(connector_public));
    assert!(
        listed.contains("Acme") && listed.contains("connectedSince"),
        "{listed}"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for f in ["secrets.json", "connectors.json"] {
            let mode = std::fs::metadata(h.core.data.root().join(f))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o077, 0, "{f} is its owner's alone");
        }
    }
    // The command beside the server sees it connected, then removes it: the server closes its channel.
    let beside = crate::privateentry::registry_of(&h.core.data, store);
    assert_eq!(beside.views()[0].state, State::Connected);
    beside
        .remove("Acme", "escouade-server", 1)
        .unwrap()
        .unwrap();
    until("cut", || !registry.is_connected(&id)).await;
    h.core.private_network.close();
    h.core.gate.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_project_s_list_is_written_within_its_connector_s_ceiling_and_the_gate_reads_it() {
    let h = harness("privnet-lists");
    let (p, _) = h.project(false).await;
    let store = Arc::new(FileStore::open(&h.core.data).unwrap());
    h.core
        .start_private_network(asked(), store.clone())
        .await
        .unwrap();
    let addr = h.core.private_network.addr().unwrap();
    let (code, _) = h.core.pair_connector("Acme").unwrap();
    let _ch = fake_connector(
        addr,
        &code.text,
        &["gitlab.acme.internal:22", "*.npm.acme.internal:443"],
    )
    .await;
    let registry = h.core.private_network.registry().unwrap();
    until("its ceiling", || {
        registry
            .views()
            .first()
            .is_some_and(|v| v.ceiling.len() == 2)
    })
    .await;
    let id = registry.views()[0].id.clone();
    let allowed = |host: &str, port: u16, connector: &str| escouade_gate::Allowed {
        host: host.into(),
        port,
        connector: connector.into(),
    };
    let list = h
        .core
        .set_project_hosts(
            &p.id,
            vec![
                allowed("GitLab.Acme.Internal.", 22, &id),
                allowed("registry.npm.acme.internal", 443, &id),
            ],
        )
        .unwrap();
    assert_eq!(list.hosts[0].host, "gitlab.acme.internal");
    assert_eq!(h.core.project_hosts(&p.id).unwrap(), list);
    // Out of its ceiling: refused, said; the server cannot widen it.
    let err = h
        .core
        .set_project_hosts(&p.id, vec![allowed("gitlab.acme.internal", 443, &id)])
        .unwrap_err()
        .to_string();
    assert!(err.contains("plafond"), "{err}");
    let err = h
        .core
        .set_project_hosts(&p.id, vec![allowed("gitlab.acme.internal", 22, "cnone")])
        .unwrap_err()
        .to_string();
    assert!(err.contains("cnone"), "{err}");
    assert!(h.core.set_project_hosts("no-project", vec![]).is_err());
    // The list kept as it was; emptied, the project reaches nothing of the private network.
    assert_eq!(h.core.project_hosts(&p.id).unwrap().hosts.len(), 2);
    h.core.set_project_hosts(&p.id, vec![]).unwrap();
    assert!(h.core.project_hosts(&p.id).unwrap().hosts.is_empty());
    let status = h.core.private_network_status();
    assert!(status.served && status.gate && status.entry.is_some());
    assert_eq!(status.connectors.len(), 1);
    h.core.private_network.close();
    h.core.gate.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_core_that_serves_no_private_network_refuses_the_pairing_and_shows_nothing() {
    let h = harness("privnet-none");
    let err = h.core.pair_connector("Acme").unwrap_err().to_string();
    assert!(err.contains("--connector-listen"), "{err}");
    let status = h.core.private_network_status();
    assert!(!status.served && !status.gate);
    assert!(status.connectors.is_empty() && status.entry.is_none());
    assert!(h.core.gate_journal().is_empty());
}
