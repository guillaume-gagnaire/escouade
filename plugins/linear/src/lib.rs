//! Linear (GraphQL API), with a personal API key: the Kanban's source of Linear issues. A project's source is a team;
//! an issue's state on the Kanban is one of the team's workflow states. The account's secret is the API key
//! (`secrets/get` of `account:linear`).

use anyhow::{anyhow, Result};
use escouade_plugin_kit::errors::{http_error_in, HttpError, Refused};
use escouade_plugin_kit::forge::BoxFuture;
use escouade_plugin_kit::hosts::{Credentials, SourceKind, TicketSource};
use escouade_plugin_kit::http::reqwest;
use escouade_plugin_kit::i18n::{self, Lang};
use escouade_plugin_kit::protocol::tickets::{
    Container, ExternalIssue, ExternalRef, ExternalState, IssueFilter, IssuePage, Query,
};
use escouade_plugin_kit::text::criteria_of;
use escouade_plugin_kit::{tr, tr_in};
use serde_json::{json, Value};

/// The source's id, and its name in what it says.
pub const ID: &str = "linear";
const S: &str = "Linear";

/// Where the API is, unless Escouade says otherwise (`ESCOUADE_LINEAR_API`: tests, demos).
const API: &str = "https://api.linear.app";

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
    pub fn new(http: reqwest::Client, account: &Credentials, base: &str) -> Self {
        Self {
            http,
            base: base.trim_end_matches('/').to_string(),
            key: account.token.trim().to_string(),
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
                let (name, e) = (S, anyhow::Error::from(e.without_url()));
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
                message: http_error_in(i18n::ui(), S, status, &text),
            }
            .into());
        }
        if v["data"].is_null() {
            let name = S;
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
                    service: ID.to_string(),
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

impl TicketSource for Linear {
    fn me(&self) -> BoxFuture<'_, Result<(String, String)>> {
        Box::pin(Linear::me(self))
    }
    fn containers(&self) -> BoxFuture<'_, Result<Vec<Container>>> {
        Box::pin(Linear::containers(self))
    }
    fn states<'a>(&'a self, container: &'a str) -> BoxFuture<'a, Result<Vec<ExternalState>>> {
        Box::pin(Linear::states(self, container))
    }
    fn issues<'a>(&'a self, container: &'a str, q: &'a Query) -> BoxFuture<'a, Result<IssuePage>> {
        Box::pin(Linear::issues(self, container, q))
    }
    fn set_state<'a>(
        &'a self,
        r: &'a ExternalRef,
        state: &'a ExternalState,
        _mapped: &'a [ExternalState],
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(Linear::set_state(self, r, state))
    }
    fn comment<'a>(&'a self, r: &'a ExternalRef, text: &'a str) -> BoxFuture<'a, Result<()>> {
        Box::pin(Linear::comment(self, r, text))
    }
}

/// The Kanban's source, Linear.
pub struct LinearSource;

impl SourceKind for LinearSource {
    fn id(&self) -> &str {
        ID
    }
    fn name(&self) -> &str {
        S
    }
    fn client(&self, http: reqwest::Client, account: &Credentials) -> Box<dyn TicketSource> {
        let base = account.api.clone().filter(|a| !a.trim().is_empty());
        Box::new(Linear::new(http, account, base.as_deref().unwrap_or(API)))
    }
}

/// What its module runs, on its first turn.
pub fn run() -> i32 {
    escouade_plugin_kit::serve::run(
        "linear",
        escouade_plugin_kit::hosts::Hosts::new(vec![], vec![Box::new(LinearSource)]),
    )
}

escouade_plugin_kit::protocol::module!(run);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_key_is_trimmed_and_goes_as_it_is() {
        let account = Credentials {
            token: " lin_api_k1 ".into(),
            ..Default::default()
        };
        let l = Linear::new(reqwest::Client::new(), &account, "https://x/");
        assert_eq!(
            (l.key.as_str(), l.base.as_str()),
            ("lin_api_k1", "https://x")
        );
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

    #[test]
    fn the_filter_of_linear_reads_in_english() {
        assert_eq!(mine_filter(Lang::En).label, "My issues");
    }
}
