//! GitHub Issues (REST API), with a token: one given, or the GitHub CLI's. An issue's state on the
//! Kanban is open or closed, or a label of the repository.

use crate::redirect;
use anyhow::Result;
use escouade_plugin_kit::errors::HttpError;
use escouade_plugin_kit::http::encode;
use escouade_plugin_kit::http::reqwest;
use escouade_plugin_kit::i18n::{self, Lang};
use escouade_plugin_kit::protocol::tickets::{
    Container, ExternalIssue, ExternalRef, ExternalState, IssueFilter, IssuePage, Query,
};
use escouade_plugin_kit::text::criteria_of;
use escouade_plugin_kit::tr_in;
use reqwest::header::{HeaderMap, LINK};
use serde_json::{json, Value};

/// The source's id, and its name in what it says.
pub const ID: &str = "github";
const S: &str = "GitHub";

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
    pub fn new(http: reqwest::Client, token: &str, base: &str) -> Self {
        Self {
            http,
            base: base.trim_end_matches('/').to_string(),
            token: token.trim().to_string(),
        }
    }

    /// A call of the API, wherever it is redirected (a renamed repository): its JSON and its headers.
    async fn send(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&Value>,
    ) -> Result<(Value, HeaderMap)> {
        let url = format!("{}{path}", self.base);
        redirect::send_named(S, method, url, body, |method, url, own, body| {
            let mut rb = self
                .http
                .request(method, url)
                .header("User-Agent", "escouade");
            if own {
                rb = rb
                    .bearer_auth(&self.token)
                    .header("Accept", "application/vnd.github+json")
                    .header("X-GitHub-Api-Version", "2022-11-28");
            }
            match body {
                Some(body) => rb.json(body),
                None => rb,
            }
        })
        .await
    }

    /// A read: its JSON.
    async fn get(&self, path: &str) -> Result<Value> {
        Ok(self.send(reqwest::Method::GET, path, None).await?.0)
    }

    /// A read, and whether the answer's `Link` header points to a next page.
    async fn get_page(&self, path: &str) -> Result<(Value, bool)> {
        let (v, headers) = self.send(reqwest::Method::GET, path, None).await?;
        let next = headers
            .get_all(LINK)
            .iter()
            .filter_map(|h| h.to_str().ok())
            .flat_map(|h| h.split(','))
            .any(|link| link.contains("rel=\"next\""));
        Ok((v, next))
    }

    pub async fn me(&self) -> Result<(String, String)> {
        let me = self.get("/user").await?;
        let login = me["login"].as_str().unwrap_or("?").to_string();
        Ok((format!("@{login}"), login))
    }

    /// The repositories I can reach, the last pushed first.
    pub async fn containers(&self) -> Result<Vec<Container>> {
        let v = self
            .get("/user/repos?per_page=100&sort=pushed&affiliation=owner,collaborator,organization_member")
            .await?;
        Ok(v.as_array()
            .into_iter()
            .flatten()
            .filter_map(|r| r["full_name"].as_str())
            .map(|n| Container {
                id: n.to_string(),
                name: n.to_string(),
                group: None,
                own: false,
            })
            .collect())
    }

    async fn labels(&self, repo: &str) -> Result<Vec<String>> {
        let v = self
            .get(&format!("/repos/{repo}/labels?per_page=100"))
            .await?;
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
            let items = match self.get(&format!("/repos/{repo}/issues/{text}")).await {
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
            let (v, more) = self
                .get_page(&format!("/repos/{repo}/issues?state=open&per_page=100{at}"))
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
            let (v, more) = self
                .get_page(&format!(
                    "/search/issues?per_page=50{at}&q={}",
                    encode(&terms.join(" "))
                ))
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
        let issue = self.get(&path).await?;
        let open = issue["state"].as_str() != Some(CLOSED);
        let has = label_names(&issue);
        let patch = |body: Value| {
            let path = &path;
            async move { self.send(reqwest::Method::PATCH, path, Some(&body)).await }
        };
        if state.id == CLOSED {
            if open {
                patch(json!({ "state": "closed", "state_reason": "completed" })).await?;
            }
            return Ok(());
        }
        if !open {
            patch(json!({ "state": "open" })).await?;
        }
        let wanted = state.id.strip_prefix(LABEL);
        for m in mapped {
            let Some(other) = m.id.strip_prefix(LABEL) else {
                continue;
            };
            if Some(other) != wanted && has.iter().any(|h| h == other) {
                self.send(
                    reqwest::Method::DELETE,
                    &format!("{path}/labels/{}", encode(other)),
                    None,
                )
                .await?;
            }
        }
        if let Some(w) = wanted.filter(|w| !has.iter().any(|h| h == w)) {
            self.send(
                reqwest::Method::POST,
                &format!("{path}/labels"),
                Some(&json!({ "labels": [w] })),
            )
            .await?;
        }
        Ok(())
    }

    pub async fn comment(&self, r: &ExternalRef, text: &str) -> Result<()> {
        self.send(
            reqwest::Method::POST,
            &format!("{}/comments", Self::issue_path(r)),
            Some(&json!({ "body": text })),
        )
        .await?;
        Ok(())
    }
}

use escouade_plugin_kit::forge::BoxFuture;
use escouade_plugin_kit::hosts::TicketSource;

impl TicketSource for Github {
    fn me(&self) -> BoxFuture<'_, Result<(String, String)>> {
        Box::pin(Github::me(self))
    }
    fn containers(&self) -> BoxFuture<'_, Result<Vec<Container>>> {
        Box::pin(Github::containers(self))
    }
    fn states<'a>(&'a self, container: &'a str) -> BoxFuture<'a, Result<Vec<ExternalState>>> {
        Box::pin(Github::states(self, container))
    }
    fn issues<'a>(&'a self, container: &'a str, q: &'a Query) -> BoxFuture<'a, Result<IssuePage>> {
        Box::pin(Github::issues(self, container, q))
    }
    fn set_state<'a>(
        &'a self,
        r: &'a ExternalRef,
        state: &'a ExternalState,
        mapped: &'a [ExternalState],
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(Github::set_state(self, r, state, mapped))
    }
    fn comment<'a>(&'a self, r: &'a ExternalRef, text: &'a str) -> BoxFuture<'a, Result<()>> {
        Box::pin(Github::comment(self, r, text))
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
        service: ID.into(),
        id: n.to_string(),
        key: format!("#{n}"),
        title: i["title"].as_str().unwrap_or_default().to_string(),
        kind: "Issue".into(),
        kind_code: "issue".into(),
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

    #[test]
    fn the_states_and_the_filter_of_github_read_in_english() {
        use escouade_plugin_kit::i18n::Lang::En;
        assert_eq!(
            fixed_states(En).map(|s| s.name),
            ["Open".to_string(), "Closed".to_string()]
        );
        assert_eq!(mine_filter(En).label, "Assigned to me");
        assert_eq!(unassigned(En), "Unassigned");
    }
}
