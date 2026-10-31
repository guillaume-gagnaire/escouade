//! Linear (GraphQL API), with a personal API key. A project's source is a team; an issue's state
//! on the Kanban is one of the team's workflow states.

use super::text::criteria_of;
use super::{
    http_error, Account, Container, ExternalIssue, HttpError, IssueFilter, IssuePage, Query,
    Refused,
};
use crate::i18n::{self, Lang};
use crate::model::{ExternalRef, ExternalState, Service};
use anyhow::{anyhow, Result};
use serde_json::{json, Value};

const S: Service = Service::LINEAR;

/// The issues of a page of the import.
const PAGE: u32 = 50;
/// The prefix of a state's chip in the import.
const STATE: &str = "state:";
/// The kinds of workflow states, in the order an issue goes through them.
const KINDS: [&str; 6] = [
    "triage",
    "backlog",
    "unstarted",
    "started",
    "completed",
    "canceled",
];

pub struct Linear {
    http: reqwest::Client,
    /// The API's address, `https://api.linear.app` (`Bases::linear`).
    base: String,
    key: String,
}

impl Linear {
    pub fn new(http: reqwest::Client, a: &Account, base: &str) -> Self {
        Self {
            http,
            base: base.trim_end_matches('/').to_string(),
            key: a.token.trim().to_string(),
        }
    }

    /// One GraphQL operation, its `data` answered. A GraphQL error (Linear answers them with 200
    /// or 400) reads as the HTTP error it amounts to: refused credentials as a 401, a limit as a
    /// 429, a request it refuses as a 400.
    async fn gql(&self, operation: &str, query: &str, variables: Value) -> Result<Value> {
        let resp = self
            .http
            .post(format!("{}/graphql", self.base))
            // A personal API key goes as it is, with no scheme.
            .header("Authorization", &self.key)
            .header("User-Agent", "escouade")
            .json(&json!({ "operationName": operation, "query": query, "variables": variables }))
            .send()
            .await
            .map_err(|e| {
                let (name, e) = (S.label(), anyhow::Error::from(e.without_url()));
                anyhow!(tr!(
                    "{name} injoignable : {e:#}",
                    "{name} can’t be reached: {e:#}"
                ))
            })?;
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
        let code = v["errors"][0]["extensions"]["code"]
            .as_str()
            .unwrap_or_default();
        let failed = match code {
            "AUTHENTICATION_ERROR" => Some(401),
            "FORBIDDEN" => Some(403),
            "RATELIMITED" => Some(429),
            _ if !(200..300).contains(&status) => Some(status),
            _ if v["errors"].as_array().is_some_and(|e| !e.is_empty()) => Some(400),
            _ => None,
        };
        if let Some(status) = failed {
            return Err(HttpError {
                status,
                message: http_error(S, status, &text),
            }
            .into());
        }
        if v["data"].is_null() {
            let name = S.label();
            return Err(anyhow!(tr!(
                "{name} : réponse illisible",
                "{name}: unreadable answer"
            )));
        }
        Ok(v["data"].clone())
    }

    pub async fn me(&self) -> Result<(String, String)> {
        let d = self
            .gql(
                "Me",
                "query Me { viewer { id name displayName email } }",
                json!({}),
            )
            .await?;
        let me = &d["viewer"];
        let label = [&me["email"], &me["name"], &me["displayName"]]
            .iter()
            .find_map(|v| v.as_str().filter(|s| !s.is_empty()))
            .unwrap_or("?")
            .to_string();
        Ok((label, me["id"].as_str().unwrap_or_default().to_string()))
    }

    /// The teams I am in.
    pub async fn containers(&self) -> Result<Vec<Container>> {
        let d = self
            .gql(
                "Teams",
                "query Teams { teams(first: 100) { nodes { id key name } } }",
                json!({}),
            )
            .await?;
        Ok(nodes(&d["teams"])
            .map(|t| Container {
                id: t["id"].as_str().unwrap_or_default().to_string(),
                name: format!(
                    "{} ({})",
                    t["name"].as_str().unwrap_or_default(),
                    t["key"].as_str().unwrap_or_default()
                ),
                group: None,
                own: false,
            })
            .collect())
    }

    /// The team's workflow states, in the order an issue goes through them.
    pub async fn states(&self, team: &str) -> Result<Vec<ExternalState>> {
        Ok(self
            .workflow(team)
            .await?
            .into_iter()
            .map(|(state, _)| state)
            .collect())
    }

    /// The team's workflow states with their kind (`KINDS`).
    async fn workflow(&self, team: &str) -> Result<Vec<(ExternalState, String)>> {
        let d = self
            .gql(
                "States",
                "query States($team: String!) { team(id: $team) { states { nodes { id name type position } } } }",
                json!({ "team": team }),
            )
            .await?;
        let mut states: Vec<&Value> = nodes(&d["team"]["states"]).collect();
        let rank = |s: &Value| {
            KINDS
                .iter()
                .position(|k| Some(*k) == s["type"].as_str())
                .unwrap_or(KINDS.len())
        };
        states.sort_by(|a, b| {
            rank(a).cmp(&rank(b)).then(
                a["position"]
                    .as_f64()
                    .unwrap_or(0.0)
                    .total_cmp(&b["position"].as_f64().unwrap_or(0.0)),
            )
        });
        Ok(states
            .into_iter()
            .map(|s| {
                (
                    ExternalState {
                        id: s["id"].as_str().unwrap_or_default().to_string(),
                        name: s["name"].as_str().unwrap_or_default().to_string(),
                    },
                    s["type"].as_str().unwrap_or_default().to_string(),
                )
            })
            .collect())
    }

    /// The team's issues not done nor canceled, the latest updated first; "mine" keeps those
    /// assigned to me, a state's chip those in it (any of the chosen states). A text like
    /// `ENG-42` (or `42`) finds that issue; another, the issues whose title holds it.
    pub async fn issues(&self, team: &str, q: &Query) -> Result<IssuePage> {
        let workflow = self.workflow(team).await?;
        let filter = issue_filter(team, q);
        let d = self
            .gql(
                "Issues",
                "query Issues($filter: IssueFilter, $after: String, $first: Int) { issues(filter: $filter, first: $first, after: $after, orderBy: updatedAt) { nodes { id identifier title description url priorityLabel assignee { displayName name } state { id name } labels { nodes { name } } } pageInfo { hasNextPage endCursor } } }",
                json!({ "filter": filter, "after": q.page, "first": PAGE }),
            )
            .await?;
        let lang = i18n::ui();
        let issues = nodes(&d["issues"])
            .map(|i| {
                let description = i["description"].as_str().unwrap_or_default().to_string();
                let assignee = [&i["assignee"]["displayName"], &i["assignee"]["name"]]
                    .iter()
                    .find_map(|v| v.as_str().filter(|s| !s.is_empty()))
                    .map(str::to_string);
                let priority = i["priorityLabel"]
                    .as_str()
                    .filter(|p| !p.is_empty() && *p != "No priority")
                    .map(str::to_string);
                let mut meta: Vec<String> = [
                    i["state"]["name"].as_str().map(str::to_string),
                    priority,
                    assignee,
                ]
                .into_iter()
                .flatten()
                .collect();
                meta.extend(
                    nodes(&i["labels"]).filter_map(|l| l["name"].as_str().map(str::to_string)),
                );
                let mut criteria = criteria_of(&description);
                criteria.truncate(20);
                ExternalIssue {
                    service: S,
                    id: i["id"].as_str().unwrap_or_default().to_string(),
                    key: i["identifier"].as_str().unwrap_or_default().to_string(),
                    title: i["title"].as_str().unwrap_or_default().to_string(),
                    kind: issue(lang),
                    kind_code: "issue".into(),
                    meta: meta.into_iter().filter(|m| !m.is_empty()).collect(),
                    url: i["url"].as_str().unwrap_or_default().to_string(),
                    criteria,
                    description,
                    container: team.to_string(),
                    imported: false,
                }
            })
            .collect();
        let info = &d["issues"]["pageInfo"];
        let next = (info["hasNextPage"] == true)
            .then(|| info["endCursor"].as_str().map(str::to_string))
            .flatten();
        let mut filters = vec![mine_filter(lang)];
        filters.extend(
            workflow
                .iter()
                .filter(|(_, kind)| kind != "completed" && kind != "canceled")
                .map(|(s, _)| IssueFilter {
                    id: format!("{STATE}{}", s.id),
                    label: s.name.clone(),
                }),
        );
        Ok(IssuePage {
            issues,
            filters,
            next,
            total: None,
        })
    }

    /// The issue goes into the state.
    pub async fn set_state(&self, r: &ExternalRef, state: &ExternalState) -> Result<()> {
        let d = self
            .gql(
                "SetState",
                "mutation SetState($id: String!, $state: String!) { issueUpdate(id: $id, input: { stateId: $state }) { success } }",
                json!({ "id": r.id, "state": state.id }),
            )
            .await?;
        if d["issueUpdate"]["success"] != true {
            let (key, name) = (&r.key, &state.name);
            return Err(Refused(tr!(
                "Linear refuse de passer {key} à « {name} »",
                "Linear refuses to move {key} to “{name}”"
            ))
            .into());
        }
        Ok(())
    }

    pub async fn comment(&self, r: &ExternalRef, text: &str) -> Result<()> {
        let d = self
            .gql(
                "Comment",
                "mutation Comment($id: String!, $body: String!) { commentCreate(input: { issueId: $id, body: $body }) { success } }",
                json!({ "id": r.id, "body": text }),
            )
            .await?;
        if d["commentCreate"]["success"] != true {
            let key = &r.key;
            return Err(Refused(tr!(
                "Linear refuse le commentaire sur {key}",
                "Linear refuses the comment on {key}"
            ))
            .into());
        }
        Ok(())
    }
}

/// The `nodes` of a GraphQL connection.
fn nodes(connection: &Value) -> impl Iterator<Item = &Value> {
    connection["nodes"].as_array().into_iter().flatten()
}

/// The filter of the issues the import asks for (`IssueFilter` of Linear's API).
fn issue_filter(team: &str, q: &Query) -> Value {
    let mut state = json!({ "type": { "nin": ["completed", "canceled"] } });
    let chosen: Vec<&str> = q
        .filters
        .iter()
        .filter_map(|f| f.strip_prefix(STATE))
        .collect();
    if !chosen.is_empty() {
        state["id"] = json!({ "in": chosen });
    }
    let mut filter = json!({ "team": { "id": { "eq": team } }, "state": state });
    if q.filters.iter().any(|f| f == "mine") {
        filter["assignee"] = json!({ "isMe": { "eq": true } });
    }
    if let Some(label) = q.label.as_deref().map(str::trim).filter(|l| !l.is_empty()) {
        filter["labels"] = json!({ "some": { "name": { "eqIgnoreCase": label } } });
    }
    let text = q.text.trim();
    if !text.is_empty() {
        // `ENG-42`, `42` or `#42`: that issue's number in the team.
        let number = match text.split_once('-') {
            Some((key, n)) if !key.is_empty() && key.chars().all(char::is_alphanumeric) => n,
            Some(_) => "",
            None => text.trim_start_matches('#'),
        };
        match number.parse::<u64>() {
            Ok(n) => filter["number"] = json!({ "eq": n }),
            Err(_) => filter["title"] = json!({ "containsIgnoreCase": text }),
        }
    }
    filter
}

/// What a Linear issue is called in the import's list.
fn issue(lang: Lang) -> String {
    tr_in!(lang, "Issue", "Issue")
}

/// The filter of the issues assigned to the account.
fn mine_filter(lang: Lang) -> IssueFilter {
    IssueFilter {
        id: "mine".into(),
        label: tr_in!(lang, "Mes issues", "My issues"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrations::fake::FakeServer;

    async fn linear() -> (FakeServer, Linear) {
        let server = FakeServer::start().await;
        let account = Account {
            token: " lin_api_k1 ".into(),
            ..Default::default()
        };
        let l = Linear::new(reqwest::Client::new(), &account, &server.url);
        (server, l)
    }

    fn workflow(server: &FakeServer) {
        server.on(
            "POST",
            "/graphql#States",
            200,
            json!({ "data": { "team": { "states": { "nodes": [
                { "id": "s-done", "name": "Done", "type": "completed", "position": 0 },
                { "id": "s-review", "name": "In Review", "type": "started", "position": 2 },
                { "id": "s-todo", "name": "Todo", "type": "unstarted", "position": 1 },
                { "id": "s-doing", "name": "In Progress", "type": "started", "position": 1 },
                { "id": "s-backlog", "name": "Backlog", "type": "backlog", "position": 0 },
                { "id": "s-canceled", "name": "Canceled", "type": "canceled", "position": 0 }
            ] } } } }),
        );
    }

    #[tokio::test]
    async fn the_account_is_who_the_key_belongs_to_and_the_key_goes_as_it_is() {
        let (server, l) = linear().await;
        server.on(
            "POST",
            "/graphql#Me",
            200,
            json!({ "data": { "viewer": { "id": "u1", "name": "Ada", "email": "ada@atlas.dev" } } }),
        );
        assert_eq!(
            l.me().await.unwrap(),
            ("ada@atlas.dev".to_string(), "u1".to_string())
        );
        let req = &server.requests()[0];
        assert_eq!(req.headers["authorization"], "lin_api_k1");
        assert_eq!(req.json()["operationName"], "Me");
    }

    #[tokio::test]
    async fn a_key_refused_reads_as_credentials_refused_and_can_be_put_right() {
        let (server, l) = linear().await;
        server.on(
            "POST",
            "/graphql#Me",
            400,
            json!({ "errors": [{ "message": "Authentication required, not authenticated",
                                 "extensions": { "code": "AUTHENTICATION_ERROR" } }] }),
        );
        let e = l.me().await.unwrap_err();
        assert_eq!(e.downcast_ref::<HttpError>().unwrap().status, 401);
        assert!(!crate::integrations::lasting(&e));
        assert!(
            e.to_string()
                .contains("Authentication required, not authenticated"),
            "{e}"
        );
        // An error of the query, answered with a 200: a request refused as it is.
        server.on(
            "POST",
            "/graphql#Teams",
            200,
            json!({ "data": null, "errors": [{ "message": "Argument Validation Error" }] }),
        );
        let e = l.containers().await.unwrap_err();
        assert!(crate::integrations::lasting(&e));
        assert!(e.to_string().contains("Argument Validation Error"), "{e}");
    }

    #[tokio::test]
    async fn the_teams_are_the_containers_and_their_states_come_in_workflow_order() {
        let (server, l) = linear().await;
        server.on(
            "POST",
            "/graphql#Teams",
            200,
            json!({ "data": { "teams": { "nodes": [{ "id": "t1", "key": "ENG", "name": "Engineering" }] } } }),
        );
        assert_eq!(
            l.containers().await.unwrap(),
            [Container {
                id: "t1".into(),
                name: "Engineering (ENG)".into(),
                group: None,
                own: false,
            }]
        );
        workflow(&server);
        let names: Vec<String> = l
            .states("t1")
            .await
            .unwrap()
            .into_iter()
            .map(|s| s.name)
            .collect();
        assert_eq!(
            names,
            [
                "Backlog",
                "Todo",
                "In Progress",
                "In Review",
                "Done",
                "Canceled"
            ]
        );
        // « En cours » and « À tester » by default, as for the other services.
        let defaults = super::super::text::default_states(&l.states("t1").await.unwrap());
        use crate::model::Column;
        assert_eq!(defaults[&Column::Doing].name, "In Progress");
        assert_eq!(defaults[&Column::Done].name, "In Review");
    }

    #[tokio::test]
    async fn the_issues_come_with_their_criteria_and_the_filters_go_to_linear() {
        let (server, l) = linear().await;
        workflow(&server);
        server.on(
            "POST",
            "/graphql#Issues",
            200,
            json!({ "data": { "issues": {
                "nodes": [{
                    "id": "i1", "identifier": "ENG-42", "title": "Exporter le journal",
                    "description": "Export CSV.\n\n## Critères d'acceptation\n- Un bouton Exporter\n- Le fichier s'ouvre dans Excel",
                    "url": "https://linear.app/atlas/issue/ENG-42", "priorityLabel": "High",
                    "assignee": { "displayName": "ada" }, "state": { "id": "s-todo", "name": "Todo" },
                    "labels": { "nodes": [{ "name": "claude-ready" }] }
                }],
                "pageInfo": { "hasNextPage": true, "endCursor": "c2" }
            } } }),
        );
        let q = Query {
            text: "eng-42".into(),
            filters: vec!["mine".into(), "state:s-todo".into()],
            label: Some("claude-ready".into()),
            page: Some("c1".into()),
        };
        let page = l.issues("t1", &q).await.unwrap();
        let i = &page.issues[0];
        assert_eq!((i.key.as_str(), i.kind_code.as_str()), ("ENG-42", "issue"));
        assert_eq!(i.meta, ["Todo", "High", "ada", "claude-ready"]);
        assert_eq!(
            i.criteria,
            ["Un bouton Exporter", "Le fichier s'ouvre dans Excel"]
        );
        assert_eq!(i.container, "t1");
        assert_eq!(page.next.as_deref(), Some("c2"));
        // The chips: mine, then the states not done nor canceled.
        let chips: Vec<&str> = page.filters.iter().map(|f| f.id.as_str()).collect();
        assert_eq!(
            chips,
            [
                "mine",
                "state:s-backlog",
                "state:s-todo",
                "state:s-doing",
                "state:s-review"
            ]
        );
        let sent = server
            .requests()
            .into_iter()
            .find(|r| r.json()["operationName"] == "Issues")
            .unwrap()
            .json();
        let f = &sent["variables"]["filter"];
        assert_eq!(f["team"]["id"]["eq"], "t1");
        assert_eq!(f["state"]["id"]["in"], json!(["s-todo"]));
        assert_eq!(f["state"]["type"]["nin"], json!(["completed", "canceled"]));
        assert_eq!(f["assignee"]["isMe"]["eq"], true);
        assert_eq!(f["labels"]["some"]["name"]["eqIgnoreCase"], "claude-ready");
        assert_eq!(f["number"]["eq"], 42);
        assert_eq!(sent["variables"]["after"], "c1");
    }

    #[test]
    fn a_text_searches_the_titles_unless_it_names_an_issue() {
        let q = |text: &str| Query {
            text: text.into(),
            ..Default::default()
        };
        assert_eq!(issue_filter("t", &q("42"))["number"]["eq"], 42);
        assert_eq!(issue_filter("t", &q("ENG-7"))["number"]["eq"], 7);
        let f = issue_filter("t", &q("export csv"));
        assert_eq!(f["title"]["containsIgnoreCase"], "export csv");
        assert!(f["number"].is_null());
        assert!(issue_filter("t", &q("v2-beta"))["title"]["containsIgnoreCase"].is_string());
        assert!(issue_filter("t", &q(""))["title"].is_null());
    }

    #[tokio::test]
    async fn a_state_and_a_comment_are_mutations_and_a_refusal_lasts() {
        let (server, l) = linear().await;
        let r = ExternalRef {
            service: S,
            id: "i1".into(),
            key: "ENG-42".into(),
            ..Default::default()
        };
        let review = ExternalState {
            id: "s-review".into(),
            name: "In Review".into(),
        };
        server.on(
            "POST",
            "/graphql#SetState",
            200,
            json!({ "data": { "issueUpdate": { "success": true } } }),
        );
        server.on(
            "POST",
            "/graphql#Comment",
            200,
            json!({ "data": { "commentCreate": { "success": true } } }),
        );
        l.set_state(&r, &review).await.unwrap();
        l.comment(&r, "Escouade : ENG-42 est prêt à tester.")
            .await
            .unwrap();
        let writes: Vec<Value> = server.writes().iter().map(|w| w.json()).collect();
        assert_eq!(
            writes[0]["variables"],
            json!({ "id": "i1", "state": "s-review" })
        );
        assert_eq!(
            writes[1]["variables"]["body"],
            "Escouade : ENG-42 est prêt à tester."
        );
        server.on(
            "POST",
            "/graphql#SetState",
            200,
            json!({ "data": { "issueUpdate": { "success": false } } }),
        );
        let e = l.set_state(&r, &review).await.unwrap_err();
        assert!(crate::integrations::lasting(&e));
    }

    #[test]
    fn the_filter_of_linear_reads_in_english() {
        assert_eq!(mine_filter(Lang::En).label, "My issues");
    }
}
