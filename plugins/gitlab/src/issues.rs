//! GitLab Issues (REST API v4), the Kanban's source on one instance's account: gitlab.com or a self-hosted one. It has
//! no account of its own: the code host's (one per instance, its token personal or `glab`'s) serves both uses. A
//! container is `<host>/<groups>/<project>`, so that the instance goes with it (the core asks the account whose host the
//! container names). An issue's state on the Kanban is opened or closed, or a label of the project (a board's lists are
//! labels: they come first).

use anyhow::{anyhow, Result};
use escouade_plugin_kit::errors::HttpError;
use escouade_plugin_kit::forge::BoxFuture;
use escouade_plugin_kit::hosts::TicketSource;
use escouade_plugin_kit::http::encode;
use escouade_plugin_kit::http::redirect;
use escouade_plugin_kit::http::reqwest;
use escouade_plugin_kit::i18n::{self, Lang};
use escouade_plugin_kit::protocol::tickets::{
    Container, ExternalIssue, ExternalRef, ExternalState, IssueFilter, IssuePage, Query,
};
use escouade_plugin_kit::text::criteria_of;
use escouade_plugin_kit::{tr, tr_in};
use reqwest::header::HeaderMap;
use serde_json::{json, Value};

/// The source's id.
pub const ID: &str = "gitlab";
const NAME: &str = "GitLab";

/// The states every project has, before its labels.
const OPEN: &str = "opened";
const CLOSED: &str = "closed";
const LABEL: &str = "label:";

/// GitLab Issues on one instance's account.
pub struct Gitlab {
    http: reqwest::Client,
    /// What the account is known by (`gitlab.com`, `git.acme.dev:8443`).
    host: String,
    /// Its API (`https://gitlab.com/api/v4`).
    base: String,
    token: String,
    /// The account's username: what "mine" filters on.
    user: String,
}

impl Gitlab {
    pub fn new(http: reqwest::Client, host: &str, base: &str, token: &str, user: &str) -> Self {
        Self {
            http,
            host: host.to_string(),
            base: base.trim_end_matches('/').to_string(),
            token: token.trim().to_string(),
            user: user.to_string(),
        }
    }

    /// The project's path of a container of this instance (`<host>/<path>`).
    fn of<'a>(&self, container: &'a str) -> Result<&'a str> {
        let (host, path) = container.split_once('/').unwrap_or((container, ""));
        if !host.eq_ignore_ascii_case(&self.host) {
            return Err(anyhow!(tr!(
                "Aucun compte GitLab connecté pour {host}",
                "No GitLab account connected for {host}"
            )));
        }
        Ok(path)
    }

    /// A call of the API, wherever it is redirected (an instance moved to https): its JSON and its headers.
    async fn send(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&Value>,
    ) -> Result<(Value, HeaderMap)> {
        let url = format!("{}{path}", self.base);
        redirect::send_named(NAME, method, url, body, |method, url, own, body| {
            let mut rb = self
                .http
                .request(method, url)
                .header("User-Agent", "escouade");
            if own {
                rb = rb.header("PRIVATE-TOKEN", &self.token);
            }
            match body {
                Some(body) => rb.json(body),
                None => rb,
            }
        })
        .await
    }

    async fn get(&self, path: &str) -> Result<(Value, HeaderMap)> {
        self.send(reqwest::Method::GET, path, None).await
    }

    /// The account, once its token is accepted.
    pub async fn me(&self) -> Result<(String, String)> {
        let (me, _) = self.get("/user").await?;
        let login = me["username"].as_str().unwrap_or("?").to_string();
        Ok((format!("@{login}"), login))
    }

    /// The projects the account is a member of, the last active first: their path keeps the groups and subgroups.
    pub async fn containers(&self) -> Result<Vec<Container>> {
        let (v, _) = self
            .get("/projects?membership=true&simple=true&archived=false&order_by=last_activity_at&per_page=100")
            .await?;
        Ok(v.as_array()
            .into_iter()
            .flatten()
            .filter_map(|p| p["path_with_namespace"].as_str())
            .map(|path| Container {
                id: format!("{}/{path}", self.host),
                name: path.to_string(),
                group: Some(self.host.clone()),
                own: false,
            })
            .collect())
    }

    async fn labels(&self, path: &str) -> Result<Vec<String>> {
        let (v, _) = self
            .get(&format!("/projects/{}/labels?per_page=100", encode(path)))
            .await?;
        Ok(v.as_array()
            .into_iter()
            .flatten()
            .filter_map(|l| l["name"].as_str().map(str::to_string))
            .collect())
    }

    /// The labels of the project's boards' lists, in their order; none when the instance gives none (an older one, no
    /// right to read them).
    async fn board_lists(&self, path: &str) -> Vec<String> {
        let Ok((v, _)) = self
            .get(&format!("/projects/{}/boards", encode(path)))
            .await
        else {
            return Vec::new();
        };
        let mut lists: Vec<(i64, String)> = v
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|b| b["lists"].as_array().cloned().unwrap_or_default())
            .filter_map(|l| {
                let name = l["label"]["name"].as_str()?.to_string();
                Some((l["position"].as_i64().unwrap_or(i64::MAX), name))
            })
            .collect();
        lists.sort_by_key(|(p, _)| *p);
        let mut out: Vec<String> = Vec::new();
        for (_, name) in lists {
            if !out.contains(&name) {
                out.push(name);
            }
        }
        out
    }

    /// Opened, closed, then the labels of the project's boards' lists, then its other labels.
    pub async fn states(&self, container: &str) -> Result<Vec<ExternalState>> {
        let path = self.of(container)?;
        let mut names = self.board_lists(path).await;
        for l in self.labels(path).await? {
            if !names.contains(&l) {
                names.push(l);
            }
        }
        let mut out = fixed_states(i18n::ui()).to_vec();
        out.extend(names.into_iter().map(|name| ExternalState {
            id: format!("{LABEL}{name}"),
            name,
        }));
        Ok(out)
    }

    /// The opened issues: by number (`#12`), else listed, searched, assigned to me or carrying a label as asked; a
    /// label's chip keeps those that carry one of the chosen. A page at a time: `X-Next-Page` says when there is a next
    /// one, `X-Total` how many match.
    pub async fn issues(&self, container: &str, q: &Query) -> Result<IssuePage> {
        let path = self.of(container)?;
        let project = encode(path);
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
        let (items, next, total): (Vec<Value>, Option<String>, Option<u64>) = if number {
            let items = match self
                .get(&format!("/projects/{project}/issues/{text}"))
                .await
            {
                Ok((i, _)) if i["state"].as_str() == Some(OPEN) => vec![i],
                Ok(_) => Vec::new(),
                Err(e)
                    if e.downcast_ref::<HttpError>()
                        .is_some_and(|h| h.status == 404) =>
                {
                    Vec::new()
                }
                Err(e) => return Err(e),
            };
            (items, None, None)
        } else {
            let mut url =
                format!("/projects/{project}/issues?state=opened&per_page=50&page={page}");
            if mine {
                url.push_str(&format!("&assignee_username={}", encode(&self.user)));
            }
            if let Some(l) = label {
                // A comma separates labels: the label is one.
                url.push_str(&format!("&labels={}", encode(&l.replace(',', " "))));
            }
            if !text.is_empty() {
                url.push_str(&format!("&search={}", encode(text)));
            }
            let (v, headers) = self.get(&url).await?;
            let header = |name: &str| {
                headers
                    .get(name)
                    .and_then(|h| h.to_str().ok())
                    .map(str::trim)
                    .filter(|h| !h.is_empty())
                    .map(str::to_string)
            };
            (
                v.as_array().cloned().unwrap_or_default(),
                header("x-next-page"),
                header("x-total").and_then(|t| t.parse().ok()),
            )
        };
        let chosen: Vec<&str> = q
            .filters
            .iter()
            .filter_map(|f| f.strip_prefix(LABEL))
            .collect();
        let issues = items
            .iter()
            .filter(|i| {
                chosen.is_empty() || label_names(i).iter().any(|l| chosen.contains(&l.as_str()))
            })
            .map(|i| issue(container, i))
            .collect();
        let mut filters = vec![mine_filter(i18n::ui())];
        filters.extend(
            self.labels(path)
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
            next,
            // The count is the instance's, before a label's chip keeps some here.
            total: total.filter(|_| chosen.is_empty()),
        })
    }

    fn issue_path(path: &str, r: &ExternalRef) -> String {
        format!("/projects/{}/issues/{}", encode(path), encode(&r.id))
    }

    /// Closed, opened again, or given its label (opened again if need be) without the other labels the link gives a
    /// column: in one call.
    pub async fn set_state(
        &self,
        r: &ExternalRef,
        state: &ExternalState,
        mapped: &[ExternalState],
    ) -> Result<()> {
        let path = self.of(&r.container)?;
        let url = Self::issue_path(path, r);
        let (issue, _) = self.get(&url).await?;
        let open = issue["state"].as_str() != Some(CLOSED);
        let has = label_names(&issue);
        let mut body = serde_json::Map::new();
        if state.id == CLOSED {
            if open {
                body.insert("state_event".into(), json!("close"));
            }
        } else {
            if !open {
                body.insert("state_event".into(), json!("reopen"));
            }
            let wanted = state.id.strip_prefix(LABEL);
            let remove: Vec<&str> = mapped
                .iter()
                .filter_map(|m| m.id.strip_prefix(LABEL))
                .filter(|other| Some(*other) != wanted && has.iter().any(|h| h == other))
                .collect();
            if !remove.is_empty() {
                body.insert("remove_labels".into(), json!(remove.join(",")));
            }
            if let Some(w) = wanted.filter(|w| !has.iter().any(|h| h == w)) {
                body.insert("add_labels".into(), json!(w));
            }
        }
        if body.is_empty() {
            return Ok(());
        }
        self.send(reqwest::Method::PUT, &url, Some(&Value::Object(body)))
            .await?;
        Ok(())
    }

    /// A note on the issue, as markdown.
    pub async fn comment(&self, r: &ExternalRef, text: &str) -> Result<()> {
        let path = self.of(&r.container)?;
        self.send(
            reqwest::Method::POST,
            &format!("{}/notes", Self::issue_path(path, r)),
            Some(&json!({ "body": text })),
        )
        .await?;
        Ok(())
    }
}

impl TicketSource for Gitlab {
    fn me(&self) -> BoxFuture<'_, Result<(String, String)>> {
        Box::pin(Gitlab::me(self))
    }
    fn containers(&self) -> BoxFuture<'_, Result<Vec<Container>>> {
        Box::pin(Gitlab::containers(self))
    }
    fn states<'a>(&'a self, container: &'a str) -> BoxFuture<'a, Result<Vec<ExternalState>>> {
        Box::pin(Gitlab::states(self, container))
    }
    fn issues<'a>(&'a self, container: &'a str, q: &'a Query) -> BoxFuture<'a, Result<IssuePage>> {
        Box::pin(Gitlab::issues(self, container, q))
    }
    fn set_state<'a>(
        &'a self,
        r: &'a ExternalRef,
        state: &'a ExternalState,
        mapped: &'a [ExternalState],
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(Gitlab::set_state(self, r, state, mapped))
    }
    fn comment<'a>(&'a self, r: &'a ExternalRef, text: &'a str) -> BoxFuture<'a, Result<()>> {
        Box::pin(Gitlab::comment(self, r, text))
    }
}

/// Its labels: names, or objects with a name (`with_labels_details`).
fn label_names(i: &Value) -> Vec<String> {
    i["labels"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|l| l.as_str().or_else(|| l["name"].as_str()))
        .map(str::to_string)
        .collect()
}

/// An issue's states that every project has, before its labels: opened, closed.
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

fn by(lang: Lang, author: &str) -> String {
    tr_in!(lang, "par @{author}", "by @{author}")
}

/// Its type as shown, and what the window colours it by.
fn kind_of(lang: Lang, issue_type: &str) -> (String, &'static str) {
    match issue_type {
        "incident" => ("Incident".into(), "bug"),
        "task" => (tr_in!(lang, "Tâche", "Task"), "task"),
        _ => ("Issue".into(), "issue"),
    }
}

fn issue(container: &str, i: &Value) -> ExternalIssue {
    let lang = i18n::ui();
    let n = i["iid"].as_u64().unwrap_or_default();
    let body = i["description"].as_str().unwrap_or_default().to_string();
    let mut meta = label_names(i);
    let assignee = i["assignees"][0]["username"]
        .as_str()
        .or_else(|| i["assignee"]["username"].as_str());
    meta.push(
        assignee
            .map(|l| format!("@{l}"))
            .unwrap_or_else(|| unassigned(lang)),
    );
    if let Some(a) = i["author"]["username"].as_str() {
        meta.push(by(lang, a));
    }
    let (kind, kind_code) = kind_of(lang, i["issue_type"].as_str().unwrap_or("issue"));
    ExternalIssue {
        service: ID.into(),
        id: n.to_string(),
        key: format!("#{n}"),
        title: i["title"].as_str().unwrap_or_default().to_string(),
        kind,
        kind_code: kind_code.into(),
        meta,
        url: i["web_url"].as_str().unwrap_or_default().to_string(),
        criteria: criteria_of(&body),
        description: body,
        container: container.to_string(),
        imported: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_states_and_the_filter_of_gitlab_read_in_english() {
        use escouade_plugin_kit::i18n::Lang::En;
        assert_eq!(
            fixed_states(En).map(|s| s.name),
            ["Open".to_string(), "Closed".to_string()]
        );
        assert_eq!(mine_filter(En).label, "Assigned to me");
        assert_eq!(unassigned(En), "Unassigned");
        assert_eq!(by(En, "grace"), "by @grace");
        assert_eq!(kind_of(En, "task"), ("Task".to_string(), "task"));
    }

    #[test]
    fn a_container_of_another_instance_is_not_this_accounts() {
        let g = Gitlab::new(
            reqwest::Client::new(),
            "gitlab.com",
            "https://gitlab.com/api/v4/",
            " glpat-x ",
            "ada",
        );
        assert_eq!(g.of("GitLab.com/acme/back/api").unwrap(), "acme/back/api");
        let e = g.of("gitlab.other.org/x/y").unwrap_err();
        assert!(e.to_string().contains("gitlab.other.org"), "{e}");
    }
}
