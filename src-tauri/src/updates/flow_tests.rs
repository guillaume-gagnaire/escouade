//! The check and the download of the updates, as the real updater plugin does them, against fake
//! servers on the loopback (never the Internet): the cases of the failures that were met, one
//! each. The app is a test app with the plugin registered, whose version (0.1.0) is below the one
//! the fake latest.json announces.
//!
//! The requests a server sees are the plugin's and the probe's (`net::probe`, made again after a
//! failure to learn why): the comments say which is which.

use super::fake_server::{closed_port, hangs_up, Reply, Server};
use super::net::{self, Network};
use super::*;
use crate::model::Settings;
use serde_json::json;
use tauri::test::MockRuntime;

/// The public key of a key pair made for these tests (`tauri signer generate`, no password): the
/// private key is not kept anywhere.
const PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDlBNTU1OUUxRDYzNzRENTcKUldSWFRUZlc0VmxWbWw1ZjRUWWVadEZ1bnZPVkVUODhFcTA0YjBCcmJlT01qSUFjQWpBdWlZeEcK";

/// What the fake installer holds, and its signature (`tauri signer sign --app-version 99.0.0`,
/// the content of the `.sig` file): the plugin checks them, so a download that returns anything else
/// fails.
const INSTALLER: &[u8] = b"escouade-test-installer";
const SIGNATURE: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVSWFRUZlc0VmxWbXJJRzcrRnRJVWQycFB1UFJWTmhxNW94ejNGWHN3UlI5UnNrK21TV2IxaEpSVHN0eUM1djhoak9qT3JCYWhRcUlCTkJUOE9ERW51RldhaWZrTnRnTGdnPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzkyMDgxMTYwCWZpbGU6aW5zdGFsbGVyLmJpbgl2ZXJzaW9uOjk5LjAuMApGcjdMdG9najJ4YlRQSU1KRVAwRkVDb3BJbW9RRlNFdGE1UWx3ZGZoY2NZcXBsWHczU0JUeGpPNm1mT3Q2Q0c5LzRtVUQ0blJlTWQ0R3N3bE45REhBUT09Cg==";

/// The version the signature was made for, and the fake latest.json announces.
const NEW: &str = "99.0.0";

const LATEST: &str = "/latest.json";
const SETUP: &str = "/me/app/releases/download/v99.0.0/app-setup.exe";

/// A latest.json that gives `url` for every platform the app is released for, so that these tests
/// run the same on Windows, macOS and Linux.
fn manifest(version: &str, url: &str) -> serde_json::Value {
    let platform = json!({ "signature": SIGNATURE, "url": url });
    let platforms: serde_json::Map<String, serde_json::Value> = [
        "windows-x86_64",
        "windows-aarch64",
        "darwin-aarch64",
        "darwin-x86_64",
        "linux-x86_64",
        "linux-aarch64",
    ]
    .into_iter()
    .map(|target| (target.to_string(), platform.clone()))
    .collect();
    json!({ "version": version, "notes": "- Test release", "platforms": platforms })
}

fn latest(url: &str) -> Reply {
    Reply::json(200, &manifest(NEW, url).to_string())
}

fn refusal() -> Reply {
    Reply::html(
        403,
        "<html><body><h1>Access denied</h1><p>Blocked by your company.</p></body></html>",
    )
}

/// What a corporate proxy answers: its name in `Server`, a page that blocks.
fn zscaler() -> Reply {
    refusal().with("Server", "Zscaler/6.2")
}

/// A test app with the plugin registered, `endpoints` being where latest.json is.
fn app(endpoints: &[String]) -> tauri::App<MockRuntime> {
    let mut context = tauri::test::mock_context(tauri::test::noop_assets());
    context.config_mut().plugins.0.insert(
        "updater".into(),
        json!({
            "pubkey": PUBKEY,
            "endpoints": endpoints,
            "dangerousInsecureTransportProtocol": true,
        }),
    );
    tauri::test::mock_builder()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .build(context)
        .unwrap()
}

/// 30 s for a test's step: a regression never hangs it.
async fn within<T>(f: impl Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(30), f)
        .await
        .expect("timed out")
}

struct Rig {
    app: tauri::App<MockRuntime>,
    /// Where latest.json and the installer are.
    origin: Server,
    /// Where the plugin and the checks look for latest.json.
    endpoint: String,
    updates: Updates,
}

impl Rig {
    async fn new() -> Self {
        Self::at(|origin| origin.url(LATEST)).await
    }

    /// A rig whose latest.json is looked for at `endpoint` (of the origin): an `https` one is
    /// reached through a proxy by a tunnel (`CONNECT`).
    async fn at(endpoint: impl FnOnce(&Server) -> String) -> Self {
        let origin = Server::start().await;
        origin.script(LATEST, vec![latest(&origin.url(SETUP))]);
        origin.script(SETUP, vec![Reply::bytes(200, INSTALLER)]);
        let endpoint = endpoint(&origin);
        Self {
            app: app(std::slice::from_ref(&endpoint)),
            origin,
            endpoint,
            updates: Updates::default(),
        }
    }

    /// A check through `proxy`, which waits 5 ms and not 3 s before it tries again.
    fn checker(&self, proxy: Option<&Server>) -> Arc<Checker<MockRuntime>> {
        Arc::new(Checker {
            app: self.app.handle().clone(),
            net: Network {
                proxy: proxy.map(|p| format!("http://{}", p.address()).parse().unwrap()),
                ..Network::default()
            },
            beta: false,
            endpoints: vec![self.endpoint.parse().unwrap()],
            pause: Duration::from_millis(5),
        })
    }

    async fn check(&self, checker: &Arc<Checker<MockRuntime>>, manual: bool) -> Checked {
        within(checker.run(&self.updates, manual)).await
    }

    /// A check that finds the update.
    async fn found(&self, checker: &Arc<Checker<MockRuntime>>) -> Found {
        match self.check(checker, false).await {
            Checked::Found { update } => update,
            other => panic!("the update should be found: {other:?}"),
        }
    }

    async fn downloaded(&self, found: &Found, manual: bool) -> Downloaded {
        within(self.updates.downloaded(found.id, manual)).await
    }
}

fn trouble(checked: Checked) -> Trouble {
    match checked {
        Checked::Failed { trouble } => trouble,
        other => panic!("the check should fail: {other:?}"),
    }
}

fn failed(downloaded: Downloaded) -> Trouble {
    match downloaded {
        Downloaded::Failed { trouble } => trouble,
        other => panic!("the download should fail: {other:?}"),
    }
}

// ---------- the check ----------

#[tokio::test]
async fn a_latest_json_that_is_not_there_yet_is_found_a_moment_later_without_a_word() {
    let rig = Rig::new().await;
    // The plugin's request, the probe's, then the plugin's again after the pause.
    rig.origin.script(
        LATEST,
        vec![
            Reply::status(404),
            Reply::status(404),
            latest(&rig.origin.url(SETUP)),
        ],
    );
    let found = rig.found(&rig.checker(None)).await;
    assert_eq!(found.version, NEW);
    assert_eq!(found.notes, "- Test release");
    assert_eq!(rig.origin.hits(LATEST).len(), 3);
}

#[tokio::test]
async fn a_403_at_the_check_that_goes_away_is_found_without_a_word() {
    let rig = Rig::new().await;
    rig.origin.script(
        LATEST,
        vec![
            Reply::status(403),
            Reply::status(403),
            latest(&rig.origin.url(SETUP)),
        ],
    );
    let found = rig.found(&rig.checker(None)).await;
    assert_eq!(found.version, NEW);
    assert_eq!(rig.origin.hits(LATEST).len(), 3);
}

#[tokio::test]
async fn a_check_refused_every_time_fails_quietly_by_itself_and_loudly_by_hand() {
    let rig = Rig::new().await;
    rig.origin.script(LATEST, vec![refusal()]);
    let checker = rig.checker(None);
    let by_itself = trouble(rig.check(&checker, false).await);
    assert!(!by_itself.show);
    assert_eq!(by_itself.page, None);
    // The plugin's request and the probe's, tried again once after the pause, and no more.
    assert_eq!(rig.origin.hits(LATEST).len(), 4);
    let by_hand = trouble(rig.check(&checker, true).await);
    assert!(by_hand.show);
    assert_eq!(rig.origin.hits(LATEST).len(), 8);
    assert!(
        by_hand.message.starts_with("403 Forbidden de 127.0.0.1"),
        "{}",
        by_hand.message
    );
    assert!(by_hand
        .message
        .contains("Access denied Blocked by your company."));
    assert!(
        by_hand
            .message
            .ends_with(" : bloqué par le réseau ou un proxy ?"),
        "{}",
        by_hand.message
    );
}

#[tokio::test]
async fn a_check_that_fails_for_good_is_not_tried_again() {
    let rig = Rig::new().await;
    // 401 is not one of the failures that pass.
    rig.origin.script(LATEST, vec![Reply::status(401)]);
    let t = trouble(rig.check(&rig.checker(None), true).await);
    assert!(
        t.message.starts_with("401 Unauthorized de 127.0.0.1"),
        "{}",
        t.message
    );
    // The plugin's request and the probe's: nothing after.
    assert_eq!(rig.origin.hits(LATEST).len(), 2);
}

#[tokio::test]
async fn the_domain_told_is_the_one_that_answered_after_a_redirection() {
    let rig = Rig::new().await;
    let assets = Server::start().await;
    assets.script("/assets/latest.json", vec![refusal()]);
    rig.origin.script(
        LATEST,
        vec![Reply::redirect(&format!(
            "http://localhost:{}/assets/latest.json",
            assets.port
        ))],
    );
    let t = trouble(rig.check(&rig.checker(None), true).await);
    assert!(
        t.message.starts_with("403 Forbidden de localhost"),
        "{}",
        t.message
    );
    // Followed by the plugin and by the probe, twice each.
    assert_eq!(assets.hits("/assets/latest.json").len(), 4);
}

#[tokio::test]
async fn a_release_that_is_not_newer_or_a_204_is_nothing_new_and_ends_the_failures() {
    let rig = Rig::new().await;
    let checker = rig.checker(None);
    for nothing_new in [
        // The version of the app (0.1.0) is the latest.
        Reply::json(200, &manifest("0.1.0", &rig.origin.url(SETUP)).to_string()),
        Reply::status(204),
    ] {
        rig.origin.script(LATEST, vec![refusal()]);
        assert!(!trouble(rig.check(&checker, false).await).show);
        assert_eq!(rig.updates.check_streak.lock().count, 1);
        rig.origin.script(LATEST, vec![nothing_new]);
        assert_eq!(rig.check(&checker, false).await, Checked::None);
        assert_eq!(rig.updates.check_streak.lock().count, 0);
    }
}

#[tokio::test]
async fn a_check_that_found_nothing_withdraws_the_update_downloaded() {
    let rig = Rig::new().await;
    let found = rig.found(&rig.checker(None)).await;
    assert_eq!(rig.downloaded(&found, false).await, Downloaded::Ready);
    assert!(rig.updates.ready().is_some());
    rig.origin.script(LATEST, vec![Reply::status(204)]);
    assert_eq!(rig.check(&rig.checker(None), false).await, Checked::None);
    assert!(rig.updates.ready().is_none());
}

#[tokio::test]
async fn the_check_reads_the_endpoints_of_the_configuration_and_the_network_of_the_settings() {
    let rig = Rig::new().await;
    // `plugins.updater.endpoints` of the configuration: the origin's. With no proxy in the
    // settings the system's is followed, as the plugin does (not asked here: a test never goes
    // through the proxy of the machine it runs on).
    let checker = Checker::new(rig.app.handle(), &Settings::default()).unwrap();
    assert_eq!(
        checker.endpoints,
        vec![rig.origin.url(LATEST).parse::<Url>().unwrap()]
    );
    assert!(checker.net.proxy.is_none() && checker.net.system_proxy);
    assert_eq!(checker.pause, RETRY_PAUSE);
    // A proxy address that is not one: told, as a failure.
    let bad = Settings {
        proxy_url: "127.0.0.1:9000".into(),
        ..Settings::default()
    };
    let checked = within(check(rig.app.handle(), &bad, &rig.updates, true)).await;
    let t = trouble(checked);
    assert!(t.show);
    assert!(
        t.message.starts_with("adresse du proxy invalide"),
        "{}",
        t.message
    );
    // The proxy of the settings is the one used.
    let proxy = Server::start().await;
    proxy.script(LATEST, vec![latest(&rig.origin.url(SETUP))]);
    let set = Settings {
        proxy_url: format!("http://{}", proxy.address()),
        ..Settings::default()
    };
    let before = rig.origin.seen().len();
    let checked = within(check(rig.app.handle(), &set, &rig.updates, false)).await;
    assert!(matches!(checked, Checked::Found { .. }), "{checked:?}");
    assert_eq!(proxy.hits(LATEST).len(), 1);
    assert_eq!(rig.origin.seen().len(), before);
}

// ---------- the proxy ----------

/// What the proxy says to the user who has to put a password in its address.
fn password_asked(proxy: &Server) -> String {
    format!(
        "le proxy {} demande une authentification (407 Proxy Authentication Required) : mets l'identifiant et le mot de passe dans l'adresse du proxy, dans Réglages › Avancé › Réseau",
        proxy.address()
    )
}

/// The tunnel a proxy is asked for to reach `server` over https: `CONNECT host:port`.
fn tunnel_to(server: &Server) -> String {
    format!("127.0.0.1:{}", server.port)
}

#[tokio::test]
async fn a_proxy_that_refuses_the_check_is_not_gone_around_and_is_told() {
    let rig = Rig::new().await;
    let proxy = Server::start().await;
    proxy.script(LATEST, vec![zscaler()]);
    let t = trouble(rig.check(&rig.checker(Some(&proxy)), true).await);
    assert!(t.show);
    assert!(
        t.message
            .starts_with("403 Forbidden de 127.0.0.1, par le proxy "),
        "{}",
        t.message
    );
    assert!(t.message.contains(&proxy.address()), "{}", t.message);
    assert!(
        t.message.ends_with("(Zscaler/6.2 : « Access denied Blocked by your company. ») : bloqué par le réseau ou un proxy ?"),
        "{}",
        t.message
    );
    // The company's rule is kept: nothing tried without the proxy, nothing said of it.
    assert!(!t.message.contains("sans proxy"), "{}", t.message);
    // The plugin's request and the probe's, again after the pause; none to the origin.
    assert_eq!(proxy.hits(LATEST).len(), 4);
    assert!(rig.origin.seen().is_empty());
}

#[tokio::test]
async fn a_proxy_that_asks_for_a_password_at_the_check_says_where_to_put_it() {
    let rig = Rig::new().await;
    let proxy = Server::start().await;
    proxy.script(LATEST, vec![Reply::status(407)]);
    let t = trouble(rig.check(&rig.checker(Some(&proxy)), true).await);
    assert_eq!(t.message, password_asked(&proxy));
    assert!(rig.origin.seen().is_empty());
}

#[tokio::test]
async fn a_proxy_that_refuses_the_installer_is_not_gone_around_and_the_browser_is_offered() {
    let rig = Rig::new().await;
    // The proxy lets latest.json through (a transparent relay of the origin's) and refuses the
    // installer, as a company's proxy does to what is not a browser.
    let proxy = Server::start().await;
    proxy.script(LATEST, vec![latest(&rig.origin.url(SETUP))]);
    proxy.script(SETUP, vec![zscaler()]);
    let found = rig.found(&rig.checker(Some(&proxy))).await;
    let t = failed(rig.downloaded(&found, true).await);
    assert!(t.show);
    assert_eq!(
        t.message,
        format!(
            "le proxy de ton entreprise (Zscaler/6.2) a refusé l'installateur (403 Forbidden de 127.0.0.1, par le proxy {}). Télécharge-le dans le navigateur : il faudra peut-être confirmer l'avertissement du proxy.",
            proxy.address()
        )
    );
    // The release's page, for the browser.
    assert_eq!(
        t.page.as_deref(),
        Some(
            format!(
                "http://127.0.0.1:{}/me/app/releases/tag/v99.0.0",
                rig.origin.port
            )
            .as_str()
        )
    );
    // The plugin's request and the probe's, again after latest.json was read once more: none to
    // the origin, which no one asked through the proxy.
    assert_eq!(proxy.hits(SETUP).len(), 4);
    assert_eq!(proxy.hits(LATEST).len(), 2);
    assert!(rig.origin.seen().is_empty());
    assert!(rig.updates.ready().is_none());
}

#[tokio::test]
async fn a_proxy_that_asks_for_a_password_at_the_download_does_not_send_to_the_browser() {
    let rig = Rig::new().await;
    let proxy = Server::start().await;
    proxy.script(LATEST, vec![latest(&rig.origin.url(SETUP))]);
    proxy.script(SETUP, vec![Reply::status(407)]);
    let found = rig.found(&rig.checker(Some(&proxy))).await;
    let t = failed(rig.downloaded(&found, true).await);
    assert_eq!(t.message, password_asked(&proxy));
    assert!(rig.origin.seen().is_empty());
}

#[tokio::test]
async fn a_tunnel_the_proxy_turns_down_is_not_gone_around_and_is_told() {
    // The update is looked for in https, through a tunnel.
    let rig = Rig::at(|origin| format!("https://127.0.0.1:{}{LATEST}", origin.port)).await;
    let proxy = Server::start().await;
    proxy.script(&tunnel_to(&rig.origin), vec![Reply::status(403)]);
    let t = trouble(rig.check(&rig.checker(Some(&proxy)), true).await);
    assert_eq!(
        t.message,
        format!(
            "le proxy {} a refusé d'ouvrir la connexion à 127.0.0.1 (tunnel CONNECT refusé) : une règle de ton entreprise le bloque peut-être.",
            proxy.address()
        )
    );
    // The plugin's tunnel and the probe's, again after the pause, and no more.
    assert_eq!(proxy.hits(&tunnel_to(&rig.origin)).len(), 4);
}

#[tokio::test]
async fn a_tunnel_that_asks_for_a_password_says_where_to_put_it() {
    let rig = Rig::at(|origin| format!("https://127.0.0.1:{}{LATEST}", origin.port)).await;
    let proxy = Server::start().await;
    proxy.script(&tunnel_to(&rig.origin), vec![Reply::status(407)]);
    let t = trouble(rig.check(&rig.checker(Some(&proxy)), true).await);
    assert_eq!(t.message, password_asked(&proxy));
    // The plugin's tunnel and the probe's: a password does not come by itself, so no second try.
    assert_eq!(proxy.hits(&tunnel_to(&rig.origin)).len(), 2);
}

#[tokio::test]
async fn an_installer_whose_tunnel_the_proxy_turns_down_is_not_gone_around_and_the_browser_is_offered(
) {
    let rig = Rig::new().await;
    let secure = Server::start().await;
    let proxy = Server::start().await;
    proxy.script(
        LATEST,
        vec![latest(&format!("https://127.0.0.1:{}{SETUP}", secure.port))],
    );
    proxy.script(&tunnel_to(&secure), vec![Reply::status(403)]);
    let found = rig.found(&rig.checker(Some(&proxy))).await;
    let t = failed(rig.downloaded(&found, true).await);
    assert_eq!(
        t.message,
        format!(
            "le proxy {} a refusé d'ouvrir la connexion à 127.0.0.1 (tunnel CONNECT refusé) : une règle de ton entreprise le bloque peut-être. Télécharge l'installateur dans le navigateur.",
            proxy.address()
        )
    );
    // The page is offered all the same, but a link in https on this machine is not the endpoint's
    // origin (http): the browser is sent to the releases of the app.
    assert_eq!(t.page.as_deref(), Some(net::RELEASES_URL));
    // The plugin's tunnel and the probe's, again after latest.json was read once more.
    assert_eq!(proxy.hits(&tunnel_to(&secure)).len(), 4);
    assert!(rig.updates.ready().is_none());
}

#[tokio::test]
async fn a_proxy_that_cannot_be_reached_is_gone_around_for_the_check() {
    let rig = Rig::new().await;
    let checker = Arc::new(Checker {
        net: Network {
            proxy: Some(
                format!("http://127.0.0.1:{}", closed_port())
                    .parse()
                    .unwrap(),
            ),
            ..Network::default()
        },
        ..Arc::into_inner(rig.checker(None)).unwrap()
    });
    let found = rig.found(&checker).await;
    assert_eq!(found.version, NEW);
    assert_eq!(rig.origin.hits(LATEST).len(), 1);
}

#[tokio::test]
async fn a_proxy_that_has_gone_is_gone_around_for_the_download() {
    let rig = Rig::new().await;
    // The proxy relays latest.json, then is shut down, as a proxy of the company is for someone
    // who has left the company's network.
    let proxy = Server::start().await;
    proxy.script(LATEST, vec![latest(&rig.origin.url(SETUP))]);
    proxy.script(SETUP, vec![zscaler()]);
    let found = rig.found(&rig.checker(Some(&proxy))).await;
    assert_eq!(proxy.hits(LATEST).len(), 1);
    proxy.stop().await;
    assert_eq!(rig.downloaded(&found, false).await, Downloaded::Ready);
    // The plugin's request alone went to the origin: the proxy was not there to be asked.
    assert_eq!(rig.origin.hits(SETUP).len(), 1);
    assert_eq!(proxy.hits(SETUP).len(), 0);
    assert_eq!(rig.updates.ready().unwrap().version, NEW);
}

#[tokio::test]
async fn a_proxy_that_hangs_up_on_the_tunnel_and_a_direct_access_that_fails_are_both_told() {
    // Nothing answers over https on this port, neither the proxy's tunnel nor a direct connection.
    let dead_end = hangs_up().await;
    let rig = Rig::at(|_| format!("https://127.0.0.1:{dead_end}{LATEST}")).await;
    let proxy = Server::start().await;
    proxy.script(&format!("127.0.0.1:{dead_end}"), vec![Reply::hang_up()]);
    let t = trouble(rig.check(&rig.checker(Some(&proxy)), true).await);
    assert!(t.show);
    assert!(
        t.message.starts_with(&format!(
            "connexion impossible à 127.0.0.1 par le proxy {} (",
            proxy.address()
        )),
        "{}",
        t.message
    );
    assert!(
        t.message
            .contains(") ; sans proxy : connexion impossible à 127.0.0.1 ("),
        "{}",
        t.message
    );
    // The tunnel asked twice, each time by the plugin and by the probe.
    assert_eq!(proxy.hits(&format!("127.0.0.1:{dead_end}")).len(), 4);
}

#[tokio::test]
async fn an_installer_neither_the_proxy_nor_a_direct_access_gives_has_both_told() {
    let dead_end = hangs_up().await;
    let rig = Rig::new().await;
    let proxy = Server::start().await;
    let tunnel = format!("127.0.0.1:{dead_end}");
    proxy.script(
        LATEST,
        vec![latest(&format!("https://127.0.0.1:{dead_end}{SETUP}"))],
    );
    proxy.script(&tunnel, vec![Reply::hang_up()]);
    let found = rig.found(&rig.checker(Some(&proxy))).await;
    let t = failed(rig.downloaded(&found, true).await);
    assert!(t.show);
    assert!(
        t.message.starts_with(&format!(
            "connexion impossible à 127.0.0.1 par le proxy {} (",
            proxy.address()
        )),
        "{}",
        t.message
    );
    assert!(
        t.message
            .contains(") ; sans proxy : connexion impossible à 127.0.0.1 ("),
        "{}",
        t.message
    );
    assert!(rig.updates.ready().is_none());
}

// ---------- the download ----------

#[tokio::test]
async fn a_download_refused_once_reads_latest_json_again_and_goes_through() {
    let rig = Rig::new().await;
    let found = rig.found(&rig.checker(None)).await;
    // The plugin's request and the probe's are refused; the link read again is good.
    rig.origin.script(
        SETUP,
        vec![
            Reply::status(403),
            Reply::status(403),
            Reply::bytes(200, INSTALLER),
        ],
    );
    assert_eq!(rig.downloaded(&found, false).await, Downloaded::Ready);
    assert_eq!(rig.origin.hits(LATEST).len(), 2);
    assert_eq!(rig.origin.hits(SETUP).len(), 3);
    assert_eq!(rig.updates.ready().unwrap().version, NEW);
}

#[tokio::test]
async fn a_download_refused_every_time_is_read_again_and_tried_once_more_only() {
    let rig = Rig::new().await;
    let found = rig.found(&rig.checker(None)).await;
    rig.origin.script(SETUP, vec![refusal()]);
    let t = failed(rig.downloaded(&found, true).await);
    assert!(t.show);
    assert!(
        t.message.starts_with("403 Forbidden de 127.0.0.1"),
        "{}",
        t.message
    );
    assert!(
        t.message.ends_with(" : bloqué par le réseau ou un proxy ?"),
        "{}",
        t.message
    );
    assert_eq!(
        t.page.as_deref(),
        Some(
            format!(
                "http://127.0.0.1:{}/me/app/releases/tag/v99.0.0",
                rig.origin.port
            )
            .as_str()
        )
    );
    // Twice: the plugin's request and the probe's, each time; latest.json read once more.
    assert_eq!(rig.origin.hits(SETUP).len(), 4);
    assert_eq!(rig.origin.hits(LATEST).len(), 2);
    assert!(rig.updates.ready().is_none());
}

#[tokio::test]
async fn a_download_refused_automatically_is_quiet_until_the_failures_have_gone_on() {
    let rig = Rig::new().await;
    rig.origin.script(SETUP, vec![refusal()]);
    let checker = rig.checker(None);
    for _ in 0..3 {
        // The checks go through; the downloads are refused: a series of their own.
        let found = rig.found(&checker).await;
        assert!(!failed(rig.downloaded(&found, false).await).show);
    }
}

#[tokio::test]
async fn the_domain_told_of_a_download_is_the_one_that_answered_after_the_redirections() {
    let rig = Rig::new().await;
    let assets = Server::start().await;
    assets.script("/assets/app-setup.exe", vec![refusal()]);
    rig.origin.script(
        SETUP,
        vec![Reply::redirect(&format!(
            "http://localhost:{}/assets/app-setup.exe",
            assets.port
        ))],
    );
    let found = rig.found(&rig.checker(None)).await;
    let t = failed(rig.downloaded(&found, true).await);
    assert!(
        t.message.starts_with("403 Forbidden de localhost"),
        "{}",
        t.message
    );
    // The release's page is the link's, not the redirection's.
    assert!(t.page.unwrap().ends_with("/me/app/releases/tag/v99.0.0"));
}

#[tokio::test]
async fn an_installer_on_another_domain_does_not_send_the_browser_there() {
    let rig = Rig::new().await;
    // A latest.json that was tampered with (a proxy that does not verify, a mirror): its link is
    // on another domain, that refuses. The page of its « release » is not opened in the browser.
    let elsewhere = Server::start().await;
    let installer = "/me/app/releases/download/v99.0.0/x";
    elsewhere.script(installer, vec![refusal()]);
    rig.origin.script(
        LATEST,
        vec![latest(&format!(
            "http://localhost:{}{installer}",
            elsewhere.port
        ))],
    );
    let found = rig.found(&rig.checker(None)).await;
    let t = failed(rig.downloaded(&found, true).await);
    assert!(
        t.message.starts_with("403 Forbidden de localhost"),
        "{}",
        t.message
    );
    assert_eq!(t.page.as_deref(), Some(net::RELEASES_URL));
    // The same domain on another port is another origin.
    rig.origin.script(
        LATEST,
        vec![latest(&format!(
            "http://127.0.0.1:{}{installer}",
            elsewhere.port
        ))],
    );
    let found = rig.found(&rig.checker(None)).await;
    let t = failed(rig.downloaded(&found, true).await);
    assert_eq!(t.page.as_deref(), Some(net::RELEASES_URL));
}

#[tokio::test]
async fn a_signed_link_that_fails_is_told_without_its_signature() {
    let rig = Rig::new().await;
    // GitHub redirects the installer to a link signed in its query. Here that link redirects to
    // itself: the plugin gives up on the redirections and says for which link, which the probe
    // (its first request is a success) does not contradict.
    let assets = Server::start().await;
    let link = format!(
        "http://127.0.0.1:{}/asset?sig=SECRET&se=2026-10-15",
        assets.port
    );
    // Eleven redirections end the plugin's request (ten are followed), then the probe's is let
    // through; once more for the second try that a failed download gets.
    let looping = || (0..11).map(|_| Reply::redirect(&link));
    let script = looping()
        .chain([Reply::bytes(200, INSTALLER)])
        .chain(looping())
        .chain([Reply::bytes(200, INSTALLER)])
        .collect();
    assets.script("/asset", script);
    rig.origin.script(SETUP, vec![Reply::redirect(&link)]);
    let found = rig.found(&rig.checker(None)).await;
    let t = failed(rig.downloaded(&found, true).await);
    assert!(
        t.message.starts_with("réponse inattendue de 127.0.0.1 : "),
        "{}",
        t.message
    );
    assert!(t.message.contains("too many redirects"), "{}", t.message);
    // The link is told, its signature is not.
    assert!(t.message.contains("/asset"), "{}", t.message);
    assert!(
        !t.message.contains("SECRET") && !t.message.contains("sig="),
        "{}",
        t.message
    );
}

#[tokio::test]
async fn a_signed_link_that_cannot_be_reached_is_told_without_its_signature_either() {
    let rig = Rig::new().await;
    let assets = Server::start().await;
    assets.script("/asset", vec![Reply::hang_up()]);
    rig.origin.script(
        SETUP,
        vec![Reply::redirect(&format!(
            "http://127.0.0.1:{}/asset?sig=SECRET&se=2026-10-15",
            assets.port
        ))],
    );
    let found = rig.found(&rig.checker(None)).await;
    let t = failed(rig.downloaded(&found, true).await);
    assert!(
        t.message.starts_with("connexion impossible à 127.0.0.1 ("),
        "{}",
        t.message
    );
    assert!(
        !t.message.contains("SECRET") && !t.message.contains("sig="),
        "{}",
        t.message
    );
}

#[tokio::test]
async fn an_installer_that_is_not_the_one_signed_fails_with_the_plugins_words() {
    let rig = Rig::new().await;
    rig.origin
        .script(SETUP, vec![Reply::bytes(200, b"something else")]);
    let found = rig.found(&rig.checker(None)).await;
    let t = failed(rig.downloaded(&found, true).await);
    assert!(
        t.message.starts_with("réponse inattendue de 127.0.0.1 : "),
        "{}",
        t.message
    );
    assert!(
        t.message.to_lowercase().contains("signature"),
        "{}",
        t.message
    );
    assert!(rig.updates.ready().is_none());
}

#[tokio::test]
async fn a_download_that_goes_through_is_checked_by_the_plugin_and_kept() {
    let rig = Rig::new().await;
    let found = rig.found(&rig.checker(None)).await;
    assert_eq!(rig.downloaded(&found, false).await, Downloaded::Ready);
    assert_eq!(rig.updates.ready().unwrap().version, NEW);
    assert_eq!(rig.origin.hits(SETUP).len(), 1);
    assert_eq!(rig.origin.hits(LATEST).len(), 1);
}

// ---------- who the app says it is ----------

#[tokio::test]
async fn the_check_and_the_download_say_who_the_app_is() {
    let rig = Rig::new().await;
    let found = rig.found(&rig.checker(None)).await;
    assert_eq!(rig.downloaded(&found, false).await, Downloaded::Ready);
    let name = net::user_agent();
    assert!(name.starts_with("Escouade/"), "{name}");
    for path in [LATEST, SETUP] {
        let hits = rig.origin.hits(path);
        assert_eq!(hits.len(), 1, "{path}");
        assert_eq!(hits[0].headers["user-agent"], name, "{path}");
    }
    // The plugin's own default is gone.
    assert!(rig
        .origin
        .seen()
        .iter()
        .all(|s| !s.headers["user-agent"].starts_with("tauri-plugin-updater")));
}

// ---------- TLS ----------

#[tokio::test]
async fn tls_verification_off_lets_the_check_and_the_probe_trust_a_self_signed_certificate() {
    let server = crate::integrations::fake::FakeServer::start_tls().await;
    let url = format!("{}{LATEST}", server.url);
    server.on(
        "GET",
        LATEST,
        200,
        manifest(NEW, &format!("{}{SETUP}", server.url)),
    );
    let app = app(std::slice::from_ref(&url));
    let checker = |insecure_tls: bool| {
        Arc::new(Checker {
            app: app.handle().clone(),
            net: Network {
                insecure_tls,
                ..Network::default()
            },
            beta: false,
            endpoints: vec![url.parse().unwrap()],
            pause: Duration::from_millis(5),
        })
    };
    let updates = Updates::default();
    // Verified: the certificate of the server is refused, and said so.
    let t = trouble(within(checker(false).run(&updates, true)).await);
    assert!(
        t.message.starts_with("connexion impossible à 127.0.0.1"),
        "{}",
        t.message
    );
    // Not verified: found.
    let checked = within(checker(true).run(&updates, true)).await;
    assert!(matches!(checked, Checked::Found { .. }), "{checked:?}");
}

// ---------- the beta channel (ESC-374) ----------

const BETA: &str = "/beta.json";

impl Rig {
    /// A check with « Recevoir les versions bêta » on or off, as `Checker::new` makes it.
    fn checker_with_betas(&self, beta: bool) -> Arc<Checker<MockRuntime>> {
        Arc::new(Checker {
            beta,
            endpoints: endpoints_for(&[self.endpoint.parse().unwrap()], beta),
            ..Arc::into_inner(self.checker(None)).unwrap()
        })
    }
}

/// beta.json beside latest.json, holding `version`.
fn serve_beta(rig: &Rig, version: &str) {
    let manifest = manifest(version, &rig.origin.url(SETUP));
    rig.origin
        .script(BETA, vec![Reply::json(200, &manifest.to_string())]);
}

#[tokio::test]
async fn without_the_betas_the_check_reads_latest_json_alone() {
    let rig = Rig::new().await;
    serve_beta(&rig, "99.1.0-beta.1");
    let found = rig.found(&rig.checker_with_betas(false)).await;
    assert_eq!(found.version, NEW);
    assert_eq!(rig.origin.hits(LATEST).len(), 1);
    assert!(rig.origin.hits(BETA).is_empty());
}

#[tokio::test]
async fn with_the_betas_the_check_reads_beta_json_first() {
    let rig = Rig::new().await;
    serve_beta(&rig, "99.1.0-beta.1");
    let found = rig.found(&rig.checker_with_betas(true)).await;
    assert_eq!(found.version, "99.1.0-beta.1");
    assert_eq!(rig.origin.hits(BETA).len(), 1);
    assert!(rig.origin.hits(LATEST).is_empty());
}

#[tokio::test]
async fn with_the_betas_and_no_beta_json_yet_the_check_reads_latest_json() {
    let rig = Rig::new().await;
    // No script for beta.json: the server answers 404, as GitHub before the first release that
    // holds one.
    let found = rig.found(&rig.checker_with_betas(true)).await;
    assert_eq!(found.version, NEW);
    assert_eq!(rig.origin.hits(BETA).len(), 1);
    assert_eq!(rig.origin.hits(LATEST).len(), 1);
}

#[tokio::test]
async fn without_the_betas_a_beta_found_in_latest_json_is_not_offered() {
    let rig = Rig::new().await;
    rig.origin.script(
        LATEST,
        vec![Reply::json(
            200,
            &manifest("99.0.0-beta.1", &rig.origin.url(SETUP)).to_string(),
        )],
    );
    let checked = rig.check(&rig.checker_with_betas(false), true).await;
    assert!(matches!(checked, Checked::None), "{checked:?}");
    // With them, it is.
    let found = rig.found(&rig.checker_with_betas(true)).await;
    assert_eq!(found.version, "99.0.0-beta.1");
}
