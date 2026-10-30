//! The code hosts' interface: the five hosts' remotes in every form, each client's identity check
//! on a fake server, the accounts and their tokens, and a fake host plugged into the real core
//! without touching it.

use super::accounts::{self, ForgeAccount};
use super::connect::{check, guess, ForgeForm, Health};
use super::*;
use crate::core_tests::{git, harness};
use crate::integrations::fake::FakeServer;
use crate::integrations::secrets::{MemoryStore, SecretStore};
use crate::integrations::{Account, Accounts};
use serde_json::json;

fn analyze(url: &str) -> Option<(Option<String>, String, String)> {
    Registry::standard()
        .analyze(url, |_| None)
        .map(|r| (r.kind, r.host, r.repo.full()))
}

fn on(kind: &str, host: &str, repo: &str) -> Option<(Option<String>, String, String)> {
    Some((Some(kind.into()), host.into(), repo.into()))
}

#[test]
fn github_remotes_are_read_in_every_form() {
    for url in [
        "https://github.com/acme/demo.git",
        "https://github.com/acme/demo",
        "http://github.com/acme/demo/",
        "https://x-access-token:ghs_secret@github.com/acme/demo.git",
        "git@github.com:acme/demo.git",
        "github.com:acme/demo",
        "ssh://git@github.com/acme/demo.git",
        "ssh://git@ssh.github.com:443/acme/demo.git",
        "git://github.com/acme/demo.git",
    ] {
        assert_eq!(
            analyze(url),
            on("github", "github.com", "acme/demo"),
            "{url}"
        );
    }
    // GHE.com: its own host.
    assert_eq!(
        analyze("git@octo.ghe.com:acme/demo.git"),
        on("github", "octo.ghe.com", "acme/demo")
    );
    // A GitHub path is an owner and a repository, nothing more.
    assert_eq!(analyze("https://github.com/acme/demo/tree/main"), None);
    assert_eq!(analyze("https://github.com/acme"), None);
}

#[test]
fn gitlab_remotes_keep_their_subgroups() {
    for url in [
        "https://gitlab.com/acme/web/front/app.git",
        "https://oauth2:glpat-x@gitlab.com/acme/web/front/app",
        "git@gitlab.com:acme/web/front/app.git",
        "ssh://git@gitlab.com/acme/web/front/app.git",
        "ssh://git@altssh.gitlab.com:443/acme/web/front/app.git",
    ] {
        assert_eq!(
            analyze(url),
            on("gitlab", "gitlab.com", "acme/web/front/app"),
            "{url}"
        );
    }
    assert_eq!(
        analyze("git@gitlab.com:ada/blog.git"),
        on("gitlab", "gitlab.com", "ada/blog")
    );
}

#[test]
fn bitbucket_remotes_are_a_workspace_and_a_repository() {
    for url in [
        "https://ada@bitbucket.org/atlas/api.git",
        "https://bitbucket.org/atlas/api",
        "git@bitbucket.org:atlas/api.git",
        "ssh://git@bitbucket.org/atlas/api.git",
        "ssh://git@altssh.bitbucket.org:443/atlas/api.git",
    ] {
        assert_eq!(
            analyze(url),
            on("bitbucket", "bitbucket.org", "atlas/api"),
            "{url}"
        );
    }
    assert_eq!(analyze("https://bitbucket.org/atlas/team/api"), None);
}

#[test]
fn gitea_and_forgejo_remotes_are_known_by_their_public_hosts() {
    for (url, host) in [
        ("https://codeberg.org/ada/notes.git", "codeberg.org"),
        ("git@codeberg.org:ada/notes.git", "codeberg.org"),
        ("ssh://git@codeberg.org:22/ada/notes.git", "codeberg.org"),
        ("https://gitea.com/ada/notes", "gitea.com"),
    ] {
        assert_eq!(analyze(url), on("gitea", host, "ada/notes"), "{url}");
    }
}

#[test]
fn a_self_hosted_instance_is_known_by_its_account_and_otherwise_left_unknown() {
    let r = Registry::standard();
    let known = |host: &str| match host {
        "git.acme.dev" => Some("gitea".to_string()),
        "gitlab.acme.dev" => Some("gitlab".to_string()),
        "ghe.acme.dev" => Some("github".to_string()),
        _ => None,
    };
    let read = |url: &str| {
        r.analyze(url, known)
            .map(|x| (x.kind, x.host, x.repo.full()))
    };
    // Its ssh on another port: the same instance.
    assert_eq!(
        read("ssh://git@git.acme.dev:2222/ada/notes.git"),
        on("gitea", "git.acme.dev", "ada/notes")
    );
    assert_eq!(
        read("https://git.acme.dev:8443/ada/notes"),
        on("gitea", "git.acme.dev", "ada/notes")
    );
    assert_eq!(
        read("git@gitlab.acme.dev:acme/web/app.git"),
        on("gitlab", "gitlab.acme.dev", "acme/web/app")
    );
    assert_eq!(
        read("https://ghe.acme.dev/acme/demo.git"),
        on("github", "ghe.acme.dev", "acme/demo")
    );
    // No account: « Connecter <hôte> », its path read as most hosts lay it out.
    assert_eq!(
        read("git@forge.example.org:team/sub/app.git"),
        Some((None, "forge.example.org".into(), "team/sub/app".into()))
    );
    assert_eq!(read("C:/remotes/demo.git"), None);
    // The address its account is connected with: an https remote's, its port with it.
    let url = |u: &str| r.analyze(u, known).unwrap().url;
    assert_eq!(
        url("https://git.acme.dev:8443/ada/notes"),
        "https://git.acme.dev:8443"
    );
    assert_eq!(
        url("http://127.0.0.1:3000/ada/notes"),
        "http://127.0.0.1:3000"
    );
    assert_eq!(
        url("ssh://git@git.acme.dev:2222/ada/notes.git"),
        "https://git.acme.dev"
    );
    assert_eq!(
        url("git@gitlab.acme.dev:acme/web/app.git"),
        "https://gitlab.acme.dev"
    );
}

#[test]
fn the_board_still_reads_github_com_repositories() {
    assert_eq!(
        crate::board::github_repo("ssh://git@ssh.github.com:443/acme/demo.git"),
        Some(("acme".into(), "demo".into()))
    );
    assert_eq!(
        crate::board::github_repo("git@gitlab.com:acme/demo.git"),
        None
    );
    assert_eq!(
        crate::board::github_repo("git@octo.ghe.com:acme/demo.git"),
        None
    );
}

#[test]
fn each_host_says_its_word_and_what_it_can_do() {
    let r = Registry::standard();
    let kinds: Vec<&str> = r.all().iter().map(|p| p.kind()).collect();
    assert_eq!(kinds, ["github", "gitlab", "bitbucket", "gitea", "azure"]);
    assert_eq!(r.get("azure").unwrap().request_word(), "PR");
    assert_eq!(r.get("gitlab").unwrap().request_word(), "MR");
    assert_eq!(r.get("github").unwrap().request_word(), "PR");
    assert!(!r.get("bitbucket").unwrap().capabilities().auto_merge);
    assert!(r.get("github").unwrap().capabilities().auto_merge);
    assert!(r.get("bitbucket").unwrap().needs_email());
    assert_eq!(
        r.get("github").unwrap().token_page("https://github.com"),
        "https://github.com/settings/personal-access-tokens/new"
    );
    assert_eq!(
        r.get("github").unwrap().token_page("https://ghe.acme.dev"),
        "https://ghe.acme.dev/settings/tokens/new"
    );
    assert_eq!(
        github::api_base("https://octo.ghe.com"),
        "https://api.octo.ghe.com"
    );
    assert_eq!(
        github::api_base("https://ghe.acme.dev"),
        "https://ghe.acme.dev/api/v3"
    );
    assert_eq!(
        r.get("gitlab")
            .unwrap()
            .cli("gitlab.acme.dev")
            .unwrap()
            .args,
        [
            "auth",
            "status",
            "--show-token",
            "--hostname",
            "gitlab.acme.dev"
        ]
    );
}

#[test]
fn glabs_token_is_read_from_its_status() {
    let shown = "gitlab.com\n  ✓ Logged in to gitlab.com as ada (keyring)\n  ✓ Git operations for gitlab.com configured to use ssh protocol.\n  ✓ API calls for gitlab.com are made over https protocol\n  ✓ REST API Endpoint: https://gitlab.com/api/v4/\n  ✓ Token: glpat-abc123\n";
    assert_eq!(gitlab::glab_token(shown).as_deref(), Some("glpat-abc123"));
    assert_eq!(gitlab::glab_token("  ✓ Token found: **************"), None);
    assert_eq!(
        gitlab::glab_token("x509: certificate signed by unknown authority"),
        None
    );
}

fn creds(server: &FakeServer, token: &str) -> Credentials {
    Credentials {
        url: server.url.clone(),
        email: "ada@atlas.dev".into(),
        token: token.into(),
        api: None,
    }
}

#[tokio::test]
async fn github_lists_the_rights_a_classic_token_lacks() {
    let server = FakeServer::start().await;
    let p = github::GithubProvider;
    // An Enterprise Server: its API under /api/v3.
    server.on_with(
        "GET",
        "/api/v3/user",
        200,
        json!({ "login": "ada" }),
        &[("X-OAuth-Scopes", "read:org, gist")],
    );
    let (status, id) = check(&p, reqwest::Client::new(), &creds(&server, "ghp")).await;
    assert_eq!(id.unwrap().label, "@ada");
    assert_eq!(status.health, Health::MissingScopes);
    assert_eq!(status.missing, ["repo"]);
    let sent = &server.requests()[0];
    assert_eq!(sent.headers["authorization"], "Bearer ghp");
    // A fine-grained token does not say its rights: nothing is said missing.
    server.on("GET", "/api/v3/user", 200, json!({ "login": "ada" }));
    let (status, _) = check(&p, reqwest::Client::new(), &creds(&server, "github_pat")).await;
    assert_eq!(status.health, Health::Connected);
    // Refused: expired or revoked.
    server.on(
        "GET",
        "/api/v3/user",
        401,
        json!({ "message": "Bad credentials" }),
    );
    let (status, id) = check(&p, reqwest::Client::new(), &creds(&server, "old")).await;
    assert!(id.is_none());
    assert_eq!(status.health, Health::Expired);
    assert_eq!(
        status.error.as_deref(),
        Some("GitHub refuse ces identifiants (401) — Bad credentials")
    );
}

#[tokio::test]
async fn gitlab_reads_the_scopes_of_its_personal_token() {
    let server = FakeServer::start().await;
    server.on("GET", "/api/v4/user", 200, json!({ "username": "ada" }));
    server.on(
        "GET",
        "/api/v4/personal_access_tokens/self",
        200,
        json!({ "scopes": ["read_api", "read_user"], "expires_at": "2027-01-01" }),
    );
    let (status, id) = check(
        &gitlab::GitlabProvider,
        reqwest::Client::new(),
        &creds(&server, "glpat"),
    )
    .await;
    assert_eq!(id.unwrap().user, "ada");
    assert_eq!(
        (status.health, status.missing),
        (Health::MissingScopes, vec!["api".to_string()])
    );
    assert!(server
        .requests()
        .iter()
        .all(|r| r.headers["private-token"] == "glpat"));
    // glab's OAuth token: no scopes to read, nothing said missing.
    server.on(
        "GET",
        "/api/v4/personal_access_tokens/self",
        404,
        Value::Null,
    );
    let (status, _) = check(
        &gitlab::GitlabProvider,
        reqwest::Client::new(),
        &creds(&server, "oauth"),
    )
    .await;
    assert_eq!(status.health, Health::Connected);
}

#[tokio::test]
async fn bitbucket_and_gitea_say_who_the_token_is() {
    let server = FakeServer::start().await;
    server.on(
        "GET",
        "/2.0/user",
        200,
        json!({ "account_id": "557058:ada", "nickname": "ada" }),
    );
    server.on("GET", "/api/v1/user", 200, json!({ "login": "ada" }));
    let (status, id) = check(
        &bitbucket::BitbucketProvider,
        reqwest::Client::new(),
        &creds(&server, "ATATT"),
    )
    .await;
    let id = id.unwrap();
    assert_eq!(
        (id.label.as_str(), id.user.as_str()),
        ("@ada", "557058:ada")
    );
    assert_eq!(status.health, Health::Connected);
    let (_, id) = check(
        &gitea::GiteaProvider,
        reqwest::Client::new(),
        &creds(&server, "gtea"),
    )
    .await;
    assert_eq!(id.unwrap().label, "@ada");
    let auth: Vec<String> = server
        .requests()
        .iter()
        .map(|r| r.headers["authorization"].clone())
        .collect();
    use base64::Engine;
    let basic = base64::engine::general_purpose::STANDARD.encode("ada@atlas.dev:ATATT");
    assert_eq!(auth, [format!("Basic {basic}"), "token gtea".to_string()]);
}

#[tokio::test]
async fn a_self_hosted_instance_says_what_it_is_when_asked() {
    let http = reqwest::Client::new();
    let r = Registry::standard();
    let gitea = FakeServer::start().await;
    gitea.on(
        "GET",
        "/api/v1/version",
        200,
        json!({ "version": "1.22.3" }),
    );
    assert_eq!(guess(&r, &http, &gitea.url).await.as_deref(), Some("gitea"));
    let gitlab = FakeServer::start().await;
    gitlab.on(
        "GET",
        "/api/v4/version",
        401,
        json!({ "message": "401 Unauthorized" }),
    );
    assert_eq!(
        guess(&r, &http, &gitlab.url).await.as_deref(),
        Some("gitlab")
    );
    let other = FakeServer::start().await;
    assert_eq!(guess(&r, &http, &other.url).await, None);
    // Its name says it: nothing asked.
    assert_eq!(
        guess(&r, &http, "gitlab.com").await.as_deref(),
        Some("gitlab")
    );
}

#[test]
fn github_coms_account_is_issues_one_and_tokens_stay_out_of_the_file() {
    let mut a = Accounts {
        github: Some(Account {
            label: "@ada · via gh".into(),
            user: "ada".into(),
            ..Default::default()
        }),
        forges: vec![ForgeAccount {
            kind: "gitlab".into(),
            url: "https://gitlab.acme.dev:8443".into(),
            token: "glpat".into(),
            label: "@ada".into(),
            ..Default::default()
        }],
        ..Default::default()
    };
    let list = accounts::all(&a);
    assert_eq!(list[0].host(), "github.com");
    assert!(list[0].via_cli);
    assert_eq!(list[0].label, "@ada");
    // A remote's ssh has no port: the same instance.
    assert_eq!(
        accounts::for_host(&list, "gitlab.acme.dev").map(|a| a.kind.as_str()),
        Some("gitlab")
    );
    let store = MemoryStore::default();
    assert!(accounts::load(&store, &mut a));
    assert_eq!(
        store.entry("forge:gitlab.acme.dev:8443").as_deref(),
        Some("glpat")
    );
    let mut saved = a.clone();
    accounts::strip(&mut saved, false);
    assert!(!serde_json::to_string(&saved).unwrap().contains("glpat"));
    // At the next start, read back from the keychain.
    let mut next = saved.clone();
    assert!(!accounts::load(&store, &mut next));
    assert_eq!(next.forges[0].token, "glpat");
    // Gone from the keychain: said unread, never sent without it.
    store.delete("forge:gitlab.acme.dev:8443").unwrap();
    let mut gone = saved;
    accounts::load(&store, &mut gone);
    assert!(gone.forges[0].unread);
}

// ---------- A fake host, plugged in without touching the core ----------

/// A code host Escouade does not know: its provider and client are all there is of it.
struct FakeHost {
    api: String,
}

struct FakeClient {
    http: reqwest::Client,
    api: String,
    token: String,
}

impl Provider for FakeHost {
    fn kind(&self) -> &'static str {
        "fake"
    }
    fn name(&self) -> &'static str {
        "Fake"
    }
    fn request_word(&self) -> &'static str {
        "CR"
    }
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            checks: true,
            merge_strategies: vec![MergeStrategy::Squash],
            ..Default::default()
        }
    }
    fn default_host(&self) -> Option<&'static str> {
        Some("fake.dev")
    }
    fn own_host(&self, host: &str) -> Option<String> {
        (host == "fake.dev").then(|| host.to_string())
    }
    fn scopes(&self) -> &'static [&'static str] {
        &["read", "write"]
    }
    fn token_page(&self, url: &str) -> String {
        format!("{url}/tokens")
    }
    fn client(&self, http: reqwest::Client, account: &Credentials) -> Box<dyn Forge> {
        Box::new(FakeClient {
            http,
            api: account.api.clone().unwrap_or_else(|| self.api.clone()),
            token: account.token.clone(),
        })
    }
}

impl Forge for FakeClient {
    fn me(&self) -> BoxFuture<'_, Result<Identity>> {
        Box::pin(async move {
            let (v, _) = call(
                "Fake",
                self.http
                    .get(format!("{}/whoami", self.api))
                    .bearer_auth(&self.token),
            )
            .await?;
            Ok(Identity {
                user: v["name"].as_str().unwrap_or("").into(),
                label: format!("~{}", v["name"].as_str().unwrap_or("")),
                scopes: Some(
                    v["rights"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|s| s.as_str().map(str::to_string))
                        .collect(),
                ),
            })
        })
    }

    fn checks<'a>(&'a self, _repo: &'a RepoRef, number: u64) -> BoxFuture<'a, Result<Vec<Check>>> {
        Box::pin(async move {
            Ok(vec![Check {
                name: format!("build #{number}"),
                state: CheckState::Success,
                url: None,
                job: None,
                ..Default::default()
            }])
        })
    }
}

#[tokio::test]
async fn a_fake_host_plugs_into_the_core_through_the_interface_alone() {
    let h = harness("forge-fake");
    let server = FakeServer::start().await;
    server.on(
        "GET",
        "/whoami",
        200,
        json!({ "name": "ada", "rights": ["read"] }),
    );
    *h.core.forges.write() = Registry::standard().with(std::sync::Arc::new(FakeHost {
        api: server.url.clone(),
    }));
    // A project whose remote is on it.
    let (p, r) = h.project(false).await;
    git(
        &r,
        &["remote", "add", "origin", "git@fake.dev:ada/notes.git"],
    );
    let remote = h.core.project_forge(&p.id).await.unwrap();
    assert_eq!(remote.kind.as_deref(), Some("fake"));
    assert_eq!(remote.repo.full(), "ada/notes");
    let view = h.core.forges_view().await;
    assert!(view
        .providers
        .iter()
        .any(|p| p.kind == "fake" && p.request_word == "CR"));
    assert_eq!(view.unknown.len(), 1);
    assert_eq!(view.unknown[0].host, "fake.dev");
    assert_eq!(view.unknown[0].url, "https://fake.dev");
    assert_eq!(view.unknown[0].projects, [p.name.as_str()]);
    // Connected: its token in the keychain, the rights it lacks listed, the project on it.
    let view = h
        .core
        .forge_connect(ForgeForm {
            kind: "fake".into(),
            token: "tok".into(),
            ..Default::default()
        })
        .await
        .unwrap();
    let a = &view.accounts[0];
    assert_eq!((a.host.as_str(), a.label.as_str()), ("fake.dev", "~ada"));
    assert_eq!(a.status.health, Health::MissingScopes);
    assert_eq!(a.status.missing, ["write"]);
    assert_eq!(a.projects, [p.name.as_str()]);
    assert!(view.unknown.is_empty());
    let store = crate::integrations::secrets::memory_of(&h.core.data);
    assert_eq!(store.entry("forge:fake.dev").as_deref(), Some("tok"));
    assert!(!std::fs::read_to_string(h.core.data.integrations_file())
        .unwrap()
        .contains("\"tok\""));
    // Its client answers what it declares, and says what it cannot do.
    let (client, _) = h.core.forge_client("fake.dev").await.unwrap();
    let repo = remote.repo.clone();
    assert_eq!(client.checks(&repo, 7).await.unwrap()[0].name, "build #7");
    let e = client
        .merge(&repo, 7, MergeStrategy::Squash)
        .await
        .unwrap_err();
    assert!(e.downcast_ref::<Unsupported>().is_some(), "{e}");
    // The token expires: the check says so.
    server.on("GET", "/whoami", 401, Value::Null);
    let view = h.core.forge_check().await;
    assert_eq!(view.accounts[0].status.health, Health::Expired);
    // Disconnected: gone, its token too, and its host back to « Connecter ».
    let view = h.core.forge_disconnect("fake.dev").await.unwrap();
    assert!(view.accounts.is_empty());
    assert_eq!(store.entry("forge:fake.dev"), None);
    assert_eq!(view.unknown[0].host, "fake.dev");
}

#[tokio::test]
async fn github_com_is_one_account_for_issues_and_pull_requests() {
    let h = harness("forge-github");
    let server = FakeServer::start().await;
    h.core.bases.write().github = server.url.clone();
    server.on_with(
        "GET",
        "/user",
        200,
        json!({ "login": "ada" }),
        &[("X-OAuth-Scopes", "repo, read:org")],
    );
    // Connected as a code host: Issues has it too.
    let view = h
        .core
        .forge_connect(ForgeForm {
            kind: "github".into(),
            token: "ghp".into(),
            ..Default::default()
        })
        .await
        .unwrap();
    let a = &view.accounts[0];
    assert_eq!((a.host.as_str(), a.issues), ("github.com", true));
    assert_eq!(a.status.health, Health::Connected);
    assert!(h.core.integration_accounts()[2].connected);
    // Several per host: another instance beside it.
    let ghes = FakeServer::start().await;
    ghes.on("GET", "/api/v3/user", 200, json!({ "login": "ada-corp" }));
    let view = h
        .core
        .forge_connect(ForgeForm {
            kind: "github".into(),
            url: ghes.url.clone(),
            token: "ghp_corp".into(),
            ..Default::default()
        })
        .await
        .unwrap();
    let labels: Vec<&str> = view.accounts.iter().map(|a| a.label.as_str()).collect();
    assert_eq!(labels, ["@ada", "@ada-corp"]);
    // Disconnected from Issues: gone as a code host too.
    h.core
        .integration_disconnect(crate::model::Service::Github)
        .unwrap();
    let view = h.core.forges_view().await;
    assert_eq!(view.accounts.len(), 1);
    assert_eq!(view.accounts[0].label, "@ada-corp");
    // Never over http to another machine.
    let e = h
        .core
        .forge_connect(ForgeForm {
            kind: "gitea".into(),
            url: "http://git.acme.dev".into(),
            token: "t".into(),
            ..Default::default()
        })
        .await
        .unwrap_err();
    assert_eq!(
        e.to_string(),
        "L'adresse doit être en https (http seulement sur cette machine)."
    );
    let e = h
        .core
        .forge_connect(ForgeForm {
            kind: "bitbucket".into(),
            token: "t".into(),
            ..Default::default()
        })
        .await
        .unwrap_err();
    assert_eq!(e.to_string(), "Indique l'e-mail.");
}
