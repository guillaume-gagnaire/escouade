//! Jira Cloud (REST API v3), with an e-mail and an API token.

use super::text::{adf_to_markdown, criteria_of, markdown_to_adf};
use super::{
    call, encode, Account, Container, ExternalIssue, IssueFilter, IssuePage, Query, Refused,
};
use crate::i18n::{self, Lang};
use crate::model::{ExternalRef, ExternalState, Service};
use anyhow::Result;
use base64::Engine;
use serde_json::{json, Value};

const S: Service = Service::Jira;

pub struct Jira {
    http: reqwest::Client,
    site: String,
    auth: String,
}

/// A site's address as typed ("atlas.atlassian.net/"), as the API is called on.
pub fn site(typed: &str) -> String {
    let s = typed.trim().trim_end_matches('/');
    if s.is_empty() || s.contains("://") {
        s.to_string()
    } else {
        format!("https://{s}")
    }
}

/// A site its token may be sent to: over https, or over http to this machine only (a fake one).
pub fn secure(site: &str) -> bool {
    if site.starts_with("https://") {
        return true;
    }
    let host = site
        .strip_prefix("http://")
        .unwrap_or(site)
        .split('/')
        .next()
        .unwrap_or("");
    let name = host.rsplit_once(':').map_or(host, |(h, _)| h);
    matches!(name, "localhost" | "127.0.0.1" | "[::1]")
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
    pub fn new(http: reqwest::Client, a: &Account) -> Self {
        let pair = format!("{}:{}", a.email.trim(), a.token.trim());
        Self {
            http,
            site: site(&a.site),
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

    pub async fn me(&self) -> Result<(String, String)> {
        let me = call(S, self.get("/rest/api/3/myself")).await?;
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
            let v = call(
                S,
                self.get(&format!(
                    "/rest/api/3/project/search?maxResults=100&orderBy=key&startAt={}",
                    page * 100
                )),
            )
            .await?;
            for p in v["values"].as_array().into_iter().flatten() {
                let key = p["key"].as_str().unwrap_or_default();
                let name = p["name"].as_str().unwrap_or_default();
                out.push(Container {
                    id: key.to_string(),
                    name: format!("{key} — {name}"),
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
        let v = call(
            S,
            self.get(&format!("/rest/api/3/project/{}/statuses", encode(project))),
        )
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
        let v = call(S, self.post("/rest/api/3/search/jql", &body)).await?;
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
        match call(S, self.post("/rest/api/3/search/approximate-count", &body)).await {
            Ok(v) => v["count"].as_u64(),
            Err(e) => {
                log::info!("Jira count: {e:#}");
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
        ExternalIssue {
            service: S,
            // Its id stays when it moves to another project (its key changes).
            id: i["id"]
                .as_str()
                .filter(|id| !id.is_empty())
                .unwrap_or(&key)
                .to_string(),
            url: format!("{}/browse/{key}", self.site),
            key,
            title: f["summary"].as_str().unwrap_or_default().to_string(),
            kind: kind_label(
                i18n::ui(),
                f["issuetype"]["name"].as_str().unwrap_or("Ticket"),
            ),
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
        let issue = call(
            S,
            self.get(&format!("/rest/api/3/issue/{id}?fields=status")),
        )
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
        let v = call(S, self.get(&format!("/rest/api/3/issue/{id}/transitions"))).await?;
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
        call(
            S,
            self.post(
                &format!("/rest/api/3/issue/{id}/transitions"),
                &json!({ "transition": { "id": t["id"] } }),
            ),
        )
        .await?;
        Ok(())
    }

    pub async fn comment(&self, r: &ExternalRef, text: &str) -> Result<()> {
        call(
            S,
            self.post(
                &format!("/rest/api/3/issue/{}/comment", encode(&r.id)),
                &json!({ "body": markdown_to_adf(text) }),
            ),
        )
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrations::fake::FakeServer;

    #[test]
    fn jiras_names_stay_as_jira_writes_them_in_english_and_the_filters_read_in_english() {
        use crate::i18n::Lang::{En, Fr};
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
    }

    async fn jira() -> (FakeServer, Jira) {
        let server = FakeServer::start().await;
        let account = Account {
            site: format!("{}/", server.url),
            email: "ada@atlas.dev".into(),
            token: "tok".into(),
            ..Default::default()
        };
        let j = Jira::new(reqwest::Client::new(), &account);
        (server, j)
    }

    fn r(key: &str) -> ExternalRef {
        ExternalRef {
            service: S,
            id: key.into(),
            key: key.into(),
            ..Default::default()
        }
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
    async fn the_account_is_checked_with_its_email_and_token() {
        let (server, j) = jira().await;
        server.on(
            "GET",
            "/rest/api/3/myself",
            200,
            json!({ "accountId": "acc-1", "emailAddress": "ada@atlas.dev", "displayName": "Ada" }),
        );
        let (label, user) = j.me().await.unwrap();
        let host = server.url.trim_start_matches("http://").to_string();
        assert_eq!(label, format!("ada@atlas.dev · {host}"));
        assert_eq!(user, "acc-1");
        let req = &server.requests()[0];
        // base64("ada@atlas.dev:tok")
        assert_eq!(
            req.headers["authorization"],
            "Basic YWRhQGF0bGFzLmRldjp0b2s="
        );
        server.on("GET", "/rest/api/3/myself", 401, Value::Null);
        assert_eq!(
            j.me().await.unwrap_err().to_string(),
            "Jira refuse ces identifiants (401)"
        );
    }

    #[tokio::test]
    async fn the_projects_and_their_statuses_are_listed() {
        let (server, j) = jira().await;
        server.on(
            "GET",
            "/rest/api/3/project/search",
            200,
            json!({ "isLast": true, "values": [{ "key": "ATL", "name": "Atlas" }, { "key": "MOB", "name": "Mobile" }] }),
        );
        let c = j.containers().await.unwrap();
        assert_eq!(
            c,
            [
                Container {
                    id: "ATL".into(),
                    name: "ATL — Atlas".into()
                },
                Container {
                    id: "MOB".into(),
                    name: "MOB — Mobile".into()
                }
            ]
        );
        server.on(
            "GET",
            "/rest/api/3/project/ATL/statuses",
            200,
            json!([
                { "name": "Story", "statuses": [{ "id": "1", "name": "To Do" }, { "id": "3", "name": "In Progress" }] },
                { "name": "Bug", "statuses": [{ "id": "3", "name": "In Progress" }, { "id": "10", "name": "In Review" }] }
            ]),
        );
        let names: Vec<String> = j
            .states("ATL")
            .await
            .unwrap()
            .into_iter()
            .map(|s| s.name)
            .collect();
        assert_eq!(names, ["To Do", "In Progress", "In Review"]);
    }

    #[tokio::test]
    async fn the_search_builds_its_jql_and_reads_the_issues() {
        let (server, j) = jira().await;
        server.on(
            "POST",
            "/rest/api/3/search/jql",
            200,
            json!({ "issues": [{
                "id": "10001", "key": "ATL-1287",
                "fields": {
                    "summary": "Rafraîchir le token",
                    "issuetype": { "name": "Task" },
                    "priority": { "name": "High" },
                    "assignee": null,
                    "status": { "name": "To Do" },
                    "description": { "type": "doc", "version": 1, "content": [
                        { "type": "paragraph", "content": [{ "type": "text", "text": "Acceptance criteria:" }] },
                        { "type": "bulletList", "content": [
                            { "type": "listItem", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "Refresh avant expiration" }] }] },
                            { "type": "listItem", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "Tests unitaires" }] }] }
                        ]}
                    ]}
                }
            }]}),
        );
        let q = Query {
            text: "token \"refresh\"+".into(),
            filters: vec!["mine".into(), "sprint".into(), "todo".into()],
            label: Some("claude-ready".into()),
            ..Default::default()
        };
        let page = j.issues("ATL", &q).await.unwrap();
        let sent = server.requests()[0].json();
        assert_eq!(
            sent["jql"],
            "project = \"ATL\" AND statusCategory != Done AND assignee = currentUser() AND sprint in openSprints() \
             AND statusCategory = \"To Do\" AND labels = \"claude-ready\" AND text ~ \"token refresh\" ORDER BY updated DESC"
        );
        assert!(sent["fields"]
            .as_array()
            .unwrap()
            .contains(&json!("description")));
        let i = &page.issues[0];
        // Known by its id, which stays when it moves to another project; shown by its key.
        assert_eq!(
            (i.id.as_str(), i.key.as_str(), i.kind.as_str()),
            ("10001", "ATL-1287", "Tâche")
        );
        assert_eq!(i.meta, ["Haute", "Non assigné", "To Do"]);
        assert_eq!(i.url, format!("{}/browse/ATL-1287", server.url));
        assert_eq!(i.criteria, ["Refresh avant expiration", "Tests unitaires"]);
        assert!(i
            .description
            .starts_with("Acceptance criteria:\n\n- Refresh"));
        assert_eq!(i.container, "ATL");
        let filters: Vec<&str> = page.filters.iter().map(|f| f.id.as_str()).collect();
        assert_eq!(filters, ["mine", "sprint", "todo"]);
        // A key is looked for as such.
        j.issues(
            "ATL",
            &Query {
                text: "atl-12".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(
            server.requests()[1].json()["jql"],
            "project = \"ATL\" AND statusCategory != Done AND key = \"ATL-12\" ORDER BY updated DESC"
        );
        server.on(
            "POST",
            "/rest/api/3/search/jql",
            400,
            json!({ "errorMessages": ["Field 'sprint' does not exist."] }),
        );
        assert_eq!(
            j.issues("ATL", &q).await.unwrap_err().to_string(),
            "Jira : erreur 400 — Field 'sprint' does not exist."
        );
    }

    #[tokio::test]
    async fn the_search_goes_on_from_its_next_page_and_says_about_how_many_match() {
        let (server, j) = jira().await;
        let page = |keys: &[&str], next: Option<&str>| {
            let issues: Vec<Value> = keys
                .iter()
                .map(|k| json!({ "id": k, "key": k, "fields": { "summary": k } }))
                .collect();
            match next {
                Some(token) => json!({ "issues": issues, "nextPageToken": token, "isLast": false }),
                None => json!({ "issues": issues, "isLast": true }),
            }
        };
        server.on(
            "POST",
            "/rest/api/3/search/jql",
            200,
            page(&["ATL-1", "ATL-2"], Some("p2")),
        );
        server.on(
            "POST",
            "/rest/api/3/search/approximate-count",
            200,
            json!({ "count": 312 }),
        );
        let first = j.issues("ATL", &Query::default()).await.unwrap();
        assert_eq!(
            (first.next.as_deref(), first.total),
            (Some("p2"), Some(312))
        );
        let sent = server.requests();
        assert_eq!(sent[0].json()["maxResults"], 50);
        assert!(sent[0].json().get("nextPageToken").is_none());
        // Counted on the same issues, without their order.
        assert_eq!(sent[1].path(), "/rest/api/3/search/approximate-count");
        assert_eq!(
            sent[1].json()["jql"],
            "project = \"ATL\" AND statusCategory != Done"
        );
        server.on(
            "POST",
            "/rest/api/3/search/jql",
            200,
            page(&["ATL-3"], None),
        );
        let q = Query {
            page: Some("p2".into()),
            ..Default::default()
        };
        let last = j.issues("ATL", &q).await.unwrap();
        assert_eq!(server.requests()[2].json()["nextPageToken"], "p2");
        assert_eq!(last.issues[0].key, "ATL-3");
        // The last page: nothing more, nothing to count.
        assert_eq!((last.next, last.total), (None, None));
        assert_eq!(server.requests().len(), 3);
        // A count refused only leaves the total unknown.
        server.on(
            "POST",
            "/rest/api/3/search/jql",
            200,
            page(&["ATL-1"], Some("p2")),
        );
        server.on(
            "POST",
            "/rest/api/3/search/approximate-count",
            404,
            Value::Null,
        );
        let again = j.issues("ATL", &Query::default()).await.unwrap();
        assert_eq!((again.next.as_deref(), again.total), (Some("p2"), None));
    }

    #[tokio::test]
    async fn a_state_is_reached_by_the_transition_that_leads_to_it() {
        let (server, j) = jira().await;
        let review = ExternalState {
            id: "10".into(),
            name: "In Review".into(),
        };
        server.on(
            "GET",
            "/rest/api/3/issue/ATL-1/transitions",
            200,
            json!({ "transitions": [
                { "id": "21", "to": { "id": "3", "name": "In Progress" } },
                { "id": "31", "to": { "id": "10", "name": "In Review" } }
            ]}),
        );
        server.on(
            "POST",
            "/rest/api/3/issue/ATL-1/transitions",
            204,
            Value::Null,
        );
        // Already there: nothing moves.
        server.on(
            "GET",
            "/rest/api/3/issue/ATL-1",
            200,
            json!({ "fields": { "status": { "id": "10", "name": "In Review" } } }),
        );
        j.set_state(&r("ATL-1"), &review).await.unwrap();
        assert!(server.writes().is_empty());
        server.on(
            "GET",
            "/rest/api/3/issue/ATL-1",
            200,
            json!({ "fields": { "status": { "id": "3", "name": "In Progress" } } }),
        );
        j.set_state(&r("ATL-1"), &review).await.unwrap();
        let w = server.writes();
        assert_eq!(w[0].target, "/rest/api/3/issue/ATL-1/transitions");
        assert_eq!(w[0].json(), json!({ "transition": { "id": "31" } }));
        // By its name when its id differs (another workflow).
        let named = ExternalState {
            id: "99".into(),
            name: "in progress".into(),
        };
        server.on(
            "GET",
            "/rest/api/3/issue/ATL-1",
            200,
            json!({ "fields": { "status": { "id": "1", "name": "To Do" } } }),
        );
        j.set_state(&r("ATL-1"), &named).await.unwrap();
        assert_eq!(
            server.writes()[1].json(),
            json!({ "transition": { "id": "21" } })
        );
        // Already there by its name: nothing moves.
        server.on(
            "GET",
            "/rest/api/3/issue/ATL-1",
            200,
            json!({ "fields": { "status": { "id": "3", "name": "In Progress" } } }),
        );
        j.set_state(&r("ATL-1"), &named).await.unwrap();
        assert_eq!(server.writes().len(), 2);
        let done = ExternalState {
            id: "5".into(),
            name: "Done".into(),
        };
        let err = j.set_state(&r("ATL-1"), &done).await.unwrap_err();
        assert_eq!(
            err.to_string(),
            "Jira : aucune transition de « In Progress » vers « Done » pour ATL-1"
        );
        // Asking again would not change it.
        assert!(crate::integrations::lasting(&err));
    }

    #[tokio::test]
    async fn a_comment_is_a_document() {
        let (server, j) = jira().await;
        server.on(
            "POST",
            "/rest/api/3/issue/ATL-1/comment",
            201,
            json!({ "id": "c1" }),
        );
        j.comment(&r("ATL-1"), "Escouade : prêt\n\n✓ un")
            .await
            .unwrap();
        let body = server.writes()[0].json();
        assert_eq!(body["body"]["type"], "doc");
        assert_eq!(body["body"]["content"][1]["content"][0]["text"], "✓ un");
    }
}
