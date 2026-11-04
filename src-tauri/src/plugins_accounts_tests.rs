//! Tests of the accounts shared by a plugin's points (INT-6, ESC-212; `docs/PLUGINS.md` §4.4): the
//! kinds of accounts (the core's and a plugin's), the plugin chosen to serve an account, a GitLab
//! account that serves GitLab Issues and the merge requests (on the fake plugin), the budget of
//! calls kept per account, the migration of the keychain's names there and back, and the keychain
//! that never gives a plugin another's secret.

use super::plugins_runtime_tests::{fake, quick, ID};
use crate::forge::accounts::{self as forge_accounts, ForgeAccount};
use crate::forge::{Credentials, Forge, RepoRef};
use crate::integrations::secrets::{self, load_accounts, save_accounts, MemoryStore, SecretStore};
use crate::integrations::source::TicketSource;
use crate::integrations::{Account, Accounts};
use crate::model::Service;
use crate::paths::{test_dir, DataDir};
use crate::plugins::accounts::{self, AccountKind, Connect, Field};
use crate::plugins::adapters::{ProcessForge, ProcessTicketSource};
use crate::plugins::manifest::{self, Hand};
use crate::plugins::runtime::NOT_YOURS;
use serde_json::json;

fn kind<'a>(kinds: &'a [AccountKind], id: &str) -> &'a AccountKind {
    kinds
        .iter()
        .find(|k| k.id == id)
        .unwrap_or_else(|| panic!("no kind {id}"))
}

#[test]
fn the_kinds_of_the_core_say_hosts_connections_fields_rights_and_token_page() {
    let kinds = accounts::standard();
    let ids: Vec<&str> = kinds.iter().map(|k| k.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "github",
            "gitlab",
            "bitbucket",
            "gitea",
            "azure",
            "jira",
            "trello",
            "linear",
            "clickup"
        ]
    );

    let gh = kind(&kinds, "github");
    assert_eq!(gh.plugin, "dev.escouade.github");
    assert_eq!(gh.default_host.as_deref(), Some("github.com"));
    assert!(gh.connect.contains(&Connect::Cli {
        program: "gh".into(),
        login: "gh auth login".into()
    }));
    assert_eq!(
        gh.scopes,
        crate::forge::Provider::scopes(&crate::forge::github::GithubProvider)
    );
    assert_eq!(
        gh.token_page.as_deref(),
        Some("https://github.com/settings/personal-access-tokens/new")
    );
    // One connection, two uses: its Issues and its pull requests.
    assert_eq!(
        (gh.sources.clone(), gh.forge.as_deref()),
        (vec!["github".to_string()], Some("github"))
    );
    assert!(gh.sources_public_only, "a GitHub Enterprise's: no Issues");

    let gl = kind(&kinds, "gitlab");
    assert_eq!(
        (gl.sources.clone(), gl.forge.as_deref()),
        (vec!["gitlab".to_string()], Some("gitlab"))
    );
    assert!(gl.self_hosted);
    assert_eq!(gl.probe.as_deref(), Some("/api/v4/version"));

    let az = kind(&kinds, "azure");
    assert_eq!(az.plugin, "dev.escouade.azure-devops");
    assert_eq!(az.sources, ["azure"]);

    let bb = kind(&kinds, "bitbucket");
    assert!(!bb.self_hosted && bb.sources.is_empty());
    assert!(bb.fields.contains(&Field {
        key: "email".into(),
        secret: false
    }));

    let jira = kind(&kinds, "jira");
    let keys: Vec<(&str, bool)> = jira
        .fields
        .iter()
        .map(|f| (f.key.as_str(), f.secret))
        .collect();
    assert_eq!(keys, [("site", false), ("email", false), ("token", true)]);
    assert!(jira.self_hosted && jira.forge.is_none());
    let trello = kind(&kinds, "trello");
    assert!(trello.fields.iter().all(|f| f.secret), "its key and token");
    assert_eq!(trello.default_host.as_deref(), Some("trello.com"));
}

/// A manifest that declares a kind of account, and a source and a host on it.
fn jira_dc() -> manifest::Manifest {
    let v = json!({
        "id": "acme.jira-dc",
        "name": "Jira Data Center",
        "version": "1.0.0",
        "escouade": { "api": "1", "min": "2.1.0" },
        "run": { "command": "jira-dc" },
        "contributes": {
            "accounts": [{
                "id": "jira",
                "name": "Jira Data Center",
                "selfHosted": true,
                "connect": ["token", { "cli": "jira-cli", "login": "jira-cli login" }],
                "fields": [{ "key": "site" }, { "key": "token", "secret": true }],
                "scopes": ["read:jira-work"],
                "tokenPage": "{url}/secure/ViewProfile.jspa"
            }],
            "ticketSources": [{ "id": "jira-dc", "name": "Jira DC", "account": "jira" }],
            "forges": [{ "id": "bitbucket-dc", "name": "Bitbucket DC", "account": "jira" }]
        }
    });
    manifest::check(&v, Hand::Third).manifest.unwrap()
}

#[test]
fn a_plugin_declares_its_kind_and_the_points_that_share_it() {
    let kinds = accounts::of_manifest(&jira_dc());
    assert_eq!(kinds.len(), 1);
    let k = &kinds[0];
    assert_eq!((k.id.as_str(), k.plugin.as_str()), ("jira", "acme.jira-dc"));
    assert_eq!(k.name, "Jira Data Center");
    assert!(k.self_hosted);
    assert_eq!(
        k.connect,
        [
            Connect::Token,
            Connect::Cli {
                program: "jira-cli".into(),
                login: "jira-cli login".into()
            }
        ]
    );
    assert_eq!(
        k.fields,
        [
            Field {
                key: "site".into(),
                secret: false
            },
            Field {
                key: "token".into(),
                secret: true
            }
        ]
    );
    assert_eq!(k.scopes, ["read:jira-work"]);
    assert_eq!(
        k.token_page.as_deref(),
        Some("{url}/secure/ViewProfile.jspa")
    );
    // Its account serves its source and its host alike.
    assert_eq!(k.sources, ["jira-dc"]);
    assert_eq!(k.forge.as_deref(), Some("bitbucket-dc"));
    assert_eq!(
        accounts::points(k, None),
        ["ticketSources:jira-dc", "forges:bitbucket-dc"]
    );
}

#[test]
fn the_plugin_that_serves_an_account_is_chosen_when_it_is_added_never_in_silence() {
    let kinds = accounts::kinds(&[jira_dc()]);
    let servers = accounts::servers(&kinds, "jira");
    let plugins: Vec<(&str, bool)> = servers
        .iter()
        .map(|s| (s.plugin.as_str(), s.first_hand))
        .collect();
    assert_eq!(
        plugins,
        [("dev.escouade.jira", true), ("acme.jira-dc", false)]
    );
    // Escouade's own hand is the one kept by default, and not written.
    assert_eq!(accounts::chosen(&kinds, "jira", "").unwrap(), "");
    assert_eq!(
        accounts::chosen(&kinds, "jira", "dev.escouade.jira").unwrap(),
        ""
    );
    assert_eq!(
        accounts::chosen(&kinds, "jira", "acme.jira-dc").unwrap(),
        "acme.jira-dc"
    );
    // Not a plugin that serves no account of it.
    assert!(accounts::chosen(&kinds, "trello", "acme.jira-dc").is_err());
    assert!(accounts::chosen(&kinds, "jira", "acme.other").is_err());
    assert_eq!(accounts::servers(&kinds, "trello").len(), 1);
}

fn gitlab_account(plugin: &str) -> ForgeAccount {
    ForgeAccount {
        kind: "gitlab".into(),
        url: "https://gitlab.com".into(),
        label: "@ada".into(),
        user: "ada".into(),
        plugin: plugin.into(),
        ..ForgeAccount::default()
    }
}

#[test]
fn one_account_serves_every_point_of_its_plugin() {
    let mut a = Accounts {
        github: Some(Account {
            label: "@ada".into(),
            user: "ada".into(),
            ..Account::default()
        }),
        jira: Some(Account {
            label: "ada@atlas.dev".into(),
            ..Account::default()
        }),
        ..Accounts::default()
    };
    a.forges.push(gitlab_account(""));
    a.forges.push(ForgeAccount {
        kind: "github".into(),
        url: "https://git.acme.dev".into(),
        label: "@ada".into(),
        ..ForgeAccount::default()
    });
    let shared = accounts::shared(&a, &accounts::standard());
    let by = |account: &str| {
        shared
            .iter()
            .find(|s| s.account == account)
            .unwrap_or_else(|| panic!("no {account}"))
            .clone()
    };
    let gh = by("github.com");
    assert_eq!(gh.plugin, "dev.escouade.github");
    assert_eq!(gh.points, ["ticketSources:github", "forges:github"]);
    let gl = by("gitlab.com");
    assert_eq!(gl.plugin, "dev.escouade.gitlab");
    assert_eq!(gl.points, ["ticketSources:gitlab", "forges:gitlab"]);
    assert!(a.connected(Service::GITLAB), "GitLab Issues on GitLab's");
    // A GitHub Enterprise's: its pull requests only.
    assert_eq!(by("git.acme.dev").points, ["forges:github"]);
    assert_eq!(by("jira").points, ["ticketSources:jira"]);
    assert_eq!(shared.len(), 4);
}

fn repo() -> RepoRef {
    RepoRef {
        namespace: "acme".into(),
        name: "demo".into(),
    }
}

/// A GitLab account served by a plugin: one secret, under the plugin's name, serves its Issues and
/// its merge requests, and their calls spend the same budget.
#[tokio::test(flavor = "multi_thread")]
async fn a_gitlab_account_serves_issues_and_merge_requests_on_the_fakes() {
    let f = fake(
        "gitlab-shared",
        &["--token-from", "account:gitlab.com", "--ticket-budget"],
        quick(),
    );
    let mut account = gitlab_account(ID);
    account.token = "good".into();
    assert!(forge_accounts::place(&*f.store, &mut account));
    assert_eq!(
        f.store
            .entry("plugin:dev.test.fake:account:gitlab.com")
            .as_deref(),
        Some("good")
    );
    // Another plugin than Escouade's own hand: no former name for a version from before.
    assert_eq!(f.store.entry("forge:gitlab.com"), None);

    let issues = ProcessTicketSource::new(
        f.rt.clone(),
        ID,
        "gitlab",
        &Account {
            site: "https://gitlab.com".into(),
            ..Account::default()
        },
    );
    let mrs = ProcessForge::new(
        f.rt.clone(),
        ID,
        "gitlab",
        &Credentials {
            url: "https://gitlab.com".into(),
            ..Credentials::default()
        },
    );
    assert!(mrs.rate_limit().is_none());
    assert_eq!(mrs.pull_request(&repo(), 7).await.unwrap().number, 7);
    // The host's answer said 4,990 left: the source sees the account's budget.
    assert_eq!(issues.rate_limit().unwrap().remaining, 4990);
    assert_eq!(issues.containers().await.unwrap().len(), 2);
    // The source's answer said 4,000: the host sees it too.
    assert_eq!(mrs.rate_limit().unwrap().remaining, 4000);

    // Another account of the same plugin keeps its own budget.
    let other = ProcessForge::new(
        f.rt.clone(),
        ID,
        "gitlab",
        &Credentials {
            url: "https://gitlab.acme.dev".into(),
            ..Credentials::default()
        },
    );
    assert!(other.rate_limit().is_none());

    // Disconnected: the secret goes, and both points are refused.
    assert!(forge_accounts::forget_of(
        &*f.store,
        "gitlab.com",
        "gitlab",
        ID
    ));
    assert_eq!(
        f.store.entry("plugin:dev.test.fake:account:gitlab.com"),
        None
    );
    assert!(issues.containers().await.is_err());
    assert!(mrs.pull_request(&repo(), 7).await.is_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn the_keychain_never_gives_a_plugin_another_plugins_account() {
    let f = fake("vault-other", &[], quick());
    // GitLab's account, served by Escouade's own hand, and its former name.
    f.store
        .set(
            "plugin:dev.escouade.gitlab:account:gitlab.com",
            "theirs-1234",
        )
        .unwrap();
    f.store.set("forge:gitlab.com", "theirs-1234").unwrap();
    for key in [
        "plugin:dev.escouade.gitlab:account:gitlab.com",
        "forge:gitlab.com",
        "account:../dev.escouade.gitlab:account:gitlab.com",
    ] {
        let got =
            f.rt.call(ID, "test/do", json!({ "behave": "steal", "key": key }))
                .await
                .unwrap();
        if key.starts_with("account:") {
            // Its pattern takes it in, under its own name: nothing there.
            assert_eq!(got, json!({ "result": null }), "{key}");
        } else {
            assert_eq!(got["error"]["code"], json!(NOT_YOURS), "{key}: {got}");
        }
        assert!(!got.to_string().contains("theirs-1234"), "{key}: {got}");
    }
    // Its own account of the same instance, under its own name.
    f.store
        .set(&accounts::vault_name(ID, "gitlab.com"), "mine-5678")
        .unwrap();
    let got =
        f.rt.call(
            ID,
            "test/do",
            json!({ "behave": "secret", "key": "account:gitlab.com" }),
        )
        .await
        .unwrap();
    assert_eq!(got, json!({ "result": "mine-5678" }));
}

/// `integrations.json` and the keychain as a version before INT-6 left them.
const BEFORE: &str = r#"{
  "jira": { "site": "https://atlas.atlassian.net", "email": "ada@atlas.dev", "label": "ada@atlas.dev", "user": "557058" },
  "trello": { "label": "@ada", "user": "m1" },
  "github": { "label": "@ada", "user": "ada" },
  "forges": [
    { "kind": "gitlab", "url": "https://gitlab.com", "label": "@ada", "user": "ada" },
    { "kind": "gitea", "url": "https://git.acme.dev", "label": "@ada", "user": "ada", "viaCli": false }
  ]
}"#;

fn before(store: &MemoryStore) {
    for (name, secret) in [
        ("jira", r#"{"key":"","token":"jira-secret"}"#),
        ("trello", r#"{"key":"trello-key","token":"trello-secret"}"#),
        ("github", r#"{"key":"","token":"gh-secret"}"#),
        ("forge:gitlab.com", "gl-secret"),
        ("forge:git.acme.dev", "gitea-secret"),
    ] {
        store.set(name, secret).unwrap();
    }
}

#[test]
fn the_migration_of_the_keychains_names_goes_there_and_back() {
    let d = DataDir::new(test_dir("accounts-vault"));
    std::fs::write(d.integrations_file(), BEFORE).unwrap();
    let store = MemoryStore::default();
    before(&store);
    let was = store.all();

    // There: every secret under its plugin's name too, the former names kept as they were.
    let mut a = load_accounts(&d, &store, false);
    assert_eq!(a.vault, accounts::VAULT);
    assert_eq!(a.jira.as_ref().unwrap().token, "jira-secret");
    assert_eq!(a.forges[0].token, "gl-secret");
    for (name, secret) in [
        (
            "plugin:dev.escouade.jira:account:jira",
            r#"{"key":"","token":"jira-secret"}"#,
        ),
        (
            "plugin:dev.escouade.trello:account:trello",
            r#"{"key":"trello-key","token":"trello-secret"}"#,
        ),
        (
            "plugin:dev.escouade.github:account:github.com",
            r#"{"key":"","token":"gh-secret"}"#,
        ),
        ("plugin:dev.escouade.gitlab:account:gitlab.com", "gl-secret"),
        (
            "plugin:dev.escouade.gitea:account:git.acme.dev",
            "gitea-secret",
        ),
    ] {
        assert_eq!(store.entry(name).as_deref(), Some(secret), "{name}");
    }
    for (name, secret) in &was {
        assert_eq!(store.entry(name).as_ref(), Some(secret), "{name} kept");
    }
    // The file keeps its shape, says so at its next write, and holds no secret.
    save_accounts(&d, &a).unwrap();
    let text = std::fs::read_to_string(d.integrations_file()).unwrap();
    assert!(text.contains("\"vault\": 2"), "{text}");
    assert!(!text.contains("secret"), "{text}");
    // A version from before reads it: the field it does not know is left aside.
    let older: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(older["jira"]["site"], "https://atlas.atlassian.net");

    // Loaded again: nothing moves.
    let again = load_accounts(&d, &store, false);
    assert_eq!(again.trello.as_ref().unwrap().key, "trello-key");
    assert_eq!(store.all().len(), was.len() * 2);

    // And back: the former names alone, as they were, and the file without the field.
    accounts::revert(&store, &mut a).unwrap();
    save_accounts(&d, &a).unwrap();
    assert_eq!(store.all(), was);
    let text = std::fs::read_to_string(d.integrations_file()).unwrap();
    assert!(!text.contains("vault"), "{text}");
    // There again from there.
    load_accounts(&d, &store, false);
    assert_eq!(store.all().len(), was.len() * 2);
}

#[test]
fn a_token_a_version_from_before_changed_is_taken_back() {
    // Back to a version from before, the account connected again there: its former entry holds
    // the new token, the plugin's name the old one. Read first while it is kept, it wins.
    let d = DataDir::new(test_dir("accounts-vault-back"));
    std::fs::write(
        d.integrations_file(),
        r#"{ "linear": { "label": "Ada" }, "forges": [{ "kind": "gitlab", "url": "https://gitlab.com" }] }"#,
    )
    .unwrap();
    let store = MemoryStore::default();
    store
        .set("linear", r#"{"key":"","token":"lin-new"}"#)
        .unwrap();
    store
        .set(
            "plugin:dev.escouade.linear:account:linear",
            r#"{"key":"","token":"lin-old"}"#,
        )
        .unwrap();
    store.set("forge:gitlab.com", "gl-new").unwrap();
    store
        .set("plugin:dev.escouade.gitlab:account:gitlab.com", "gl-old")
        .unwrap();
    let a = load_accounts(&d, &store, false);
    assert_eq!(a.linear.as_ref().unwrap().token, "lin-new");
    assert_eq!(a.forges[0].token, "gl-new");
    assert_eq!(
        store
            .entry("plugin:dev.escouade.linear:account:linear")
            .as_deref(),
        Some(r#"{"key":"","token":"lin-new"}"#)
    );
    assert_eq!(
        store
            .entry("plugin:dev.escouade.gitlab:account:gitlab.com")
            .as_deref(),
        Some("gl-new")
    );
    // Disconnected there (its former entry gone): not read under the plugin's name either.
    store.delete("linear").unwrap();
    let a = load_accounts(&d, &store, false);
    assert!(a.linear.as_ref().unwrap().unread);
}

#[test]
fn a_disconnect_forgets_every_name_of_the_account() {
    let store = MemoryStore::default();
    let mut jira = Account {
        site: "https://atlas.atlassian.net".into(),
        token: "jira-secret".into(),
        ..Account::default()
    };
    assert!(secrets::place(&store, Service::JIRA, &mut jira));
    assert!(store.entry("jira").is_some());
    assert!(store
        .entry("plugin:dev.escouade.jira:account:jira")
        .is_some());
    assert!(secrets::forget(&store, Service::JIRA));
    assert!(store.all().is_empty());

    let mut gl = gitlab_account("");
    gl.token = "gl".into();
    assert!(forge_accounts::place(&store, &mut gl));
    assert_eq!(store.all().len(), 2);
    // A « Déconnecter » that failed knows only the host, at the next start.
    assert!(forge_accounts::forget(&store, "gitlab.com"));
    assert!(store.all().is_empty());
}

/// A plugin that serves GitLab accounts (its source and its host), loaded in development in the
/// harness's core.
fn gitlab_plugin(h: &crate::core_tests::Harness, id: &str) {
    #[cfg(windows)]
    crate::paths::trim_test_path();
    let dir = h.dir.join(id);
    std::fs::create_dir_all(&dir).unwrap();
    let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures");
    for f in ["fake-plugin.mjs", "fake-plugin.cmd", "fake-plugin"] {
        std::fs::copy(fixtures.join(f), dir.join(f)).unwrap();
    }
    let manifest = json!({
        "id": id,
        "name": "GitLab d'entreprise",
        "version": "1.0.0",
        "escouade": { "api": "1", "min": "2.1.0" },
        "run": {
            "command": "fake-plugin",
            "windows": "fake-plugin.cmd",
            "args": ["--token-from", "account:gitlab.com"]
        },
        "permissions": { "secrets": ["account:*"] },
        "contributes": {
            "accounts": [{ "id": "gitlab", "name": "GitLab" }],
            "ticketSources": [{ "id": "gitlab", "name": "GitLab Issues", "account": "gitlab" }],
            "forges": [{ "id": "gitlab", "name": "GitLab", "account": "gitlab" }]
        }
    });
    std::fs::write(
        dir.join("escouade-plugin.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();
    h.core
        .plugins
        .load_dev(&dir, crate::i18n::Lang::En)
        .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn an_account_added_through_another_plugin_is_served_by_it_for_issues_and_mrs() {
    use crate::forge::connect::ForgeForm;
    let h = crate::core_tests::harness("int6-chosen-plugin");
    gitlab_plugin(&h, "acme.gitlab");
    let store = secrets::memory_of(&h.core.data);

    // Offered beside Escouade's own GitLab.
    let view = h.core.forges_view().await;
    let gl = view.providers.iter().find(|p| p.kind == "gitlab").unwrap();
    let servers: Vec<&str> = gl.servers.iter().map(|s| s.plugin.as_str()).collect();
    assert_eq!(servers, ["dev.escouade.gitlab", "acme.gitlab"]);

    // Not a plugin that serves no GitLab account.
    let form = |token: &str, plugin: &str| ForgeForm {
        kind: "gitlab".into(),
        token: token.into(),
        plugin: plugin.into(),
        ..Default::default()
    };
    assert!(h
        .core
        .forge_connect(form("good", "acme.none"))
        .await
        .is_err());
    // Refused by the plugin: nothing kept.
    assert!(h
        .core
        .forge_connect(form("bad", "acme.gitlab"))
        .await
        .is_err());
    assert!(store.all().is_empty(), "{:?}", store.all().keys());

    let view = h
        .core
        .forge_connect(form("good", "acme.gitlab"))
        .await
        .unwrap();
    let a = &view.accounts[0];
    assert_eq!((a.host.as_str(), a.label.as_str()), ("gitlab.com", "@ada"));
    assert_eq!(a.plugin, "acme.gitlab");
    assert!(a.issues, "its GitLab Issues too");
    // Under the plugin's name alone: a version from before does not know that plugin.
    let names: Vec<String> = store.all().into_keys().collect();
    assert_eq!(names, ["plugin:acme.gitlab:account:gitlab.com"]);
    let text = std::fs::read_to_string(h.core.data.integrations_file()).unwrap();
    assert!(text.contains("\"plugin\": \"acme.gitlab\""), "{text}");
    assert!(!text.contains("good"), "{text}");

    // Its merge requests and its Issues, through the plugin.
    let (mrs, _) = h.core.forge_client("gitlab.com").await.unwrap();
    assert_eq!(mrs.pull_request(&repo(), 7).await.unwrap().number, 7);
    let containers = h
        .core
        .integration_containers(Service::GITLAB, None)
        .await
        .unwrap();
    assert_eq!(containers.len(), 2);

    // Disconnected: its secret goes.
    h.core.forge_disconnect("gitlab.com").await.unwrap();
    assert!(store.all().is_empty());
}

// The Mistral account (ESC-327, INT-10.6 bis): the plugin's, shared by the voice and Vibe, its keys under their former
// names read first; on a fake keychain, never the user's.

const VOICE_FORMER: &str = "dictation-mistral";

fn vibe_account(id: &str, name: &str) -> crate::model::Account {
    crate::model::Account {
        id: id.into(),
        name: name.into(),
        engine: crate::model::EngineId::VIBE,
        active: true,
        // Unique to the test: what the keys in memory go by.
        config_dir: test_dir(id).to_string_lossy().into_owned(),
        ..Default::default()
    }
}

fn mistral_manifest() -> manifest::Manifest {
    let text = crate::plugins::builtin::manifest_text("dev.escouade.mistral", "2.2.0").unwrap();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    manifest::check(&v, Hand::Builtin)
        .manifest
        .expect("the manifest of Mistral holds")
}

#[test]
fn the_plugin_mistral_declares_one_account_for_its_voice_and_vibe() {
    let m = mistral_manifest();
    let kinds = accounts::of_manifest(&m);
    let k = kind(&kinds, accounts::MISTRAL);
    assert_eq!(k.plugin, "dev.escouade.mistral");
    assert_eq!(k.connect, [Connect::Token]);
    assert_eq!(
        k.fields,
        [Field {
            key: "token".into(),
            secret: true
        }]
    );
    assert_eq!(
        accounts::points(k, None),
        ["harnesses:vibe", "voice:mistral"],
        "one account, every point of its plugin"
    );
    // The plugin reads no secret: the key goes in each call to its voice, and to Vibe by MISTRAL_API_KEY.
    assert!(m.permissions.secrets.is_empty());
}

#[test]
fn nobody_gives_the_voice_key_again_and_its_former_entry_stays_read_first() {
    let store = MemoryStore::default();
    store.set(VOICE_FORMER, "cle-de-la-voix").unwrap();
    let name = accounts::mistral_vault_name();
    assert_eq!(name, "plugin:dev.escouade.mistral:account:mistral");
    assert_eq!(
        crate::voice::online::mistral_key(&store).as_deref(),
        Some("cle-de-la-voix")
    );
    assert_eq!(
        store.get(&name).unwrap().as_deref(),
        Some("cle-de-la-voix"),
        "copied under the account"
    );
    assert_eq!(
        store.get(VOICE_FORMER).unwrap().as_deref(),
        Some("cle-de-la-voix"),
        "the former one stays"
    );
    // A version from before changed it after a return to it: taken back.
    store.set(VOICE_FORMER, "cle-changee-avant").unwrap();
    assert_eq!(
        crate::voice::online::mistral_key(&store).as_deref(),
        Some("cle-changee-avant")
    );
    assert_eq!(
        store.get(&name).unwrap().as_deref(),
        Some("cle-changee-avant")
    );
    // A version from before took it away: the former name decides.
    store.delete(VOICE_FORMER).unwrap();
    assert_eq!(crate::voice::online::mistral_key(&store), None);
}

#[test]
fn nobody_gives_the_key_of_a_vibe_account_again() {
    use crate::accounts::{forget_account_key, keep_account_key, read_account_key};
    use crate::model::EngineId;
    let store = MemoryStore::default();
    store.set("vibe:equipe", "cle-du-compte-equipe").unwrap();
    let name = accounts::vibe_vault_name("equipe");
    assert_eq!(name, "plugin:dev.escouade.mistral:account:vibe-equipe");
    assert_eq!(
        read_account_key(&store, EngineId::VIBE, "equipe")
            .unwrap()
            .as_deref(),
        Some("cle-du-compte-equipe")
    );
    assert_eq!(
        store.get(&name).unwrap().as_deref(),
        Some("cle-du-compte-equipe")
    );
    keep_account_key(&store, EngineId::VIBE, "equipe", "nouvelle-cle-equipe").unwrap();
    assert_eq!(
        store.get("vibe:equipe").unwrap().as_deref(),
        Some("nouvelle-cle-equipe")
    );
    assert_eq!(
        store.get(&name).unwrap().as_deref(),
        Some("nouvelle-cle-equipe")
    );
    forget_account_key(&store, EngineId::VIBE, "equipe").unwrap();
    assert_eq!(store.get("vibe:equipe").unwrap(), None);
    assert_eq!(store.get(&name).unwrap(), None);
    // Cursor's keep their one name.
    keep_account_key(&store, EngineId::CURSOR, "pro", "cle-de-cursor").unwrap();
    assert_eq!(
        store.get("cursor:pro").unwrap().as_deref(),
        Some("cle-de-cursor")
    );
    assert_eq!(
        read_account_key(&store, EngineId::CURSOR, "pro")
            .unwrap()
            .as_deref(),
        Some("cle-de-cursor")
    );
}

#[test]
fn the_migration_of_the_mistral_keys_goes_back() {
    let store = MemoryStore::default();
    store.set(VOICE_FORMER, "cle-de-la-voix").unwrap();
    store.set("vibe:equipe", "cle-du-compte-equipe").unwrap();
    let vibe = [(
        "vibe:equipe".to_string(),
        accounts::vibe_vault_name("equipe"),
    )];
    crate::voice::online::mistral_key(&store);
    crate::accounts::read_account_key(&store, crate::model::EngineId::VIBE, "equipe").unwrap();
    // A key only under its new name (the version that drops the former ones) goes back too.
    store.delete("vibe:equipe").unwrap();
    accounts::revert_mistral(&store, VOICE_FORMER, &vibe).unwrap();
    assert_eq!(
        store.get(VOICE_FORMER).unwrap().as_deref(),
        Some("cle-de-la-voix")
    );
    assert_eq!(
        store.get("vibe:equipe").unwrap().as_deref(),
        Some("cle-du-compte-equipe")
    );
    assert_eq!(store.get(&accounts::mistral_vault_name()).unwrap(), None);
    assert_eq!(store.get(&vibe[0].1).unwrap(), None);
}

#[test]
fn the_voice_and_vibe_go_by_each_others_key_and_a_key_of_its_own_wins() {
    use crate::accounts::{account_key, key_ready, remember_key, remember_mistral_key};
    use crate::voice::online::{self, Provider};
    let team = vibe_account("vibe-equipe", "Équipe");
    let other = vibe_account("vibe-autre", "Autre");
    let settings = crate::model::Settings {
        accounts: vec![team.clone(), other.clone()],
        ..Default::default()
    };
    let store = MemoryStore::default();
    // Nothing yet: a Vibe account cannot start, the voice has no key.
    assert!(key_ready(&team).is_err());
    assert!(online::keyed(Provider::Mistral, &settings, &store).is_err());
    // The key of a Vibe account serves the voice, which says whose it is.
    remember_key(&team, Some("cle-du-compte-equipe"));
    let spec = online::keyed(Provider::Mistral, &settings, &store).expect("Vibe's key");
    assert_eq!(spec.key, "cle-du-compte-equipe");
    let state = online::mistral_state(&settings);
    assert_eq!(state.own, None);
    let borrowed = state.borrowed.expect("the account it goes by");
    assert_eq!(borrowed.account, "Équipe");
    assert!(borrowed.hint.ends_with("uipe"), "{}", borrowed.hint);
    // Another service borrows nothing.
    assert!(online::keyed(Provider::OpenAi, &settings, &store).is_err());
    // The Mistral account's key: the voice's, and Vibe's accounts without a key of their own go by it; never in place of one's own.
    store.set(VOICE_FORMER, "cle-de-la-voix-0000").unwrap();
    remember_mistral_key(online::mistral_key(&store).as_deref());
    let spec = online::keyed(Provider::Mistral, &settings, &store).expect("its key");
    assert_eq!(spec.key, "cle-de-la-voix-0000");
    assert_eq!(online::mistral_state(&settings).borrowed, None);
    assert_eq!(account_key(&other).as_deref(), Some("cle-de-la-voix-0000"));
    assert!(key_ready(&other).is_ok());
    assert_eq!(account_key(&team).as_deref(), Some("cle-du-compte-equipe"));
    // A key of the Principal's is the machine's: never read, never shared.
    let principal = crate::model::Account {
        id: "principal-vibe".into(),
        ..vibe_account("principal-vibe", "Principal")
    };
    assert!(!crate::accounts::by_key(&principal));
}
