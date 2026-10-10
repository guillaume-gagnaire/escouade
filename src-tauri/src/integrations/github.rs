//! GitHub Issues (REST API), with a token: one given, or the GitHub CLI's. An issue's state on the
//! Kanban is open or closed, or a label of the repository.

use super::text::criteria_of;
use super::{
    call, call_page, encode, Account, Container, ExternalIssue, HttpError, IssueFilter, IssuePage,
    Query,
};
use crate::i18n::{self, Lang};
use crate::model::{ExternalRef, ExternalState, Service};
use anyhow::Result;
use serde_json::{json, Value};

const S: Service = Service::Github;

/// The states every repository has, before its labels.
const OPEN: &str = "open";
const CLOSED: &str = "closed";
const LABEL: &str = "label:";

pub struct Github {
    http: reqwest::Client,
    base: String,
    token: String,
}

impl Github {
    pub fn new(http: reqwest::Client, a: &Account, base: &str) -> Self {
        Self {
            http,
            base: base.trim_end_matches('/').to_string(),
            token: a.token.trim().to_string(),
        }
    }

    fn req(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        self.http
            .request(method, format!("{}{path}", self.base))
            .bearer_auth(&self.token)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "escouade")
    }

    fn get(&self, path: &str) -> reqwest::RequestBuilder {
        self.req(reqwest::Method::GET, path)
    }

    pub async fn me(&self) -> Result<(String, String)> {
        let me = call(S, self.get("/user")).await?;
        let login = me["login"].as_str().unwrap_or("?").to_string();
        Ok((format!("@{login}"), login))
    }

    /// The repositories I can reach, the last pushed first.
    pub async fn containers(&self) -> Result<Vec<Container>> {
        let v = call(
            S,
            self.get("/user/repos?per_page=100&sort=pushed&affiliation=owner,collaborator,organization_member"),
        )
        .await?;
        Ok(v.as_array()
            .into_iter()
            .flatten()
            .filter_map(|r| r["full_name"].as_str())
            .map(|n| Container {
                id: n.to_string(),
                name: n.to_string(),
            })
            .collect())
    }

    async fn labels(&self, repo: &str) -> Result<Vec<String>> {
        let v = call(S, self.get(&format!("/repos/{repo}/labels?per_page=100"))).await?;
        Ok(v.as_array()
            .into_iter()
            .flatten()
            .filter_map(|l| l["name"].as_str().map(str::to_string))
            .collect())
    }

    /// Open, closed, then a label of the repository.
    pub async fn states(&self, repo: &str) -> Result<Vec<ExternalState>> {
        let mut out = fixed_states(i18n::ui()).to_vec();
        out.extend(
            self.labels(repo)
                .await?
                .into_iter()
                .map(|name| ExternalState {
                    id: format!("{LABEL}{name}"),
                    name,
                }),
        );
        Ok(out)
    }

    /// The open issues (pull requests aside): searched when there is text, an assignee or a
    /// label to look for, else listed; a label's chip keeps those that carry one of the chosen.
    /// A page at a time: the `Link` header says when there is a next one, a search how many
    /// match.
    pub async fn issues(&self, repo: &str, q: &Query) -> Result<IssuePage> {
        let mine = q.filters.iter().any(|f| f == "mine");
        let text = q.text.trim().trim_start_matches('#');
        let label = q.label.as_deref().map(str::trim).filter(|l| !l.is_empty());
        let number = !text.is_empty() && text.chars().all(|c| c.is_ascii_digit());
        let page: u32 = q
            .page
            .as_deref()
            .and_then(|p| p.parse().ok())
            .unwrap_or(1)
            .max(1);
        let at = if page > 1 {
            format!("&page={page}")
        } else {
            String::new()
        };
        let (items, more, total): (Vec<Value>, bool, Option<u64>) = if number {
            // The issue itself: none when there is no such issue.
            let items = match call(S, self.get(&format!("/repos/{repo}/issues/{text}"))).await {
                Ok(i) if i["state"].as_str() == Some(OPEN) => vec![i],
                Ok(_) => Vec::new(),
                Err(e)
                    if e.downcast_ref::<HttpError>()
                        .is_some_and(|h| h.status == 404) =>
                {
                    Vec::new()
                }
                Err(e) => return Err(e),
            };
            (items, false, None)
        } else if text.is_empty() && !mine && label.is_none() {
            let (v, more) = call_page(
                S,
                self.get(&format!("/repos/{repo}/issues?state=open&per_page=100{at}")),
            )
            .await?;
            (v.as_array().cloned().unwrap_or_default(), more, None)
        } else {
            let mut terms = vec![format!("repo:{repo}"), "is:issue".into(), "is:open".into()];
            if mine {
                terms.push("assignee:@me".into());
            }
            if let Some(l) = label {
                terms.push(format!("label:\"{}\"", l.replace('"', "")));
            }
            if !text.is_empty() {
                terms.push(text.replace('"', " "));
            }
            let (v, more) = call_page(
                S,
                self.get(&format!(
                    "/search/issues?per_page=50{at}&q={}",
                    encode(&terms.join(" "))
                )),
            )
            .await?;
            let items = v["items"].as_array().cloned().unwrap_or_default();
            (items, more, v["total_count"].as_u64())
        };
        let chosen: Vec<&str> = q
            .filters
            .iter()
            .filter_map(|f| f.strip_prefix(LABEL))
            .collect();
        let issues = items
            .iter()
            .filter(|i| i.get("pull_request").is_none())
            .filter(|i| {
                chosen.is_empty() || label_names(i).iter().any(|l| chosen.contains(&l.as_str()))
            })
            .map(|i| issue(repo, i))
            .collect();
        let mut filters = vec![mine_filter(i18n::ui())];
        filters.extend(
            self.labels(repo)
                .await?
                .into_iter()
                .take(12)
                .map(|l| IssueFilter {
                    id: format!("{LABEL}{l}"),
                    label: l,
                }),
        );
        Ok(IssuePage {
            issues,
            filters,
            next: more.then(|| (page + 1).to_string()),
            // The search counts them before a label's chip keeps some here.
            total: total.filter(|_| chosen.is_empty()),
        })
    }

    fn issue_path(r: &ExternalRef) -> String {
        format!("/repos/{}/issues/{}", r.container, encode(&r.id))
    }

    /// Closed, opened again, or given its label (opened again if need be) without the other
    /// labels the link gives a column.
    pub async fn set_state(
        &self,
        r: &ExternalRef,
        state: &ExternalState,
        mapped: &[ExternalState],
    ) -> Result<()> {
        let path = Self::issue_path(r);
        let issue = call(S, self.get(&path)).await?;
        let open = issue["state"].as_str() != Some(CLOSED);
        let has = label_names(&issue);
        let patch = |body: Value| self.req(reqwest::Method::PATCH, &path).json(&body);
        if state.id == CLOSED {
            if open {
                call(
                    S,
                    patch(json!({ "state": "closed", "state_reason": "completed" })),
                )
                .await?;
            }
            return Ok(());
        }
        if !open {
            call(S, patch(json!({ "state": "open" }))).await?;
        }
        let wanted = state.id.strip_prefix(LABEL);
        for m in mapped {
            let Some(other) = m.id.strip_prefix(LABEL) else {
                continue;
            };
            if Some(other) != wanted && has.iter().any(|h| h == other) {
                call(
                    S,
                    self.req(
                        reqwest::Method::DELETE,
                        &format!("{path}/labels/{}", encode(other)),
                    ),
                )
                .await?;
            }
        }
        if let Some(w) = wanted.filter(|w| !has.iter().any(|h| h == w)) {
            call(
                S,
                self.req(reqwest::Method::POST, &format!("{path}/labels"))
                    .json(&json!({ "labels": [w] })),
            )
            .await?;
        }
        Ok(())
    }

    pub async fn comment(&self, r: &ExternalRef, text: &str) -> Result<()> {
        call(
            S,
            self.req(
                reqwest::Method::POST,
                &format!("{}/comments", Self::issue_path(r)),
            )
            .json(&json!({ "body": text })),
        )
        .await?;
        Ok(())
    }
}

fn label_names(i: &Value) -> Vec<String> {
    i["labels"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|l| l["name"].as_str().map(str::to_string))
        .collect()
}

/// An issue's states that every repository has, before its labels: open, closed.
fn fixed_states(lang: Lang) -> [ExternalState; 2] {
    [
        ExternalState {
            id: OPEN.into(),
            name: tr_in!(lang, "Ouverte", "Open"),
        },
        ExternalState {
            id: CLOSED.into(),
            name: tr_in!(lang, "Fermée", "Closed"),
        },
    ]
}

/// The filter of the issues assigned to the account.
fn mine_filter(lang: Lang) -> IssueFilter {
    IssueFilter {
        id: "mine".into(),
        label: tr_in!(lang, "Assignées à moi", "Assigned to me"),
    }
}

fn unassigned(lang: Lang) -> String {
    tr_in!(lang, "Non assignée", "Unassigned")
}

fn issue(repo: &str, i: &Value) -> ExternalIssue {
    let n = i["number"].as_u64().unwrap_or_default();
    let body = i["body"].as_str().unwrap_or_default().to_string();
    let mut meta = label_names(i);
    meta.push(
        i["assignee"]["login"]
            .as_str()
            .map(|l| format!("@{l}"))
            .unwrap_or_else(|| unassigned(i18n::ui())),
    );
    ExternalIssue {
        service: S,
        id: n.to_string(),
        key: format!("#{n}"),
        title: i["title"].as_str().unwrap_or_default().to_string(),
        kind: "Issue".into(),
        meta,
        url: i["html_url"].as_str().unwrap_or_default().to_string(),
        criteria: criteria_of(&body),
        description: body,
        container: repo.to_string(),
        imported: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrations::fake::FakeServer;

    #[test]
    fn the_states_and_the_filter_of_github_read_in_english() {
        use crate::i18n::Lang::En;
        assert_eq!(
            fixed_states(En).map(|s| s.name),
            ["Open".to_string(), "Closed".to_string()]
        );
        assert_eq!(mine_filter(En).label, "Assigned to me");
        assert_eq!(unassigned(En), "Unassigned");
    }

    async fn github() -> (FakeServer, Github) {
        let server = FakeServer::start().await;
        let account = Account {
            token: "ghp_x".into(),
            user: "ada".into(),
            ..Default::default()
        };
        let g = Github::new(reqwest::Client::new(), &account, &server.url);
        (server, g)
    }

    fn r() -> ExternalRef {
        ExternalRef {
            service: S,
            id: "42".into(),
            key: "#42".into(),
            container: "acme/api".into(),
            ..Default::default()
        }
    }

    fn label(name: &str) -> ExternalState {
        ExternalState {
            id: format!("label:{name}"),
            name: name.into(),
        }
    }

    #[tokio::test]
    async fn the_token_is_sent_and_the_login_read() {
        let (server, g) = github().await;
        server.on("GET", "/user", 200, json!({ "login": "ada" }));
        assert_eq!(
            g.me().await.unwrap(),
            ("@ada".to_string(), "ada".to_string())
        );
        let req = &server.requests()[0];
        assert_eq!(req.headers["authorization"], "Bearer ghp_x");
        assert_eq!(req.headers["user-agent"], "escouade");
    }

    #[tokio::test]
    async fn the_states_are_open_closed_and_the_labels() {
        let (server, g) = github().await;
        server.on(
            "GET",
            "/repos/acme/api/labels",
            200,
            json!([{ "name": "in-progress" }, { "name": "to test" }]),
        );
        server.on(
            "GET",
            "/user/repos",
            200,
            json!([{ "full_name": "acme/api" }]),
        );
        assert_eq!(g.containers().await.unwrap()[0].id, "acme/api");
        let ids: Vec<String> = g
            .states("acme/api")
            .await
            .unwrap()
            .into_iter()
            .map(|s| s.id)
            .collect();
        assert_eq!(
            ids,
            ["open", "closed", "label:in-progress", "label:to test"]
        );
    }

    #[tokio::test]
    async fn the_issues_are_listed_or_searched_without_the_pull_requests() {
        let (server, g) = github().await;
        server.on(
            "GET",
            "/repos/acme/api/labels",
            200,
            json!([{ "name": "bug" }]),
        );
        let issues = json!([
            { "number": 42, "title": "Crash au paiement", "body": "- [ ] Plus de crash\n- [ ] Test e2e",
              "html_url": "https://github.com/acme/api/issues/42", "labels": [{ "name": "bug" }], "assignee": { "login": "ada" } },
            { "number": 43, "title": "Une PR", "pull_request": {}, "labels": [] },
            { "number": 44, "title": "Doc", "body": null, "labels": [], "assignee": null }
        ]);
        server.on("GET", "/repos/acme/api/issues", 200, issues.clone());
        let page = g.issues("acme/api", &Query::default()).await.unwrap();
        let i = &page.issues[0];
        assert_eq!(page.issues.len(), 2);
        assert_eq!(
            (i.id.as_str(), i.key.as_str(), i.kind.as_str()),
            ("42", "#42", "Issue")
        );
        assert_eq!(i.meta, ["bug", "@ada"]);
        assert_eq!(i.criteria, ["Plus de crash", "Test e2e"]);
        assert_eq!(page.issues[1].meta, ["Non assignée"]);
        assert_eq!(server.requests()[0].query("state").as_deref(), Some("open"));
        let ids: Vec<&str> = page.filters.iter().map(|f| f.id.as_str()).collect();
        assert_eq!(ids, ["mine", "label:bug"]);
        // A label's chip keeps those that carry it.
        let q = Query {
            filters: vec!["label:bug".into()],
            ..Default::default()
        };
        assert_eq!(g.issues("acme/api", &q).await.unwrap().issues.len(), 1);
        server.on("GET", "/search/issues", 200, json!({ "items": issues }));
        let q = Query {
            text: "#crash".into(),
            filters: vec!["mine".into()],
            label: Some("claude-ready".into()),
            ..Default::default()
        };
        assert_eq!(g.issues("acme/api", &q).await.unwrap().issues.len(), 2);
        let search = server
            .requests()
            .into_iter()
            .find(|r| r.path() == "/search/issues")
            .unwrap();
        assert_eq!(
            search.query("q").as_deref(),
            Some("repo:acme/api is:issue is:open assignee:@me label:\"claude-ready\" crash")
        );
        // A number is the issue itself, when it is an open one.
        server.on(
            "GET",
            "/repos/acme/api/issues/44",
            200,
            json!({ "number": 44, "title": "Doc", "state": "open", "labels": [] }),
        );
        let q = Query {
            text: "#44".into(),
            ..Default::default()
        };
        let found = g.issues("acme/api", &q).await.unwrap().issues;
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].key, "#44");
        server.on(
            "GET",
            "/repos/acme/api/issues/44",
            200,
            json!({ "number": 44, "title": "Doc", "state": "closed", "labels": [] }),
        );
        assert!(g.issues("acme/api", &q).await.unwrap().issues.is_empty());
        server.on(
            "GET",
            "/repos/acme/api/issues/45",
            404,
            json!({ "message": "Not Found" }),
        );
        let q = Query {
            text: "45".into(),
            ..Default::default()
        };
        assert!(g.issues("acme/api", &q).await.unwrap().issues.is_empty());
    }

    #[tokio::test]
    async fn the_issues_come_a_page_at_a_time_and_a_search_says_how_many_there_are() {
        let (server, g) = github().await;
        server.on("GET", "/repos/acme/api/labels", 200, json!([]));
        let issue = |n: u64| json!({ "number": n, "title": format!("Issue {n}"), "labels": [] });
        let more = format!(
            "<{0}/repositories/1/issues?page=2>; rel=\"next\", <{0}/repositories/1/issues?page=4>; rel=\"last\"",
            server.url
        );
        server.on_with(
            "GET",
            "/repos/acme/api/issues",
            200,
            json!([issue(1), issue(2)]),
            &[("Link", &more)],
        );
        let first = g.issues("acme/api", &Query::default()).await.unwrap();
        assert_eq!((first.next.as_deref(), first.total), (Some("2"), None));
        assert_eq!(server.requests()[0].query("page"), None);
        let end = format!(
            "<{0}/repositories/1/issues?page=1>; rel=\"prev\", <{0}/repositories/1/issues?page=1>; rel=\"first\"",
            server.url
        );
        server.on_with(
            "GET",
            "/repos/acme/api/issues",
            200,
            json!([issue(3)]),
            &[("Link", &end)],
        );
        let q = Query {
            page: Some("2".into()),
            ..Default::default()
        };
        let last = g.issues("acme/api", &q).await.unwrap();
        let listed = |path: &str| {
            server
                .requests()
                .into_iter()
                .rfind(|r| r.path() == path)
                .unwrap()
        };
        assert_eq!(
            listed("/repos/acme/api/issues").query("page").as_deref(),
            Some("2")
        );
        assert_eq!(last.issues[0].key, "#3");
        assert_eq!(last.next, None);
        server.on_with(
            "GET",
            "/search/issues",
            200,
            json!({ "total_count": 312, "items": [issue(4)] }),
            &[("Link", &more)],
        );
        let q = Query {
            text: "crash".into(),
            ..Default::default()
        };
        let found = g.issues("acme/api", &q).await.unwrap();
        assert_eq!((found.next.as_deref(), found.total), (Some("2"), Some(312)));
        let q = Query {
            page: Some("3".into()),
            ..q
        };
        g.issues("acme/api", &q).await.unwrap();
        let search = listed("/search/issues");
        assert_eq!(
            (
                search.query("per_page").as_deref(),
                search.query("page").as_deref()
            ),
            (Some("50"), Some("3"))
        );
        // A label's chip keeps some of them here: the search's count is no longer theirs.
        let q = Query {
            filters: vec!["label:bug".into()],
            ..q
        };
        let chipped = g.issues("acme/api", &q).await.unwrap();
        assert_eq!((chipped.next.as_deref(), chipped.total), (Some("4"), None));
    }

    #[tokio::test]
    async fn a_label_state_replaces_the_other_mapped_labels() {
        let (server, g) = github().await;
        server.on(
            "GET",
            "/repos/acme/api/issues/42",
            200,
            json!({ "state": "open", "labels": [{ "name": "in-progress" }, { "name": "bug" }] }),
        );
        server.on(
            "DELETE",
            "/repos/acme/api/issues/42/labels/in-progress",
            200,
            json!([]),
        );
        server.on("POST", "/repos/acme/api/issues/42/labels", 200, json!([]));
        let mapped = [label("in-progress"), label("to test")];
        g.set_state(&r(), &label("to test"), &mapped).await.unwrap();
        let w = server.writes();
        assert_eq!(w.len(), 2);
        assert_eq!(
            (w[0].method.as_str(), w[0].path()),
            ("DELETE", "/repos/acme/api/issues/42/labels/in-progress")
        );
        assert_eq!(w[1].json(), json!({ "labels": ["to test"] }));
        // Already carrying it: nothing to do.
        g.set_state(&r(), &label("in-progress"), &mapped)
            .await
            .unwrap();
        assert_eq!(server.writes().len(), 2);
    }

    #[tokio::test]
    async fn closed_closes_and_open_or_a_label_opens_again() {
        let (server, g) = github().await;
        server.on("PATCH", "/repos/acme/api/issues/42", 200, json!({}));
        server.on("POST", "/repos/acme/api/issues/42/labels", 200, json!([]));
        server.on(
            "GET",
            "/repos/acme/api/issues/42",
            200,
            json!({ "state": "open", "labels": [] }),
        );
        let closed = ExternalState {
            id: "closed".into(),
            name: "Fermée".into(),
        };
        g.set_state(&r(), &closed, &[]).await.unwrap();
        assert_eq!(
            server.writes()[0].json(),
            json!({ "state": "closed", "state_reason": "completed" })
        );
        server.on(
            "GET",
            "/repos/acme/api/issues/42",
            200,
            json!({ "state": "closed", "labels": [] }),
        );
        g.set_state(&r(), &closed, &[]).await.unwrap();
        assert_eq!(server.writes().len(), 1);
        g.set_state(&r(), &label("wip"), &[label("wip")])
            .await
            .unwrap();
        let w = server.writes();
        assert_eq!(w[1].json(), json!({ "state": "open" }));
        assert_eq!(w[2].json(), json!({ "labels": ["wip"] }));
    }

    #[tokio::test]
    async fn a_comment_is_markdown() {
        let (server, g) = github().await;
        server.on(
            "POST",
            "/repos/acme/api/issues/42/comments",
            201,
            json!({ "id": 1 }),
        );
        g.comment(&r(), "Escouade : prêt").await.unwrap();
        assert_eq!(
            server.writes()[0].json(),
            json!({ "body": "Escouade : prêt" })
        );
    }
}
