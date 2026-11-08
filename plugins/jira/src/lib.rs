//! Jira Cloud (REST API v3), with an e-mail and an API token: the Kanban's source of Jira issues, a project's statuses
//! as the columns' states. The account's secret is the API token (`secrets/get` of `account:jira`); its site and its
//! e-mail are the account's fields, which Escouade tells.

mod adf;

use adf::{adf_to_markdown, markdown_to_adf};
use anyhow::Result;
use base64::Engine;
use escouade_plugin_kit::errors::{HttpError, Refused};
use escouade_plugin_kit::forge::BoxFuture;
use escouade_plugin_kit::hosts::{Credentials, SourceKind, TicketSource};
use escouade_plugin_kit::http::reqwest;
use escouade_plugin_kit::http::{self, encode};
use escouade_plugin_kit::i18n::{self, Lang};
use escouade_plugin_kit::protocol::tickets::{
    Container, ExternalIssue, ExternalRef, ExternalState, IssueFilter, IssuePage, Query,
};
use escouade_plugin_kit::text::criteria_of;
use escouade_plugin_kit::{tr, tr_in};
use serde_json::{json, Value};

/// The source's id, and its name in what it says.
pub const ID: &str = "jira";
const S: &str = "Jira";

pub struct Jira {
    http: reqwest::Client,
    site: String,
    auth: String,
}

/// A site's address as typed ("atlas.atlassian.net/"), as the API is called on.
fn site(typed: &str) -> String {
    let s = typed.trim().trim_end_matches('/');
    if s.is_empty() || s.contains("://") {
        s.to_string()
    } else {
        format!("https://{s}")
    }
}

/// A Jira key (ATL-1287).
fn is_key(s: &str) -> bool {
    let Some((project, n)) = s.split_once('-') else {
        return false;
    };
    project
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic())
        && project
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !n.is_empty()
        && n.chars().all(|c| c.is_ascii_digit())
}

/// A string inside a JQL literal.
fn quoted(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Text searched with `text ~`: Lucene's own signs would make it a query of its own.
fn words(s: &str) -> String {
    s.chars()
        .map(|c| {
            if "+-&|!(){}[]^~*?\\:\"/".contains(c) {
                ' '
            } else {
                c
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// The kind of an issue as the import shows it: Jira's own English names in French, as they are in
/// English (and the names of a Jira set up in another language as it writes them).
fn kind_label(lang: Lang, name: &str) -> String {
    match (lang, name) {
        (Lang::Fr, "Task") => "Tâche".into(),
        (Lang::Fr, "Sub-task" | "Subtask") => "Sous-tâche".into(),
        (_, other) => other.into(),
    }
}

/// The code of an issue's type (`ExternalIssue::kind_code`): Jira's names in English, and those of
/// a Jira set up in French.
fn kind_code(name: &str) -> &'static str {
    match name {
        "Task" | "Tâche" => "task",
        "Sub-task" | "Subtask" | "Sous-tâche" => "subtask",
        "Bug" => "bug",
        "Story" => "story",
        "Epic" | "Épopée" => "epic",
        _ => "",
    }
}

/// The priority of an issue as the import shows it (see `kind_label`).
fn priority_label(lang: Lang, name: &str) -> String {
    match (lang, name) {
        (Lang::Fr, "Highest") => "Très haute".into(),
        (Lang::Fr, "High") => "Haute".into(),
        (Lang::Fr, "Medium") => "Moyenne".into(),
        (Lang::Fr, "Low") => "Basse".into(),
        (Lang::Fr, "Lowest") => "Très basse".into(),
        (_, other) => other.into(),
    }
}

/// The filters the import offers on a Jira project.
fn filters(lang: Lang) -> Vec<IssueFilter> {
    [
        ("mine", tr_in!(lang, "Assignés à moi", "Assigned to me")),
        ("sprint", tr_in!(lang, "Sprint actif", "Active sprint")),
        ("todo", tr_in!(lang, "À faire", "To do")),
    ]
    .into_iter()
    .map(|(id, label)| IssueFilter {
        id: id.to_string(),
        label,
    })
    .collect()
}

/// Refused for good: the workflow of the issue `key` leads nowhere from `from` to `to`.
fn no_transition(lang: Lang, from: &str, to: &str, key: &str) -> String {
    tr_in!(
        lang,
        "Jira : aucune transition de « {from} » vers « {to} » pour {key}",
        "Jira: no transition from “{from}” to “{to}” for {key}"
    )
}

impl Jira {
    pub fn new(http: reqwest::Client, account: &Credentials) -> Self {
        let pair = format!("{}:{}", account.email.trim(), account.token.trim());
        Self {
            http,
            site: site(&account.url),
            auth: format!(
                "Basic {}",
                base64::engine::general_purpose::STANDARD.encode(pair)
            ),
        }
    }

    fn get(&self, path: &str) -> reqwest::RequestBuilder {
        self.http
            .get(format!("{}{path}", self.site))
            .header("Authorization", &self.auth)
            .header("Accept", "application/json")
    }

    fn post(&self, path: &str, body: &Value) -> reqwest::RequestBuilder {
        self.http
            .post(format!("{}{path}", self.site))
            .header("Authorization", &self.auth)
            .header("Accept", "application/json")
            .json(body)
    }

    /// A call of its API. An account with no site has nowhere to go: said as a token refused, it can be put right.
    async fn call(&self, rb: reqwest::RequestBuilder) -> Result<Value> {
        if self.site.is_empty() {
            return Err(HttpError {
                status: 401,
                message: tr!(
                    "Jira : ce compte n'a pas de site. Reconnecte-le dans Réglages › Services connectés.",
                    "Jira: this account has no site. Connect it again in Settings › Connected services."
                ),
            }
            .into());
        }
        http::call(S, rb).await
    }

    pub async fn me(&self) -> Result<(String, String)> {
        let me = self.call(self.get("/rest/api/3/myself")).await?;
        let who = me["emailAddress"]
            .as_str()
            .filter(|e| !e.is_empty())
            .or(me["displayName"].as_str())
            .unwrap_or("?");
        let host = self.site.split("://").last().unwrap_or(&self.site);
        Ok((
            format!("{who} · {host}"),
            me["accountId"].as_str().unwrap_or_default().to_string(),
        ))
    }

    pub async fn containers(&self) -> Result<Vec<Container>> {
        let mut out = Vec::new();
        for page in 0..5 {
            let v = self
                .call(self.get(&format!(
                    "/rest/api/3/project/search?maxResults=100&orderBy=key&startAt={}",
                    page * 100
                )))
                .await?;
            for p in v["values"].as_array().into_iter().flatten() {
                let key = p["key"].as_str().unwrap_or_default();
                let name = p["name"].as_str().unwrap_or_default();
                out.push(Container {
                    id: key.to_string(),
                    name: format!("{key} — {name}"),
                    group: None,
                    own: false,
                });
            }
            if v["isLast"].as_bool() != Some(false) {
                break;
            }
        }
        Ok(out)
    }

    /// The statuses of the project's issue types, each once.
    pub async fn states(&self, project: &str) -> Result<Vec<ExternalState>> {
        let v = self
            .call(self.get(&format!("/rest/api/3/project/{}/statuses", encode(project))))
            .await?;
        let mut out: Vec<ExternalState> = Vec::new();
        for t in v.as_array().into_iter().flatten() {
            for s in t["statuses"].as_array().into_iter().flatten() {
                let id = s["id"].as_str().unwrap_or_default();
                if !id.is_empty() && !out.iter().any(|x| x.id == id) {
                    out.push(ExternalState {
                        id: id.to_string(),
                        name: s["name"].as_str().unwrap_or_default().to_string(),
                    });
                }
            }
        }
        Ok(out)
    }

    pub async fn issues(&self, project: &str, q: &Query) -> Result<IssuePage> {
        let mut jql = vec![
            format!("project = {}", quoted(project)),
            "statusCategory != Done".to_string(),
        ];
        for f in &q.filters {
            match f.as_str() {
                "mine" => jql.push("assignee = currentUser()".into()),
                "sprint" => jql.push("sprint in openSprints()".into()),
                "todo" => jql.push("statusCategory = \"To Do\"".into()),
                _ => {}
            }
        }
        if let Some(label) = q.label.as_deref().filter(|l| !l.trim().is_empty()) {
            jql.push(format!("labels = {}", quoted(label.trim())));
        }
        let text = q.text.trim();
        if is_key(text) {
            jql.push(format!("key = {}", quoted(&text.to_uppercase())));
        } else if !words(text).is_empty() {
            jql.push(format!("text ~ {}", quoted(&words(text))));
        }
        let jql = jql.join(" AND ");
        let mut body = json!({
            "jql": format!("{jql} ORDER BY updated DESC"),
            "maxResults": 50,
            "fields": ["summary", "status", "issuetype", "priority", "assignee", "description", "labels"],
        });
        if let Some(token) = q.page.as_deref().filter(|t| !t.is_empty()) {
            body["nextPageToken"] = json!(token);
        }
        let v = self
            .call(self.post("/rest/api/3/search/jql", &body))
            .await?;
        let issues = v["issues"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|i| self.issue(project, i))
            .collect();
        let next = v["nextPageToken"]
            .as_str()
            .filter(|t| !t.is_empty() && v["isLast"].as_bool() != Some(true))
            .map(str::to_string);
        // The search gives no total: it is counted apart, only when the list is cut.
        let total = match &next {
            Some(_) => self.count(&jql).await,
            None => None,
        };
        Ok(IssuePage {
            issues,
            filters: filters(i18n::ui()),
            next,
            total,
        })
    }

    /// About how many issues `jql` finds; none when Jira does not say (only the total is missing
    /// then).
    async fn count(&self, jql: &str) -> Option<u64> {
        let body = json!({ "jql": jql });
        match self
            .call(self.post("/rest/api/3/search/approximate-count", &body))
            .await
        {
            Ok(v) => v["count"].as_u64(),
            Err(e) => {
                eprintln!("jira: count: {e:#}");
                None
            }
        }
    }

    fn issue(&self, project: &str, i: &Value) -> ExternalIssue {
        let f = &i["fields"];
        let key = i["key"].as_str().unwrap_or_default().to_string();
        let description = match &f["description"] {
            Value::String(s) => s.clone(),
            doc => adf_to_markdown(doc),
        };
        let mut meta = Vec::new();
        if let Some(p) = f["priority"]["name"].as_str() {
            meta.push(priority_label(i18n::ui(), p));
        }
        meta.push(
            f["assignee"]["displayName"]
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| tr!("Non assigné", "Unassigned")),
        );
        if let Some(s) = f["status"]["name"].as_str() {
            meta.push(s.to_string());
        }
        let kind_name = f["issuetype"]["name"].as_str().unwrap_or("Ticket");
        ExternalIssue {
            service: ID.to_string(),
            // Its id stays when it moves to another project (its key changes).
            id: i["id"]
                .as_str()
                .filter(|id| !id.is_empty())
                .unwrap_or(&key)
                .to_string(),
            url: format!("{}/browse/{key}", self.site),
            key,
            title: f["summary"].as_str().unwrap_or_default().to_string(),
            kind: kind_label(i18n::ui(), kind_name),
            kind_code: kind_code(kind_name).to_string(),
            meta,
            criteria: criteria_of(&description),
            description,
            container: project.to_string(),
            imported: false,
        }
    }

    /// The transition that leads to `state`, unless the issue is there already.
    pub async fn set_state(&self, r: &ExternalRef, state: &ExternalState) -> Result<()> {
        let id = encode(&r.id);
        let issue = self
            .call(self.get(&format!("/rest/api/3/issue/{id}?fields=status")))
            .await?;
        let current = &issue["fields"]["status"];
        let named = |v: &Value| {
            v["name"]
                .as_str()
                .is_some_and(|n| n.to_lowercase() == state.name.to_lowercase())
        };
        if current["id"].as_str() == Some(state.id.as_str()) || named(current) {
            return Ok(());
        }
        let v = self
            .call(self.get(&format!("/rest/api/3/issue/{id}/transitions")))
            .await?;
        let same = |t: &Value| t["to"]["id"].as_str() == Some(state.id.as_str()) || named(&t["to"]);
        let Some(t) = v["transitions"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|t| same(t))
        else {
            // Its workflow will not lead there however often it is asked.
            return Err(Refused(no_transition(
                i18n::ui(),
                current["name"].as_str().unwrap_or("?"),
                &state.name,
                &r.key,
            ))
            .into());
        };
        self.call(self.post(
            &format!("/rest/api/3/issue/{id}/transitions"),
            &json!({ "transition": { "id": t["id"] } }),
        ))
        .await?;
        Ok(())
    }

    pub async fn comment(&self, r: &ExternalRef, text: &str) -> Result<()> {
        self.call(self.post(
            &format!("/rest/api/3/issue/{}/comment", encode(&r.id)),
            &json!({ "body": markdown_to_adf(text) }),
        ))
        .await?;
        Ok(())
    }
}

impl TicketSource for Jira {
    fn me(&self) -> BoxFuture<'_, Result<(String, String)>> {
        Box::pin(Jira::me(self))
    }
    fn containers(&self) -> BoxFuture<'_, Result<Vec<Container>>> {
        Box::pin(Jira::containers(self))
    }
    fn states<'a>(&'a self, container: &'a str) -> BoxFuture<'a, Result<Vec<ExternalState>>> {
        Box::pin(Jira::states(self, container))
    }
    fn issues<'a>(&'a self, container: &'a str, q: &'a Query) -> BoxFuture<'a, Result<IssuePage>> {
        Box::pin(Jira::issues(self, container, q))
    }
    fn set_state<'a>(
        &'a self,
        r: &'a ExternalRef,
        state: &'a ExternalState,
        _mapped: &'a [ExternalState],
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(Jira::set_state(self, r, state))
    }
    fn comment<'a>(&'a self, r: &'a ExternalRef, text: &'a str) -> BoxFuture<'a, Result<()>> {
        Box::pin(Jira::comment(self, r, text))
    }
}

/// The Kanban's source, Jira.
pub struct JiraSource;

impl SourceKind for JiraSource {
    fn id(&self) -> &str {
        ID
    }
    fn name(&self) -> &str {
        S
    }
    fn client(&self, http: reqwest::Client, account: &Credentials) -> Box<dyn TicketSource> {
        Box::new(Jira::new(http, account))
    }
    /// One Jira account, whatever its site: its token is `account:jira`, not the site's host.
    fn account_name(&self, _url: &str) -> String {
        ID.to_string()
    }
}

/// What its module runs, on its first turn.
pub fn run() -> i32 {
    escouade_plugin_kit::serve::run(
        "jira",
        escouade_plugin_kit::hosts::Hosts::new(vec![], vec![Box::new(JiraSource)]),
    )
}

escouade_plugin_kit::protocol::module!(run);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jiras_names_stay_as_jira_writes_them_in_english_and_the_filters_read_in_english() {
        use Lang::{En, Fr};
        assert_eq!(
            [kind_label(En, "Task"), priority_label(En, "Highest")],
            ["Task", "Highest"]
        );
        assert_eq!(
            [kind_label(Fr, "Task"), priority_label(Fr, "Highest")],
            ["Tâche", "Très haute"]
        );
        assert_eq!(
            filters(En)
                .iter()
                .map(|f| f.label.as_str())
                .collect::<Vec<_>>(),
            ["Assigned to me", "Active sprint", "To do"]
        );
        assert_eq!(
            no_transition(En, "In Progress", "Done", "ATL-1"),
            "Jira: no transition from “In Progress” to “Done” for ATL-1"
        );
        // The window colours a type by its code, the same whatever its name says.
        assert_eq!(
            [
                "Task",
                "Tâche",
                "Sub-task",
                "Sous-tâche",
                "Bug",
                "Story",
                "Epic",
                "Spike"
            ]
            .map(kind_code),
            ["task", "task", "subtask", "subtask", "bug", "story", "epic", ""]
        );
    }

    #[test]
    fn a_typed_site_is_an_address() {
        assert_eq!(
            site(" atlas.atlassian.net/ "),
            "https://atlas.atlassian.net"
        );
        assert_eq!(site("http://localhost:4101"), "http://localhost:4101");
        assert!(is_key("ATL-1287") && is_key("A1_B-2"));
        assert!(!is_key("ATL-") && !is_key("1A-2") && !is_key("token refresh"));
    }

    #[tokio::test]
    async fn an_account_with_no_site_is_said_as_credentials_refused() {
        let j = Jira::new(reqwest::Client::new(), &Credentials::default());
        let e = j.me().await.unwrap_err();
        assert_eq!(e.downcast_ref::<HttpError>().unwrap().status, 401);
    }

    #[test]
    fn the_account_is_one_whatever_its_site_and_the_credentials_make_the_basic_header() {
        assert_eq!(
            JiraSource.account_name("https://atlas.atlassian.net"),
            "jira"
        );
        let account = Credentials {
            url: "atlas.atlassian.net/".into(),
            email: " ada@atlas.dev ".into(),
            token: " tok ".into(),
            ..Default::default()
        };
        let j = Jira::new(reqwest::Client::new(), &account);
        assert_eq!(j.site, "https://atlas.atlassian.net");
        // base64("ada@atlas.dev:tok")
        assert_eq!(j.auth, "Basic YWRhQGF0bGFzLmRldjp0b2s=");
    }
}
