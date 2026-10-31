//! External ticket systems (Jira, Trello, GitHub Issues, Linear, ClickUp, GitLab Issues, Azure Boards): their accounts, the tickets a project
//! imports from them, and the sync of their states with the Kanban's columns.

pub mod azure;
pub mod clickup;
#[cfg(test)]
pub(crate) mod fake;
pub mod github;
pub mod gitlab;
pub mod jira;
pub mod linear;
pub mod secrets;
pub mod source;
pub mod sync;
pub mod text;
pub mod trello;

use crate::model::*;
use anyhow::{anyhow, bail, Result};
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// The accounts as saved (`integrations.json`, apart from the settings; their secrets in the
/// system's keychain, `secrets`): their secrets never go to the window.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Accounts {
    pub jira: Option<Account>,
    pub trello: Option<Account>,
    pub github: Option<Account>,
    pub linear: Option<Account>,
    pub clickup: Option<Account>,
    /// The code hosts' accounts but github.com's (`github`): one per instance
    /// (`crate::forge::accounts`).
    pub forges: Vec<crate::forge::accounts::ForgeAccount>,
    /// The hosts whose token a « Déconnecter » could not take out of the keychain: taken out at
    /// the next start, unless the instance has a token of its own again.
    pub forget_forges: Vec<String>,
    /// The services whose entry in the system's keychain a « Déconnecter » (or a switch to the
    /// GitHub CLI's token) could not delete: deleted at the next start, unless the service has a
    /// token of its own again. Their names only, never a secret.
    #[serde(deserialize_with = "known_services")]
    pub to_forget: Vec<Service>,
    /// The file keeps a copy of the secrets the keychain holds (`secrets::file_keeps_copy`).
    /// Known while the app runs, never saved.
    #[serde(skip)]
    pub file_copy: bool,
}

impl Accounts {
    pub fn get(&self, s: Service) -> Option<&Account> {
        match s {
            Service::JIRA => self.jira.as_ref(),
            Service::TRELLO => self.trello.as_ref(),
            Service::GITHUB => self.github.as_ref(),
            Service::LINEAR => self.linear.as_ref(),
            Service::CLICKUP => self.clickup.as_ref(),
            // The GitLab and Azure DevOps code hosts' accounts (`hosted_accounts`).
            _ => None,
        }
    }

    pub fn get_mut(&mut self, s: Service) -> Option<&mut Account> {
        match s {
            Service::JIRA => self.jira.as_mut(),
            Service::TRELLO => self.trello.as_mut(),
            Service::GITHUB => self.github.as_mut(),
            Service::LINEAR => self.linear.as_mut(),
            Service::CLICKUP => self.clickup.as_mut(),
            _ => None,
        }
    }

    pub fn set(&mut self, s: Service, a: Option<Account>) {
        match s {
            Service::JIRA => self.jira = a,
            Service::TRELLO => self.trello = a,
            Service::GITHUB => self.github = a,
            Service::LINEAR => self.linear = a,
            Service::CLICKUP => self.clickup = a,
            // Connected and disconnected as a code host.
            _ => {}
        }
    }

    /// The GitLab code host's accounts, one per instance: GitLab Issues' too.
    pub fn gitlab_accounts(&self) -> Vec<&crate::forge::accounts::ForgeAccount> {
        self.hosted_accounts(Service::GITLAB)
    }

    /// The code host's accounts a source has as its own (GitLab Issues: GitLab's; Azure Boards:
    /// Azure DevOps', one per organization); none for the others.
    pub fn hosted_accounts(&self, s: Service) -> Vec<&crate::forge::accounts::ForgeAccount> {
        let Some(kind) = hosted_kind(s) else {
            return Vec::new();
        };
        self.forges.iter().filter(|a| a.kind == kind).collect()
    }

    /// An account of `s` is connected (for GitLab and Azure Boards, a code host's one).
    pub fn connected(&self, s: Service) -> bool {
        match hosted_kind(s) {
            Some(_) => !self.hosted_accounts(s).is_empty(),
            None => self.get(s).is_some(),
        }
    }

    /// What the window knows of them.
    pub fn views(&self) -> Vec<AccountView> {
        source::registry()
            .services()
            .iter()
            .map(|&service| match hosted_kind(service) {
                Some(_) => self.hosted_view(service),
                None => AccountView {
                    service,
                    connected: self.get(service).is_some(),
                    label: self
                        .get(service)
                        .map(|a| a.label.clone())
                        .unwrap_or_default(),
                    in_file: self.get(service).is_some_and(|a| a.in_file),
                    unread: self.get(service).is_some_and(|a| a.unread),
                },
            })
            .collect()
    }

    /// GitLab Issues' and Azure Boards' view: their code host's accounts, their instance named
    /// but gitlab.com's (an Azure DevOps organization, by its address's end).
    fn hosted_view(&self, service: Service) -> AccountView {
        let list = self.hosted_accounts(service);
        let label = list
            .iter()
            .map(|a| match a.host().as_str() {
                "gitlab.com" => a.label.clone(),
                crate::forge::azure::CLOUD => format!(
                    "{} · {}",
                    a.label,
                    a.url.trim_end_matches('/').rsplit('/').next().unwrap_or("")
                ),
                host => format!("{} · {host}", a.label),
            })
            .collect::<Vec<_>>()
            .join(", ");
        AccountView {
            service,
            connected: !list.is_empty(),
            label,
            in_file: list.iter().any(|a| a.in_file),
            unread: !list.is_empty() && list.iter().all(|a| a.unread),
        }
    }
}

/// The code host whose accounts are a source's own: GitLab Issues', Azure Boards'.
pub fn hosted_kind(s: Service) -> Option<&'static str> {
    let kind = source::registry().get(s)?;
    kind.host().filter(|_| !kind.own_account())
}

/// The services of the list this version knows: one a later version added (the app rolled back)
/// is left out, rather than the whole file and every account with it.
fn known_services<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<Service>, D::Error> {
    let names: Vec<Value> = Vec::deserialize(d)?;
    Ok(names
        .into_iter()
        .filter_map(|n| serde_json::from_value::<Service>(n).ok())
        .filter(|s| source::registry().get(*s).is_some())
        .collect())
}

/// An account: what its form gave ("Connecter…"), then what its check found.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Account {
    /// Jira: the site's address (`https://atlas.atlassian.net`).
    pub site: String,
    /// Jira: the e-mail its API token belongs to.
    pub email: String,
    /// Trello: the API key. A secret, as the token is.
    pub key: String,
    /// The API token (the personal API key of Linear, of ClickUp); for GitHub, empty: the GitHub CLI's
    /// (`gh auth token`).
    pub token: String,
    /// As the window shows it: "ada@atlas.dev · atlas.atlassian.net", "@ada".
    pub label: String,
    /// Who it is for the service (Jira account id, Trello member id, GitHub login): what "mine"
    /// filters on.
    pub user: String,
    /// The system's keychain refused its key and token: they stay in `integrations.json` (they
    /// move at the next start that it takes them). Known while the app runs, never saved.
    #[serde(skip)]
    pub in_file: bool,
    /// The system's keychain holds its key and token, and did not give them at this start (out of
    /// reach, refused, the entry gone or unreadable): no call goes out without them. Known while
    /// the app runs, never saved.
    #[serde(skip)]
    pub unread: bool,
}

/// What `integration_connect` adds to the label of a GitHub account without a token of its own:
/// it uses the CLI's.
pub(crate) const VIA_GH: &str = " · via gh";

/// An account as the window knows it.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AccountView {
    pub service: Service,
    pub connected: bool,
    pub label: String,
    /// Its token stays in `integrations.json`: the system's keychain refused it.
    pub in_file: bool,
    /// The system's keychain did not give its token at this start (`Account::unread`).
    pub unread: bool,
}

/// Where an account's tickets live: a Jira project, a Trello board, a GitHub repository, a Linear
/// team, a ClickUp list.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Container {
    pub id: String,
    pub name: String,
    /// Where it is in its service's hierarchy, as the window groups it: a ClickUp list's
    /// « Space › Folder ». None for the others.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// The project's own repository (its remote's), which the import offers to link at once.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub own: bool,
}

/// The states of a container, and those each column gives by default.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StatesView {
    pub states: Vec<ExternalState>,
    pub defaults: BTreeMap<Column, ExternalState>,
}

/// A filter of the import modal, shown as a chip.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IssueFilter {
    pub id: String,
    pub label: String,
}

/// What the import asks of a source.
#[derive(Debug, Clone, Default)]
pub struct Query {
    /// Typed in the search field.
    pub text: String,
    /// The ids of the chips turned on (`IssueFilter::id`).
    pub filters: Vec<String>,
    /// Only the tickets that carry this label (automatic import).
    pub label: Option<String>,
    /// The page after the first one (« Afficher plus »): `IssuePage::next` of the one before.
    pub page: Option<String>,
}

/// An external ticket as the import modal lists it, and as it is imported.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ExternalIssue {
    pub service: Service,
    /// What its API knows it by (`ExternalRef::id`).
    pub id: String,
    /// As shown: ATL-1287, #142, #42.
    pub key: String,
    pub title: String,
    /// As shown, in the interface's language for the names Escouade knows: Story, Bug, Tâche ;
    /// Carte ; Issue.
    pub kind: String,
    /// What the window colours it by, the same in every language: `task`, `subtask`, `bug`,
    /// `story`, `epic` (Jira), `card` (Trello), `issue` (GitHub); empty for a type it does not
    /// know (one a Jira was set up with).
    pub kind_code: String,
    /// Its priority, assignee and state, those it has.
    pub meta: Vec<String>,
    pub url: String,
    /// As markdown.
    pub description: String,
    /// Those its description or checklists list.
    pub criteria: Vec<String>,
    pub container: String,
    /// Already on the project's Kanban.
    pub imported: bool,
}

/// A page of a source's tickets for the import modal, and the chips that filter them.
#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct IssuePage {
    pub issues: Vec<ExternalIssue>,
    pub filters: Vec<IssueFilter>,
    /// What asks for the next page (`Query::page`), when the service has more.
    pub next: Option<String>,
    /// How many tickets match in all, when the service says so.
    pub total: Option<u64>,
}

/// Where the Trello, GitHub, Linear and ClickUp APIs are (Jira's is the account's site);
/// `ESCOUADE_TRELLO_API`, `ESCOUADE_GITHUB_API`, `ESCOUADE_LINEAR_API` and `ESCOUADE_CLICKUP_API`
/// put a fake one in their place (tests, demos).
#[derive(Debug, Clone, PartialEq)]
pub struct Bases {
    pub trello: String,
    pub github: String,
    pub linear: String,
    pub clickup: String,
}

impl Bases {
    pub fn from_env() -> Self {
        let var = |k: &str, default: &str| {
            std::env::var(k)
                .ok()
                .filter(|v| !v.trim().is_empty())
                .unwrap_or_else(|| default.to_string())
                .trim_end_matches('/')
                .to_string()
        };
        Self {
            trello: var("ESCOUADE_TRELLO_API", "https://api.trello.com/1"),
            github: var("ESCOUADE_GITHUB_API", "https://api.github.com"),
            linear: var("ESCOUADE_LINEAR_API", "https://api.linear.app"),
            clickup: var("ESCOUADE_CLICKUP_API", "https://api.clickup.com/api/v2"),
        }
    }
}

/// Sends a request to `service` and reads its JSON answer (null when empty); an error says, in
/// French, what went wrong and what the service said.
pub(crate) async fn call(service: Service, rb: reqwest::RequestBuilder) -> Result<Value> {
    Ok(send(service, rb).await?.0)
}

/// `call`, and whether the answer's `Link` header points to a next page (GitHub's lists).
pub(crate) async fn call_page(
    service: Service,
    rb: reqwest::RequestBuilder,
) -> Result<(Value, bool)> {
    let (v, headers) = send(service, rb).await?;
    let next = headers
        .get_all(reqwest::header::LINK)
        .iter()
        .filter_map(|h| h.to_str().ok())
        .flat_map(|h| h.split(','))
        .any(|link| link.contains("rel=\"next\""));
    Ok((v, next))
}

async fn send(service: Service, rb: reqwest::RequestBuilder) -> Result<(Value, HeaderMap)> {
    send_named(service.label(), rb).await
}

/// `send` for a service known by its name only (a code host).
pub(crate) async fn send_named(
    name: &str,
    rb: reqwest::RequestBuilder,
) -> Result<(Value, HeaderMap)> {
    let resp = rb.send().await.map_err(|e| {
        let e = anyhow::Error::from(e.without_url());
        anyhow!(tr!(
            "{name} injoignable : {e:#}",
            "{name} can’t be reached: {e:#}"
        ))
    })?;
    let status = resp.status().as_u16();
    let headers = resp.headers().clone();
    let text = resp.text().await.unwrap_or_default();
    if !(200..300).contains(&status) {
        return Err(HttpError {
            status,
            message: http_error_in(crate::i18n::ui(), name, status, &text),
        }
        .into());
    }
    if text.trim().is_empty() {
        return Ok((Value::Null, headers));
    }
    let v = serde_json::from_str(&text).map_err(|_| {
        anyhow!(tr!(
            "{name} : réponse illisible",
            "{name}: unreadable answer"
        ))
    })?;
    Ok((v, headers))
}

/// A service's answer other than a success: its status, and what `http_error` says of it (the
/// error's text).
#[derive(Debug)]
pub struct HttpError {
    pub status: u16,
    pub message: String,
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for HttpError {}

/// A refusal that asking again would not change (Jira: no transition leads from where the issue
/// is to the state).
#[derive(Debug)]
pub struct Refused(pub String);

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Refused {}

/// The failure stays however often the call is made again: a request refused as it is (400, 422),
/// a ticket gone (404, 410), no transition to the state. Tried again later, it would only hold
/// back its ticket's next syncs. A service out of reach, too slow, down (5xx) or busy (429), and
/// credentials refused (401, 403: they can be put right), may pass later.
pub(crate) fn lasting(e: &anyhow::Error) -> bool {
    e.downcast_ref::<Refused>().is_some()
        || e.downcast_ref::<HttpError>()
            .is_some_and(|h| matches!(h.status, 400 | 404 | 410 | 422))
}

/// An HTTP error of `service`, and what its body says.
pub fn http_error(service: Service, status: u16, body: &str) -> String {
    http_error_in(crate::i18n::ui(), service.label(), status, body)
}

pub(crate) fn http_error_in(
    lang: crate::i18n::Lang,
    name: &str,
    status: u16,
    body: &str,
) -> String {
    let tail = said(body).map(|m| format!(" — {m}")).unwrap_or_default();
    match status {
        401 => tr_in!(
            lang,
            "{name} refuse ces identifiants (401){tail}",
            "{name} refuses these credentials (401){tail}"
        ),
        403 => tr_in!(
            lang,
            "{name} refuse l'accès (403){tail}",
            "{name} denies access (403){tail}"
        ),
        404 => tr_in!(
            lang,
            "{name} : introuvable (404){tail}",
            "{name}: not found (404){tail}"
        ),
        429 => tr_in!(
            lang,
            "{name} limite les requêtes (429) : réessaie dans un moment",
            "{name} limits the requests (429): try again in a moment"
        ),
        _ => tr_in!(
            lang,
            "{name} : erreur {status}{tail}",
            "{name}: error {status}{tail}"
        ),
    }
}

/// The message of an error's body: Jira's `errorMessages` or `errors`, the first of GraphQL's
/// `errors` (Linear), GitHub's `message`, ClickUp's `err`, or Trello's text.
fn said(body: &str) -> Option<String> {
    let body = body.trim();
    let msg = match serde_json::from_str::<Value>(body) {
        Ok(Value::String(s)) => s,
        Ok(v) => v["errorMessages"][0]
            .as_str()
            .map(str::to_string)
            .or_else(|| {
                v["errors"]
                    .as_object()
                    .and_then(|o| o.values().find_map(|e| e.as_str().map(str::to_string)))
            })
            .or_else(|| v["errors"][0]["message"].as_str().map(str::to_string))
            .or_else(|| v["message"].as_str().map(str::to_string))
            .or_else(|| v["error"].as_str().map(str::to_string))
            .or_else(|| v["err"].as_str().map(str::to_string))?,
        Err(_) if body.starts_with('<') => return None,
        Err(_) => body.lines().next()?.to_string(),
    };
    let msg = msg.trim();
    (!msg.is_empty()).then(|| crate::board::first_line(msg))
}

/// A path segment or a query value, percent-encoded.
pub(crate) fn encode(s: &str) -> String {
    crate::board::encode(s, false)
}

/// A project's links as the window sends them: one per service, each with a container, and each
/// column commented once.
pub fn checked_links(mut p: ProjectIntegrations) -> ProjectIntegrations {
    let mut seen = Vec::new();
    p.links.retain(|l| {
        let keep = !l.container.trim().is_empty() && !seen.contains(&l.service);
        if keep {
            seen.push(l.service);
        }
        keep
    });
    let mut columns = Vec::new();
    for c in p.comments {
        if !columns.contains(&c) {
            columns.push(c);
        }
    }
    p.comments = columns;
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_service_refusing_says_so_in_english() {
        use crate::i18n::Lang::En;
        assert_eq!(
            [
                http_error_in(En, "Jira", 401, ""),
                http_error_in(En, "GitHub", 403, r#"{"message":"Bad credentials"}"#),
                http_error_in(En, "Trello", 404, "model not found\n"),
                http_error_in(En, "Trello", 429, ""),
                http_error_in(En, "GitHub", 502, "<html>Bad gateway</html>"),
            ],
            [
                "Jira refuses these credentials (401)",
                "GitHub denies access (403) — Bad credentials",
                "Trello: not found (404) — model not found",
                "Trello limits the requests (429): try again in a moment",
                "GitHub: error 502",
            ]
        );
    }

    #[tokio::test]
    async fn a_refusal_that_asking_again_would_not_change_is_told_apart() {
        let server = fake::FakeServer::start().await;
        let http = reqwest::Client::new();
        for (status, lasts) in [
            (400, true),
            (404, true),
            (410, true),
            (422, true),
            (401, false),
            (403, false),
            (429, false),
            (500, false),
            (503, false),
        ] {
            server.on("GET", "/x", status, Value::Null);
            let e = call(Service::GITHUB, http.get(format!("{}/x", server.url)))
                .await
                .unwrap_err();
            assert_eq!(lasting(&e), lasts, "{status}");
            // It still reads as before.
            assert_eq!(e.to_string(), http_error(Service::GITHUB, status, ""));
        }
        // Not reached: it may be, later.
        let e = call(Service::GITHUB, http.get("http://"))
            .await
            .unwrap_err();
        assert!(e.to_string().starts_with("GitHub injoignable"), "{e}");
        assert!(!lasting(&e));
        assert!(lasting(&anyhow::Error::from(Refused("non".into()))));
        assert!(!lasting(&anyhow!("Aucun compte GitHub connecté")));
    }

    #[test]
    fn an_http_error_says_what_the_service_said() {
        assert_eq!(
            http_error(Service::JIRA, 401, ""),
            "Jira refuse ces identifiants (401)"
        );
        assert_eq!(
            http_error(
                Service::JIRA,
                400,
                r#"{"errorMessages":["Le champ 'sprint' n'existe pas."],"errors":{}}"#
            ),
            "Jira : erreur 400 — Le champ 'sprint' n'existe pas."
        );
        assert_eq!(
            http_error(
                Service::JIRA,
                400,
                r#"{"errorMessages":[],"errors":{"jql":"JQL invalide"}}"#
            ),
            "Jira : erreur 400 — JQL invalide"
        );
        assert_eq!(
            http_error(
                Service::GITHUB,
                403,
                r#"{"message":"Resource not accessible by personal access token"}"#
            ),
            "GitHub refuse l'accès (403) — Resource not accessible by personal access token"
        );
        assert_eq!(
            http_error(Service::TRELLO, 404, "model not found\n"),
            "Trello : introuvable (404) — model not found"
        );
        assert_eq!(
            http_error(Service::GITHUB, 502, "<html>Bad gateway</html>"),
            "GitHub : erreur 502"
        );
        assert!(http_error(Service::TRELLO, 429, "").contains("réessaie"));
    }

    #[test]
    fn segments_are_percent_encoded() {
        assert_eq!(encode("in progress/à"), "in%20progress%2F%C3%A0");
        assert_eq!(encode("claude-ready_1.0~"), "claude-ready_1.0~");
    }

    #[test]
    fn links_are_one_per_service_with_a_container() {
        let link = |s: Service, c: &str| SourceLink {
            service: s,
            container: c.into(),
            ..Default::default()
        };
        let p = checked_links(ProjectIntegrations {
            links: vec![
                link(Service::JIRA, "ATL"),
                link(Service::TRELLO, " "),
                link(Service::JIRA, "MOB"),
                link(Service::TRELLO, "b1"),
                link(Service::GITHUB, "acme/api"),
            ],
            comments: vec![Column::Done, Column::Review, Column::Done],
            ..Default::default()
        });
        let kept: Vec<&str> = p.links.iter().map(|l| l.container.as_str()).collect();
        assert_eq!(kept, ["ATL", "b1", "acme/api"]);
        assert_eq!(p.comments, [Column::Done, Column::Review]);
    }

    #[test]
    fn the_window_sees_who_is_connected_and_never_a_secret() {
        let mut a = Accounts::default();
        a.set(
            Service::TRELLO,
            Some(Account {
                key: "k".into(),
                token: "secret".into(),
                label: "@ada".into(),
                ..Default::default()
            }),
        );
        let v = serde_json::to_string(&a.views()).unwrap();
        assert!(!v.contains("secret"));
        let views = a.views();
        assert_eq!(views.len(), 7);
        assert!(!views[0].connected);
        assert_eq!(
            (views[1].connected, views[1].label.as_str()),
            (true, "@ada")
        );
    }
}
