//! ClickUp (API v2), with a personal API token: the Kanban's source of ClickUp tasks. A project's source is a list,
//! chosen through the hierarchy (workspace, space, folder, list); a task's state on the Kanban is one of the list's
//! statuses, which each list has its own. The account's secret is the token (`secrets/get` of `account:clickup`).

use anyhow::{anyhow, Result};
use escouade_plugin_kit::errors::{http_error_in, HttpError, Refused};
use escouade_plugin_kit::forge::BoxFuture;
use escouade_plugin_kit::hosts::{Credentials, SourceKind, StatesView, TicketSource};
use escouade_plugin_kit::http::encode;
use escouade_plugin_kit::http::reqwest;
use escouade_plugin_kit::i18n::{self, Lang};
use escouade_plugin_kit::protocol::tickets::{
    Container, ExternalIssue, ExternalRef, ExternalState, IssueFilter, IssuePage, Query,
};
use escouade_plugin_kit::text::{criteria_of, names_criteria};
use escouade_plugin_kit::{tr, tr_in};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::time::Duration;

/// The source's id, and its name in what it says.
pub const ID: &str = "clickup";
const S: &str = "ClickUp";

/// Where the API is, unless its account says otherwise.
const API: &str = "https://api.clickup.com/api/v2";
/// The prefix of a status's chip in the import.
const STATUS: &str = "status:";
/// The pages a search by title reads before it shows what it found (ClickUp has no search by
/// title: the tasks are read a page at a time, 100 by 100).
const SEARCH_PAGES: u32 = 5;
/// The longest wait for the limit of requests to reset (`X-RateLimit-Reset`) before asking again;
/// beyond it, the limit is said at once.
const WAIT_MAX: Duration = Duration::from_secs(60);
/// The wait after a 429 that says nothing of its reset.
const WAIT_DEFAULT: Duration = Duration::from_secs(2);

pub struct Clickup {
    http: reqwest::Client,
    /// The API's address.
    base: String,
    key: String,
    /// The account's user id: what « Mes tâches » filters on.
    user: String,
}

/// An answer: its JSON, or the wait the limit of requests asks for (429).
enum Answer {
    Done(Value),
    Limited(Duration),
}

impl Clickup {
    pub fn new(http: reqwest::Client, a: &Credentials) -> Self {
        Self {
            http,
            base: a
                .api
                .as_deref()
                .filter(|b| !b.trim().is_empty())
                .unwrap_or(API)
                .trim_end_matches('/')
                .to_string(),
            key: a.token.trim().to_string(),
            user: a.user.clone(),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base)
    }

    /// Sends a request; a 429 waits for the limit's reset (`X-RateLimit-Reset`, in seconds since
    /// 1970) and asks once more.
    async fn send(&self, rb: reqwest::RequestBuilder) -> Result<Value> {
        // A personal API token goes as it is, with no scheme.
        let rb = rb
            .header("Authorization", &self.key)
            .header("User-Agent", "escouade");
        let again = rb.try_clone();
        match self.once(rb).await? {
            Answer::Done(v) => Ok(v),
            Answer::Limited(wait) => match again {
                Some(rb) if wait <= WAIT_MAX => {
                    escouade_plugin_kit::time::sleep(wait).await;
                    match self.once(rb).await? {
                        Answer::Done(v) => Ok(v),
                        Answer::Limited(_) => Err(limited()),
                    }
                }
                _ => Err(limited()),
            },
        }
    }

    async fn once(&self, rb: reqwest::RequestBuilder) -> Result<Answer> {
        let resp = rb.send().await.map_err(|e| {
            let e = anyhow::Error::from(e.without_url());
            anyhow!(tr!(
                "{S} injoignable : {e:#}",
                "{S} can’t be reached: {e:#}"
            ))
        })?;
        let status = resp.status().as_u16();
        if status == 429 {
            let reset = resp
                .headers()
                .get("x-ratelimit-reset")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.trim().parse::<i64>().ok());
            return Ok(Answer::Limited(wait_until(reset, now_ms())));
        }
        let text = resp.text().await.unwrap_or_default();
        if !(200..300).contains(&status) {
            return Err(HttpError {
                status,
                message: http_error_in(i18n::ui(), S, status, &text),
            }
            .into());
        }
        if text.trim().is_empty() {
            return Ok(Answer::Done(Value::Null));
        }
        let v = serde_json::from_str(&text)
            .map_err(|_| anyhow!(tr!("{S} : réponse illisible", "{S}: unreadable answer")))?;
        Ok(Answer::Done(v))
    }

    async fn get(&self, path: &str, query: &[(&str, String)]) -> Result<Value> {
        self.send(self.http.get(self.url(path)).query(query)).await
    }

    /// The account's name (its e-mail when it has none) and its user id.
    pub async fn me(&self) -> Result<(String, String)> {
        let v = self.get("/user", &[]).await?;
        let me = &v["user"];
        let label = [&me["username"], &me["email"]]
            .iter()
            .find_map(|v| v.as_str().filter(|s| !s.trim().is_empty()))
            .unwrap_or("?")
            .to_string();
        Ok((label, id_of(&me["id"])))
    }

    /// The lists of every workspace the token reaches, each in its place: « Space › Folder »
    /// (the workspace before it when there are several).
    pub async fn containers(&self) -> Result<Vec<Container>> {
        let teams = self.get("/team", &[]).await?;
        let teams: Vec<&Value> = teams["teams"].as_array().into_iter().flatten().collect();
        let open = [("archived", "false".to_string())];
        let mut out = Vec::new();
        for team in &teams {
            let spaces = self
                .get(&format!("/team/{}/space", id_of(&team["id"])), &open)
                .await?;
            for space in spaces["spaces"].as_array().into_iter().flatten() {
                let mut place = Vec::new();
                if teams.len() > 1 {
                    place.push(name_of(team));
                }
                place.push(name_of(space));
                let id = id_of(&space["id"]);
                let folders = self.get(&format!("/space/{id}/folder"), &open).await?;
                let lists = self.get(&format!("/space/{id}/list"), &open).await?;
                // The lists outside a folder first, as ClickUp's sidebar shows them.
                for list in lists["lists"].as_array().into_iter().flatten() {
                    out.push(container(list, &place));
                }
                for folder in folders["folders"].as_array().into_iter().flatten() {
                    let mut place = place.clone();
                    place.push(name_of(folder));
                    for list in folder["lists"].as_array().into_iter().flatten() {
                        out.push(container(list, &place));
                    }
                }
            }
        }
        Ok(out)
    }

    /// The list's statuses with their type (`open`, `custom`, `done`, `closed`), in its order. A
    /// list gone or out of the token's reach says so.
    async fn statuses(&self, list: &str) -> Result<(String, Vec<(ExternalState, String)>)> {
        let v = match self.get(&format!("/list/{}", encode(list)), &[]).await {
            Ok(v) => v,
            Err(e) => return Err(self.list_lost(list, e).await),
        };
        let mut statuses: Vec<&Value> = v["statuses"].as_array().into_iter().flatten().collect();
        statuses.sort_by_key(|s| s["orderindex"].as_i64().unwrap_or(0));
        let statuses = statuses
            .into_iter()
            .filter_map(|s| {
                let name = s["status"].as_str()?.to_string();
                Some((
                    ExternalState {
                        id: name.clone(),
                        name,
                    },
                    s["type"].as_str().unwrap_or_default().to_string(),
                ))
            })
            .collect();
        Ok((name_of(&v), statuses))
    }

    /// A list ClickUp does not give: gone (404), or refused while the token is good (the list
    /// left the token's reach). Either reads as the list lost, which asking again would not
    /// change; another error stays as it is.
    async fn list_lost(&self, list: &str, e: anyhow::Error) -> anyhow::Error {
        let status = e.downcast_ref::<HttpError>().map(|h| h.status);
        let lost = match status {
            Some(404) => true,
            Some(401 | 403) => self.get("/user", &[]).await.is_ok(),
            _ => false,
        };
        if !lost {
            return e;
        }
        HttpError {
            status: 404,
            message: tr!(
                "La liste {S} {list} n'existe plus ou n'est plus accessible : choisis-en une autre dans Réglages › Projets › Tickets importés",
                "The {S} list {list} no longer exists or can’t be reached any more: choose another one in Settings › Projects › Imported tickets"
            ),
        }
        .into()
    }

    pub async fn states(&self, list: &str) -> Result<Vec<ExternalState>> {
        Ok(self
            .statuses(list)
            .await?
            .1
            .into_iter()
            .map(|(s, _)| s)
            .collect())
    }

    /// The list's statuses, and those each column gives by default: by their name as for the
    /// other services, else by their type (`defaults_by_type`).
    pub async fn states_view(&self, list: &str) -> Result<StatesView> {
        let (_, statuses) = self.statuses(list).await?;
        let defaults = defaults_by_type(&statuses);
        Ok(StatesView {
            states: statuses.into_iter().map(|(s, _)| s).collect(),
            defaults,
        })
    }

    /// The list's open tasks, the latest updated first, 100 a page; "mine" keeps those assigned
    /// to me, a status's chip those in it. A text that names a task (`#86abx3`, `86abx3`, or a
    /// custom id like `ABC-12`) finds it; another, the tasks whose name holds it.
    pub async fn issues(&self, list: &str, q: &Query) -> Result<IssuePage> {
        let (_, statuses) = self.statuses(list).await?;
        let lang = i18n::ui();
        let mut filters = vec![mine_filter(lang)];
        filters.extend(
            statuses
                .iter()
                .filter(|(_, kind)| !is_closed(kind))
                .map(|(s, _)| IssueFilter {
                    id: format!("{STATUS}{}", s.id),
                    label: s.name.clone(),
                }),
        );
        let text = q.text.trim();
        if q.page.is_none() {
            if let Some(task) = self.lookup(list, text).await? {
                let issues = [task]
                    .iter()
                    .filter(|t| !is_closed(t["status"]["type"].as_str().unwrap_or_default()))
                    .map(|t| issue_of(t, list, lang))
                    .collect();
                return Ok(IssuePage {
                    issues,
                    filters,
                    next: None,
                    total: None,
                });
            }
        }
        let mut page: u32 = q.page.as_deref().and_then(|p| p.parse().ok()).unwrap_or(0);
        let needle = text.to_lowercase();
        let mut found = Vec::new();
        let mut read = 0;
        let next = loop {
            let v = self
                .get(
                    &format!("/list/{}/task", encode(list)),
                    &self.task_query(q, page),
                )
                .await?;
            read += 1;
            found.extend(
                v["tasks"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|t| !is_closed(t["status"]["type"].as_str().unwrap_or_default()))
                    .filter(|t| {
                        needle.is_empty()
                            || t["name"]
                                .as_str()
                                .unwrap_or_default()
                                .to_lowercase()
                                .contains(&needle)
                    })
                    .map(|t| issue_of(t, list, lang)),
            );
            let last = v["last_page"] != false
                || v["tasks"].as_array().is_none_or(|tasks| tasks.is_empty());
            page += 1;
            if last {
                break None;
            }
            if needle.is_empty() || !found.is_empty() || read >= SEARCH_PAGES {
                break Some(page.to_string());
            }
        };
        Ok(IssuePage {
            issues: found,
            filters,
            next,
            total: None,
        })
    }

    /// The query of a page of the list's tasks.
    fn task_query(&self, q: &Query, page: u32) -> Vec<(&'static str, String)> {
        let mut query = vec![
            ("archived", "false".to_string()),
            ("include_closed", "false".to_string()),
            ("include_markdown_description", "true".to_string()),
            ("subtasks", "true".to_string()),
            ("order_by", "updated".to_string()),
            ("page", page.to_string()),
        ];
        if q.filters.iter().any(|f| f == "mine") && !self.user.is_empty() {
            query.push(("assignees[]", self.user.clone()));
        }
        for status in q.filters.iter().filter_map(|f| f.strip_prefix(STATUS)) {
            query.push(("statuses[]", status.to_string()));
        }
        if let Some(tag) = q.label.as_deref().map(str::trim).filter(|l| !l.is_empty()) {
            // ClickUp keeps its tags in lower case.
            query.push(("tags[]", tag.to_lowercase()));
        }
        query
    }

    /// The task a text names, when it names one of the list: a task's id (`#86abx3`, or
    /// `86abx3`), a custom id (`ABC-12`). None when the text names none, or none ClickUp knows:
    /// the text is then searched in the names.
    async fn lookup(&self, list: &str, text: &str) -> Result<Option<Value>> {
        let found = match id_kind(text) {
            None => return Ok(None),
            Some(Named::Task(id)) => self.task(id, &[]).await?,
            Some(Named::Custom(id)) => {
                let teams = self.get("/team", &[]).await?;
                let mut found = None;
                for team in teams["teams"].as_array().into_iter().flatten() {
                    let query = [
                        ("custom_task_ids", "true".to_string()),
                        ("team_id", id_of(&team["id"])),
                    ];
                    if let Some(t) = self.task(id, &query).await? {
                        found = Some(t);
                        break;
                    }
                }
                found
            }
        };
        Ok(found.filter(|t| id_of(&t["list"]["id"]) == list))
    }

    /// A task, or none when ClickUp does not give it (gone, or not one the token reaches).
    async fn task(&self, id: &str, query: &[(&str, String)]) -> Result<Option<Value>> {
        let mut query = query.to_vec();
        query.push(("include_markdown_description", "true".to_string()));
        match self.get(&format!("/task/{}", encode(id)), &query).await {
            Ok(v) => Ok(Some(v)),
            Err(e)
                if e.downcast_ref::<HttpError>()
                    .is_some_and(|h| matches!(h.status, 400 | 401 | 403 | 404)) =>
            {
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    /// The task takes the status, when its list has it: one it lacks (renamed, removed, or the
    /// list's statuses changed) says so rather than fail without a word.
    pub async fn set_state(&self, r: &ExternalRef, state: &ExternalState) -> Result<()> {
        let (list, statuses) = self.statuses(&r.container).await?;
        let Some((known, _)) = statuses
            .iter()
            .find(|(s, _)| s.name.eq_ignore_ascii_case(&state.name))
        else {
            let (status, key) = (&state.name, &r.key);
            return Err(Refused(tr!(
                "La liste « {list} » de ClickUp n'a pas le statut « {status} » : {key} reste où il est ; choisis un autre statut dans Réglages › Projets › Tickets importés",
                "The ClickUp list “{list}” has no “{status}” status: {key} stays where it is; choose another status in Settings › Projects › Imported tickets"
            ))
            .into());
        };
        self.send(
            self.http
                .put(self.url(&format!("/task/{}", encode(&r.id))))
                .json(&json!({ "status": known.name })),
        )
        .await?;
        Ok(())
    }

    pub async fn comment(&self, r: &ExternalRef, text: &str) -> Result<()> {
        self.send(
            self.http
                .post(self.url(&format!("/task/{}/comment", encode(&r.id))))
                .json(&json!({ "comment_text": text, "notify_all": false })),
        )
        .await?;
        Ok(())
    }
}

/// What a text names: a task's id, or a custom one.
#[derive(Debug, PartialEq)]
enum Named<'a> {
    Task(&'a str),
    Custom(&'a str),
}

/// `#86abx3` and `86abx3` (letters and digits, both, at least 6) name a task's id; `ABC-12` a
/// custom id; other texts, none.
fn id_kind(text: &str) -> Option<Named<'_>> {
    let text = text.trim();
    if let Some((prefix, n)) = text.split_once('-') {
        let custom = prefix
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic())
            && prefix.chars().all(|c| c.is_ascii_alphanumeric())
            && !n.is_empty()
            && n.chars().all(|c| c.is_ascii_digit());
        return custom.then_some(Named::Custom(text));
    }
    let (hashed, id) = match text.strip_prefix('#') {
        Some(id) => (true, id),
        None => (false, text),
    };
    let alnum = !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric());
    let looks = id.len() >= 6
        && id.chars().any(|c| c.is_ascii_digit())
        && id.chars().any(|c| c.is_ascii_alphabetic());
    (alnum && (hashed || looks)).then_some(Named::Task(id))
}

/// A closed status: `closed`, or `done` (finished, not yet closed).
fn is_closed(kind: &str) -> bool {
    matches!(kind, "closed" | "done")
}

/// What looks like « en cours », then « à tester », by priority (the names Escouade guesses the columns' states by).
const DOING: &[&str] = &[
    "in-progress",
    "en-cours",
    "doing",
    "wip",
    "in-development",
    "en-developpement",
    "progress",
    "started",
];
const TO_TEST: &[&str] = &[
    "a-tester",
    "to-test",
    "to-be-tested",
    "testing",
    "qa",
    "in-review",
    "review",
    "recette",
    "a-valider",
    "validation",
    "test",
];

/// A name as a kebab-case ASCII slug (accents folded).
fn slug(s: &str) -> String {
    let folded: String = s
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ä' | 'ã' => 'a',
            'ç' => 'c',
            'è' | 'é' | 'ê' | 'ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' => 'i',
            'ò' | 'ó' | 'ô' | 'ö' | 'õ' => 'o',
            'ù' | 'ú' | 'û' | 'ü' => 'u',
            'ÿ' => 'y',
            'ñ' => 'n',
            c if c.is_ascii_alphanumeric() => c,
            _ => '-',
        })
        .collect();
    folded
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// The first state whose name holds one of `patterns` (as whole words), in their order.
fn looks_like<'a>(states: &'a [ExternalState], patterns: &[&str]) -> Option<&'a ExternalState> {
    patterns.iter().find_map(|p| {
        states.iter().find(|s| {
            let slug = slug(&s.name);
            slug == *p
                || slug.starts_with(&format!("{p}-"))
                || slug.ends_with(&format!("-{p}"))
                || slug.contains(&format!("-{p}-"))
        })
    })
}

/// The defaults of the columns (`doing`, `review`, `done`): by the statuses' names as for the other services (the one
/// that looks like « en cours » for « En cours » and « À tester », the one that looks like « à tester » for
/// « Terminé »); else « En cours » and « À tester » take the first status of the `custom` type (between the open one
/// and the closed ones), « Terminé » the next one, else the first of the `done` type. A list with no such status
/// leaves the column unchanged.
fn defaults_by_type(statuses: &[(ExternalState, String)]) -> BTreeMap<String, ExternalState> {
    let states: Vec<ExternalState> = statuses.iter().map(|(s, _)| s.clone()).collect();
    let mut out = BTreeMap::new();
    let doing = looks_like(&states, DOING);
    if let Some(d) = doing {
        out.insert("doing".to_string(), d.clone());
        out.insert("review".to_string(), d.clone());
    }
    let others: Vec<ExternalState> = states
        .iter()
        .filter(|s| Some(*s) != doing)
        .cloned()
        .collect();
    if let Some(t) = looks_like(&others, TO_TEST) {
        out.insert("done".to_string(), t.clone());
    }
    let custom: Vec<&ExternalState> = statuses
        .iter()
        .filter(|(_, k)| k == "custom")
        .map(|(s, _)| s)
        .collect();
    if !out.contains_key("doing") {
        if let Some(first) = custom.first() {
            out.insert("doing".to_string(), (*first).clone());
            out.insert("review".to_string(), (*first).clone());
        }
    }
    if !out.contains_key("done") {
        let doing = out.get("doing").cloned();
        let next = custom
            .iter()
            .find(|s| Some(**s) != doing.as_ref())
            .map(|s| (*s).clone())
            .or_else(|| {
                statuses
                    .iter()
                    .find(|(_, k)| k == "done")
                    .map(|(s, _)| s.clone())
            });
        if let Some(next) = next {
            out.insert("done".to_string(), next);
        }
    }
    out
}

/// A task as the import lists it.
fn issue_of(t: &Value, list: &str, lang: Lang) -> ExternalIssue {
    let description = ["markdown_description", "description", "text_content"]
        .iter()
        .find_map(|k| t[*k].as_str().filter(|s| !s.trim().is_empty()))
        .unwrap_or_default()
        .to_string();
    let id = id_of(&t["id"]);
    let key = t["custom_id"]
        .as_str()
        .filter(|c| !c.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("#{id}"));
    let mut meta: Vec<String> = [
        t["status"]["status"].as_str().map(str::to_string),
        t["priority"]["priority"].as_str().map(str::to_string),
    ]
    .into_iter()
    .flatten()
    .collect();
    meta.extend(
        t["assignees"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|a| a["username"].as_str().map(str::to_string)),
    );
    meta.extend(
        t["tags"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|l| l["name"].as_str().map(str::to_string)),
    );
    let checklists: Vec<&Value> = t["checklists"].as_array().into_iter().flatten().collect();
    ExternalIssue {
        service: ID.to_string(),
        key,
        title: t["name"].as_str().unwrap_or_default().to_string(),
        kind: task(lang),
        kind_code: "task".into(),
        meta: meta.into_iter().filter(|m| !m.is_empty()).collect(),
        url: t["url"].as_str().unwrap_or_default().to_string(),
        criteria: task_criteria(&checklists, &description),
        description,
        container: list.to_string(),
        imported: false,
        id,
    }
}

/// The criteria of a task: the items of its checklist named after them, else of all its
/// checklists, else those its description lists.
fn task_criteria(checklists: &[&Value], description: &str) -> Vec<String> {
    let items = |lists: Vec<&&Value>| -> Vec<String> {
        lists
            .iter()
            .flat_map(|c| {
                let mut items = c["items"].as_array().cloned().unwrap_or_default();
                items.sort_by_key(|i| i["orderindex"].as_f64().unwrap_or(0.0) as i64);
                items
            })
            .filter_map(|i| i["name"].as_str().map(|n| n.trim().to_string()))
            .filter(|n| !n.is_empty())
            .collect()
    };
    let named: Vec<&&Value> = checklists
        .iter()
        .filter(|c| names_criteria(c["name"].as_str().unwrap_or("")))
        .collect();
    let mut found = items(named);
    if found.is_empty() {
        found = items(checklists.iter().collect());
    }
    if found.is_empty() {
        found = criteria_of(description);
    }
    found.truncate(20);
    found
}

/// An id, given as a string or a number.
fn id_of(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => String::new(),
    }
}

fn name_of(v: &Value) -> String {
    v["name"].as_str().unwrap_or_default().to_string()
}

fn container(list: &Value, place: &[String]) -> Container {
    Container {
        id: id_of(&list["id"]),
        name: name_of(list),
        group: Some(place.join(" › ")),
        own: false,
    }
}

/// How long to wait for a limit that resets at `reset` (seconds since 1970), `now` in
/// milliseconds: until then, or `WAIT_DEFAULT` when the answer did not say.
fn wait_until(reset: Option<i64>, now: i64) -> Duration {
    match reset {
        Some(r) => Duration::from_millis((r.saturating_mul(1000) - now).max(0) as u64),
        None => WAIT_DEFAULT,
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn limited() -> anyhow::Error {
    HttpError {
        status: 429,
        message: http_error_in(i18n::ui(), S, 429, ""),
    }
    .into()
}

/// What a ClickUp task is called in the import's list.
fn task(lang: Lang) -> String {
    tr_in!(lang, "Tâche", "Task")
}

/// The filter of the tasks assigned to the account.
fn mine_filter(lang: Lang) -> IssueFilter {
    IssueFilter {
        id: "mine".into(),
        label: tr_in!(lang, "Mes tâches", "My tasks"),
    }
}

impl TicketSource for Clickup {
    fn me(&self) -> BoxFuture<'_, Result<(String, String)>> {
        Box::pin(Clickup::me(self))
    }
    fn containers(&self) -> BoxFuture<'_, Result<Vec<Container>>> {
        Box::pin(Clickup::containers(self))
    }
    fn states<'a>(&'a self, container: &'a str) -> BoxFuture<'a, Result<Vec<ExternalState>>> {
        Box::pin(Clickup::states(self, container))
    }
    /// ClickUp's own view of the states: the defaults follow the statuses' types too.
    fn states_view<'a>(&'a self, container: &'a str) -> BoxFuture<'a, Result<Option<StatesView>>> {
        Box::pin(async move { Clickup::states_view(self, container).await.map(Some) })
    }
    fn issues<'a>(&'a self, container: &'a str, q: &'a Query) -> BoxFuture<'a, Result<IssuePage>> {
        Box::pin(Clickup::issues(self, container, q))
    }
    fn set_state<'a>(
        &'a self,
        r: &'a ExternalRef,
        state: &'a ExternalState,
        _mapped: &'a [ExternalState],
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(Clickup::set_state(self, r, state))
    }
    fn comment<'a>(&'a self, r: &'a ExternalRef, text: &'a str) -> BoxFuture<'a, Result<()>> {
        Box::pin(Clickup::comment(self, r, text))
    }
}

/// The Kanban's source, ClickUp.
pub struct ClickupSource;

impl SourceKind for ClickupSource {
    fn id(&self) -> &str {
        ID
    }
    fn name(&self) -> &str {
        S
    }
    fn client(&self, http: reqwest::Client, account: &Credentials) -> Box<dyn TicketSource> {
        Box::new(Clickup::new(http, account))
    }
}

/// What its module runs, on its first turn.
pub fn run() -> i32 {
    escouade_plugin_kit::serve::run(
        "clickup",
        escouade_plugin_kit::hosts::Hosts::new(vec![], vec![Box::new(ClickupSource)]),
    )
}

escouade_plugin_kit::protocol::module!(run);

#[cfg(test)]
mod tests {
    use super::*;

    fn s(name: &str, kind: &str) -> (ExternalState, String) {
        (
            ExternalState {
                id: name.into(),
                name: name.into(),
            },
            kind.to_string(),
        )
    }

    #[test]
    fn the_wait_lasts_until_the_reset() {
        assert_eq!(wait_until(Some(1_000), 999_500), Duration::from_millis(500));
        assert_eq!(wait_until(Some(1_000), 1_200_000), Duration::ZERO);
        assert_eq!(wait_until(None, 0), WAIT_DEFAULT);
    }

    #[test]
    fn statuses_whose_names_say_nothing_get_their_defaults_by_type() {
        let d = defaults_by_type(&[
            s("ouvert", "open"),
            s("chez Ada", "custom"),
            s("chez Bob", "custom"),
            s("fini", "closed"),
        ]);
        assert_eq!(d["doing"].name, "chez Ada");
        assert_eq!(d["review"].name, "chez Ada");
        assert_eq!(d["done"].name, "chez Bob");
        // One custom status: « Terminé » takes the `done` one.
        let d = defaults_by_type(&[
            s("ouvert", "open"),
            s("chez Ada", "custom"),
            s("livré", "done"),
            s("fini", "closed"),
        ]);
        assert_eq!(d["done"].name, "livré");
        // Open and closed only: no column moves the task.
        assert!(defaults_by_type(&[s("ouvert", "open"), s("fini", "closed")]).is_empty());
    }

    #[test]
    fn the_names_give_their_defaults_first() {
        let d = defaults_by_type(&[
            s("to do", "open"),
            s("in progress", "custom"),
            s("in review", "custom"),
            s("shipped", "done"),
            s("complete", "closed"),
        ]);
        assert_eq!(d["doing"].name, "in progress");
        assert_eq!(d["review"].name, "in progress");
        assert_eq!(d["done"].name, "in review");
        assert!(!d.contains_key("todo"));
        // Accents are folded: « À tester » is « a-tester ».
        let d = defaults_by_type(&[s("À tester", "custom"), s("En cours", "custom")]);
        assert_eq!(d["doing"].name, "En cours");
        assert_eq!(d["done"].name, "À tester");
    }

    #[test]
    fn without_a_checklist_the_criteria_come_from_the_description() {
        let t = json!({
            "id": "86abx3", "name": "Mode sombre", "checklists": [],
            "markdown_description": "## Critères\n- Bascule persistée\n- Contrastes AA",
        });
        let i = issue_of(&t, "l1", Lang::Fr);
        assert_eq!(i.criteria, ["Bascule persistée", "Contrastes AA"]);
        // Without a custom id, its key is its id.
        assert_eq!(i.key, "#86abx3");
    }

    #[test]
    fn a_text_names_a_task_by_its_id_or_its_custom_id() {
        assert_eq!(id_kind("#86abx3"), Some(Named::Task("86abx3")));
        assert_eq!(id_kind("#9hz"), Some(Named::Task("9hz")));
        assert_eq!(id_kind("86abx3"), Some(Named::Task("86abx3")));
        assert_eq!(id_kind(" ABC-12 "), Some(Named::Custom("ABC-12")));
        assert_eq!(id_kind("export"), None);
        assert_eq!(id_kind("utf8"), None);
        assert_eq!(id_kind("mode sombre"), None);
        assert_eq!(id_kind("v2-beta"), None);
        assert_eq!(id_kind(""), None);
    }

    #[test]
    fn the_filter_of_clickup_reads_in_english() {
        assert_eq!(mine_filter(Lang::En).label, "My tasks");
        assert_eq!(task(Lang::En), "Task");
    }

    #[test]
    fn the_api_is_the_one_the_core_says_else_clickups_and_the_key_goes_trimmed() {
        let account = Credentials {
            token: " pk_42_k1 ".into(),
            api: Some("http://localhost:4101/".into()),
            ..Default::default()
        };
        let c = Clickup::new(reqwest::Client::new(), &account);
        assert_eq!(
            (c.base.as_str(), c.key.as_str()),
            ("http://localhost:4101", "pk_42_k1")
        );
        let c = Clickup::new(reqwest::Client::new(), &Credentials::default());
        assert_eq!(c.base, API);
        assert_eq!(ClickupSource.account_name(""), "clickup");
    }
}
