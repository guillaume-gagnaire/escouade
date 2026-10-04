//! External ticket systems (Jira, Trello, GitHub Issues): their accounts, the tickets a project
//! imports from them, and the sync of their states with the Kanban's columns.

#[cfg(test)]
pub(crate) mod fake;
pub mod github;
pub mod jira;
pub mod sync;
pub mod text;
pub mod trello;

use crate::model::*;
use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// The accounts as saved (`integrations.json`, apart from the settings): their secrets never go
/// to the window.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Accounts {
    pub jira: Option<Account>,
    pub trello: Option<Account>,
    pub github: Option<Account>,
}

impl Accounts {
    pub fn get(&self, s: Service) -> Option<&Account> {
        match s {
            Service::Jira => self.jira.as_ref(),
            Service::Trello => self.trello.as_ref(),
            Service::Github => self.github.as_ref(),
        }
    }

    pub fn set(&mut self, s: Service, a: Option<Account>) {
        match s {
            Service::Jira => self.jira = a,
            Service::Trello => self.trello = a,
            Service::Github => self.github = a,
        }
    }

    /// What the window knows of them.
    pub fn views(&self) -> Vec<AccountView> {
        Service::ALL
            .iter()
            .map(|&service| AccountView {
                service,
                connected: self.get(service).is_some(),
                label: self
                    .get(service)
                    .map(|a| a.label.clone())
                    .unwrap_or_default(),
            })
            .collect()
    }
}

/// An account: what its form gave ("Connecter…"), then what its check found.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Account {
    /// Jira: the site's address (`https://atlas.atlassian.net`).
    pub site: String,
    /// Jira: the e-mail its API token belongs to.
    pub email: String,
    /// Trello: the API key.
    pub key: String,
    /// The API token; for GitHub, empty: the GitHub CLI's (`gh auth token`).
    pub token: String,
    /// As the window shows it: "ada@atlas.dev · atlas.atlassian.net", "@ada".
    pub label: String,
    /// Who it is for the service (Jira account id, Trello member id, GitHub login): what "mine"
    /// filters on.
    pub user: String,
}

/// An account as the window knows it.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AccountView {
    pub service: Service,
    pub connected: bool,
    pub label: String,
}

/// Where an account's tickets live: a Jira project, a Trello board, a GitHub repository.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Container {
    pub id: String,
    pub name: String,
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
    /// Story, Bug, Tâche ; Carte ; Issue.
    pub kind: String,
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

/// A source's tickets for the import modal, and the chips that filter them.
#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct IssuePage {
    pub issues: Vec<ExternalIssue>,
    pub filters: Vec<IssueFilter>,
}

/// Where the Trello and GitHub APIs are (Jira's is the account's site); `ESCOUADE_TRELLO_API`
/// and `ESCOUADE_GITHUB_API` put a fake one in their place (tests, demos).
#[derive(Debug, Clone, PartialEq)]
pub struct Bases {
    pub trello: String,
    pub github: String,
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
        }
    }
}

/// What talks to one service for one account.
pub enum Client {
    Jira(jira::Jira),
    Trello(trello::Trello),
    Github(github::Github),
}

impl Client {
    /// For GitHub, `account.token` must be the token itself (the CLI's already asked for).
    pub fn new(service: Service, account: &Account, http: reqwest::Client, bases: &Bases) -> Self {
        match service {
            Service::Jira => Client::Jira(jira::Jira::new(http, account)),
            Service::Trello => Client::Trello(trello::Trello::new(http, account, &bases.trello)),
            Service::Github => Client::Github(github::Github::new(http, account, &bases.github)),
        }
    }

    /// The account's label and user, once its credentials are accepted.
    pub async fn me(&self) -> Result<(String, String)> {
        match self {
            Client::Jira(c) => c.me().await,
            Client::Trello(c) => c.me().await,
            Client::Github(c) => c.me().await,
        }
    }

    pub async fn containers(&self) -> Result<Vec<Container>> {
        match self {
            Client::Jira(c) => c.containers().await,
            Client::Trello(c) => c.containers().await,
            Client::Github(c) => c.containers().await,
        }
    }

    pub async fn states(&self, container: &str) -> Result<Vec<ExternalState>> {
        match self {
            Client::Jira(c) => c.states(container).await,
            Client::Trello(c) => c.states(container).await,
            Client::Github(c) => c.states(container).await,
        }
    }

    pub async fn issues(&self, container: &str, q: &Query) -> Result<IssuePage> {
        match self {
            Client::Jira(c) => c.issues(container, q).await,
            Client::Trello(c) => c.issues(container, q).await,
            Client::Github(c) => c.issues(container, q).await,
        }
    }

    /// Gives the external ticket `state`; `mapped`: every state the link gives a column (GitHub
    /// takes the labels of the others off).
    pub async fn set_state(
        &self,
        r: &ExternalRef,
        state: &ExternalState,
        mapped: &[ExternalState],
    ) -> Result<()> {
        match self {
            Client::Jira(c) => c.set_state(r, state).await,
            Client::Trello(c) => c.set_state(r, state).await,
            Client::Github(c) => c.set_state(r, state, mapped).await,
        }
    }

    pub async fn comment(&self, r: &ExternalRef, text: &str) -> Result<()> {
        match self {
            Client::Jira(c) => c.comment(r, text).await,
            Client::Trello(c) => c.comment(r, text).await,
            Client::Github(c) => c.comment(r, text).await,
        }
    }
}

/// Sends a request to `service` and reads its JSON answer (null when empty); an error says, in
/// French, what went wrong and what the service said.
pub(crate) async fn call(service: Service, rb: reqwest::RequestBuilder) -> Result<Value> {
    let resp = rb.send().await.map_err(|e| {
        anyhow!(
            "{} injoignable : {:#}",
            service.label(),
            anyhow::Error::from(e.without_url())
        )
    })?;
    let status = resp.status().as_u16();
    let text = resp.text().await.unwrap_or_default();
    if !(200..300).contains(&status) {
        bail!(http_error(service, status, &text));
    }
    if text.trim().is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(&text).map_err(|_| anyhow!("{} : réponse illisible", service.label()))
}

/// An HTTP error of `service`, and what its body says.
pub fn http_error(service: Service, status: u16, body: &str) -> String {
    let name = service.label();
    let tail = said(body).map(|m| format!(" — {m}")).unwrap_or_default();
    match status {
        401 => format!("{name} refuse ces identifiants (401){tail}"),
        403 => format!("{name} refuse l'accès (403){tail}"),
        404 => format!("{name} : introuvable (404){tail}"),
        429 => format!("{name} limite les requêtes (429) : réessaie dans un moment"),
        _ => format!("{name} : erreur {status}{tail}"),
    }
}

/// The message of an error's body: Jira's `errorMessages` or `errors`, GitHub's `message`, or
/// Trello's text.
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
            .or_else(|| v["message"].as_str().map(str::to_string))
            .or_else(|| v["error"].as_str().map(str::to_string))?,
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
    fn an_http_error_says_what_the_service_said() {
        assert_eq!(
            http_error(Service::Jira, 401, ""),
            "Jira refuse ces identifiants (401)"
        );
        assert_eq!(
            http_error(
                Service::Jira,
                400,
                r#"{"errorMessages":["Le champ 'sprint' n'existe pas."],"errors":{}}"#
            ),
            "Jira : erreur 400 — Le champ 'sprint' n'existe pas."
        );
        assert_eq!(
            http_error(
                Service::Jira,
                400,
                r#"{"errorMessages":[],"errors":{"jql":"JQL invalide"}}"#
            ),
            "Jira : erreur 400 — JQL invalide"
        );
        assert_eq!(
            http_error(
                Service::Github,
                403,
                r#"{"message":"Resource not accessible by personal access token"}"#
            ),
            "GitHub refuse l'accès (403) — Resource not accessible by personal access token"
        );
        assert_eq!(
            http_error(Service::Trello, 404, "model not found\n"),
            "Trello : introuvable (404) — model not found"
        );
        assert_eq!(
            http_error(Service::Github, 502, "<html>Bad gateway</html>"),
            "GitHub : erreur 502"
        );
        assert!(http_error(Service::Trello, 429, "").contains("réessaie"));
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
                link(Service::Jira, "ATL"),
                link(Service::Trello, " "),
                link(Service::Jira, "MOB"),
                link(Service::Trello, "b1"),
                link(Service::Github, "acme/api"),
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
            Service::Trello,
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
        assert_eq!(views.len(), 3);
        assert!(!views[0].connected);
        assert_eq!(
            (views[1].connected, views[1].label.as_str()),
            (true, "@ada")
        );
    }
}
