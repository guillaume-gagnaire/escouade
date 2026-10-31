//! Azure Boards (REST API 7.1), on dev.azure.com and Azure DevOps Server. It has no account of its
//! own: the Azure DevOps code host's (`forge::accounts`, an organization's, its token personal or
//! `az`'s) serves both uses, through the code host's client (`forge::azure_api::Azure`). A
//! container is a project, `<org>/<project>`, so that the organization goes with it. A work
//! item's states are those of its type, by its project's process (Basic, Agile, Scrum, CMMI or
//! one inherited from them): the Kanban's are those of every type, each with its category.

use super::text::{criteria_in, criteria_of, html_to_markdown};
use super::{
    encode, Container, ExternalIssue, HttpError, IssueFilter, IssuePage, Query, StatesView,
};
use crate::forge::azure_api::Azure;
use crate::i18n::{self, Lang};
use crate::model::{Column, ExternalRef, ExternalState, Service};
use anyhow::{anyhow, Result};
use reqwest::Method;
use serde_json::{json, Value};
use std::collections::BTreeMap;

const S: Service = Service::AZURE;

const VERSION: &str = "api-version=7.1";
/// The tags' and the comments' APIs are still previews in 7.1.
const TAGS_VERSION: &str = "api-version=7.1-preview.1";
const COMMENTS_VERSION: &str = "api-version=7.1-preview.4";

/// The import's page.
const PAGE: usize = 50;
/// The work items a query gives at most: the import pages through them.
const MAX_QUERY: usize = 1000;

const MINE: &str = "mine";
const TYPE: &str = "type:";
const TAG: &str = "tag:";

/// The types a project has that are no backlog's (tests, reviews, feedback): never imported.
const HIDDEN_TYPES: &[&str] = &[
    "Code Review Request",
    "Code Review Response",
    "Feedback Request",
    "Feedback Response",
    "Shared Steps",
    "Shared Parameter",
    "Test Case",
    "Test Plan",
    "Test Suite",
];

/// The fields read.
const DESCRIPTION: &str = "System.Description";
const REPRO_STEPS: &str = "Microsoft.VSTS.TCM.ReproSteps";
const ACCEPTANCE: &str = "Microsoft.VSTS.Common.AcceptanceCriteria";

/// An organization (a Server's collection) the code host's account reaches.
pub struct Instance {
    /// Its name, as a container starts with it.
    pub org: String,
    /// The code host's client, on the account's token.
    pub client: Azure,
    /// Where the account's token is made again: its tokens' page.
    pub token_page: String,
}

/// A state of a type, and its category (`Proposed`, `InProgress`, `Resolved`, `Completed`,
/// `Removed`).
#[derive(Debug, Clone, PartialEq)]
struct TypeState {
    name: String,
    category: String,
}

/// A work item type of the project, and its states in their order.
#[derive(Debug, Clone, PartialEq)]
struct WorkType {
    name: String,
    states: Vec<TypeState>,
}

/// The categories in the order the Kanban lists their states.
const CATEGORIES: &[&str] = &["Proposed", "InProgress", "Resolved", "Completed"];

fn closed(category: &str) -> bool {
    matches!(category, "Completed" | "Removed")
}

pub struct Boards {
    instances: Vec<Instance>,
}

/// A text in a query, between its quotes.
fn quoted(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// `url` with the API's version.
fn v(url: String, version: &str) -> String {
    let sep = if url.contains('?') { '&' } else { '?' };
    format!("{url}{sep}{version}")
}

impl Boards {
    pub fn new(instances: Vec<Instance>) -> Self {
        Self { instances }
    }

    /// The organization of a container, and its project.
    fn of<'a>(&self, container: &'a str) -> Result<(&Instance, &'a str)> {
        let (org, project) = container.split_once('/').unwrap_or((container, ""));
        let inst = self
            .instances
            .iter()
            .find(|i| i.org.eq_ignore_ascii_case(org))
            .ok_or_else(|| {
                anyhow!(tr!(
                    "Aucun compte Azure DevOps connecté pour l'organisation {org}",
                    "No Azure DevOps account connected for the {org} organization"
                ))
            })?;
        Ok((inst, project))
    }

    fn project_api(inst: &Instance, project: &str) -> String {
        format!(
            "{}/{}/_apis",
            inst.client.org_url(&inst.org),
            encode(project)
        )
    }

    /// A call on the work items: a token without their right is told so, and where to make
    /// another one.
    async fn call(
        &self,
        inst: &Instance,
        method: Method,
        url: String,
        body: Option<(&Value, &str)>,
    ) -> Result<(Value, reqwest::header::HeaderMap)> {
        inst.client.call(method, &url, body).await.map_err(|e| {
            match e.downcast_ref::<HttpError>() {
                Some(h) if matches!(h.status, 401 | 403) => HttpError {
                    status: h.status,
                    message: no_right(i18n::ui(), &h.message, &inst.token_page),
                }
                .into(),
                _ => e,
            }
        })
    }

    async fn get(&self, inst: &Instance, url: String) -> Result<Value> {
        Ok(self.call(inst, Method::GET, url, None).await?.0)
    }

    /// The account's name, once its token is accepted.
    pub async fn me(&self) -> Result<(String, String)> {
        let inst = self.instances.first().ok_or_else(|| {
            anyhow!(tr!(
                "Aucun compte Azure DevOps connecté",
                "No Azure DevOps account connected"
            ))
        })?;
        let data = self
            .get(
                inst,
                format!("{}/_apis/connectionData", inst.client.org_url(&inst.org)),
            )
            .await?;
        let user = &data["authenticatedUser"];
        let name = user["providerDisplayName"]
            .as_str()
            .or_else(|| user["customDisplayName"].as_str())
            .unwrap_or("?")
            .to_string();
        let id = user["id"].as_str().unwrap_or_default().to_string();
        Ok((name, id))
    }

    /// The projects of each organization, by name, the next ones asked as long as Azure says
    /// there are more (`x-ms-continuationtoken`).
    pub async fn containers(&self) -> Result<Vec<Container>> {
        let mut out = Vec::new();
        for inst in &self.instances {
            let mut token: Option<String> = None;
            // A hundred thousand projects at most: Azure would not go on forever.
            for _ in 0..200 {
                let mut url = format!(
                    "{}/_apis/projects?stateFilter=wellFormed&$top=500",
                    inst.client.org_url(&inst.org)
                );
                if let Some(t) = &token {
                    url.push_str(&format!("&continuationToken={}", encode(t)));
                }
                let (v, headers) = self
                    .call(inst, Method::GET, self::v(url, VERSION), None)
                    .await?;
                out.extend(
                    v["value"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|p| p["name"].as_str())
                        .map(|name| Container {
                            id: format!("{}/{name}", inst.org),
                            name: name.to_string(),
                            group: Some(inst.org.clone()),
                            own: false,
                        }),
                );
                token = headers
                    .get("x-ms-continuationtoken")
                    .and_then(|h| h.to_str().ok())
                    .map(str::trim)
                    .filter(|t| !t.is_empty())
                    .map(str::to_string);
                if token.is_none() {
                    break;
                }
            }
        }
        Ok(out)
    }

    /// The project's work item types that a backlog has (not its tests', reviews' or feedback's),
    /// their states as its process gives them.
    async fn types(&self, inst: &Instance, project: &str) -> Result<Vec<WorkType>> {
        let v = self
            .get(
                inst,
                v(
                    format!("{}/wit/workitemtypes", Self::project_api(inst, project)),
                    VERSION,
                ),
            )
            .await?;
        Ok(v["value"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|t| !t["isDisabled"].as_bool().unwrap_or(false))
            .filter_map(|t| {
                let name = t["name"].as_str()?;
                if HIDDEN_TYPES.contains(&name) {
                    return None;
                }
                let states: Vec<TypeState> = t["states"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|s| {
                        Some(TypeState {
                            name: s["name"].as_str()?.to_string(),
                            category: s["category"].as_str().unwrap_or("").to_string(),
                        })
                    })
                    .collect();
                (!states.is_empty()).then(|| WorkType {
                    name: name.to_string(),
                    states,
                })
            })
            .collect())
    }

    /// The states of every type, by category (proposed, in progress, resolved, completed), each
    /// once; those removed are left out.
    async fn type_states(&self, container: &str) -> Result<Vec<TypeState>> {
        let (inst, project) = self.of(container)?;
        Ok(all_states(&self.types(inst, project).await?))
    }

    pub async fn states(&self, container: &str) -> Result<Vec<ExternalState>> {
        Ok(self
            .type_states(container)
            .await?
            .into_iter()
            .map(|s| ExternalState {
                id: s.name.clone(),
                name: s.name,
            })
            .collect())
    }

    /// The states, and those each column gives by default: by their category (`defaults_of`).
    pub async fn states_view(&self, container: &str) -> Result<StatesView> {
        let states = self.type_states(container).await?;
        Ok(StatesView {
            defaults: defaults_of(&states),
            states: states
                .into_iter()
                .map(|s| ExternalState {
                    id: s.name.clone(),
                    name: s.name,
                })
                .collect(),
        })
    }

    /// The project's open work items, the latest changed first: a WIQL query gives their ids
    /// (the backlog's types, not closed, and what the chips and the text ask), then a page of
    /// them is read (50 a page, the query's offset in `Query::page`). « #12 » or « 12 » gives the
    /// work item itself, a text the ones whose title holds it.
    pub async fn issues(&self, container: &str, q: &Query) -> Result<IssuePage> {
        let (inst, project) = self.of(container)?;
        let types = self.types(inst, project).await?;
        let lang = i18n::ui();
        let wiql = query_of(&types, q);
        let (found, _) = self
            .call(
                inst,
                Method::POST,
                v(
                    format!(
                        "{}/wit/wiql?$top={MAX_QUERY}",
                        Self::project_api(inst, project)
                    ),
                    VERSION,
                ),
                Some((&json!({ "query": wiql }), "application/json")),
            )
            .await?;
        let ids: Vec<u64> = found["workItems"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|w| w["id"].as_u64())
            .collect();
        let offset: usize = q.page.as_deref().and_then(|p| p.parse().ok()).unwrap_or(0);
        let page: Vec<u64> = ids.iter().skip(offset).take(PAGE).copied().collect();
        let mut items: Vec<Value> = Vec::new();
        if !page.is_empty() {
            let list = page
                .iter()
                .map(u64::to_string)
                .collect::<Vec<_>>()
                .join(",");
            let v = self
                .get(
                    inst,
                    v(
                        format!(
                            "{}/wit/workitems?ids={list}&errorPolicy=omit",
                            Self::project_api(inst, project)
                        ),
                        VERSION,
                    ),
                )
                .await?;
            items = v["value"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter(|w| !w.is_null())
                .collect();
            // In the query's order.
            items.sort_by_key(|w| {
                page.iter()
                    .position(|id| Some(*id) == w["id"].as_u64())
                    .unwrap_or(usize::MAX)
            });
        }
        let web = inst.client.org_url(&inst.org);
        let issues = items
            .iter()
            .map(|w| issue(lang, container, &web, project, w))
            .collect();
        let mut filters = vec![IssueFilter {
            id: MINE.into(),
            label: tr_in!(lang, "Assignés à moi", "Assigned to me"),
        }];
        filters.extend(types.iter().map(|t| IssueFilter {
            id: format!("{TYPE}{}", t.name),
            label: t.name.clone(),
        }));
        filters.extend(
            self.tags(inst, project)
                .await
                .into_iter()
                .take(12)
                .map(|t| IssueFilter {
                    id: format!("{TAG}{t}"),
                    label: t,
                }),
        );
        let next = offset + PAGE;
        Ok(IssuePage {
            issues,
            filters,
            next: (next < ids.len()).then(|| next.to_string()),
            total: Some(ids.len() as u64),
        })
    }

    /// The project's tags; none when Azure does not give them.
    async fn tags(&self, inst: &Instance, project: &str) -> Vec<String> {
        let Ok(v) = self
            .get(
                inst,
                v(
                    format!("{}/wit/tags", Self::project_api(inst, project)),
                    TAGS_VERSION,
                ),
            )
            .await
        else {
            return Vec::new();
        };
        v["value"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|t| t["name"].as_str().map(str::to_string))
            .collect()
    }

    fn item_api(inst: &Instance, project: &str, r: &ExternalRef) -> String {
        format!(
            "{}/wit/workitems/{}",
            Self::project_api(inst, project),
            encode(&r.id)
        )
    }

    /// Gives the work item `state`, in one JSON Patch of `System.State`. A state its type does
    /// not have (the Kanban's are those of every type) is its type's first one of the same
    /// category: « Active » of a User Story is « In Progress » for a Task of a Scrum project.
    pub async fn set_state(&self, r: &ExternalRef, state: &ExternalState) -> Result<()> {
        let (inst, project) = self.of(&r.container)?;
        let url = Self::item_api(inst, project, r);
        let item = self
            .get(
                inst,
                v(
                    format!("{url}?fields=System.State,System.WorkItemType"),
                    VERSION,
                ),
            )
            .await?;
        let now = item["fields"]["System.State"].as_str().unwrap_or("");
        let kind = item["fields"]["System.WorkItemType"].as_str().unwrap_or("");
        let types = self.types(inst, project).await?;
        let target = target_state(&types, kind, &state.id);
        if target == now {
            return Ok(());
        }
        let patch = json!([{ "op": "add", "path": "/fields/System.State", "value": target }]);
        self.call(
            inst,
            Method::PATCH,
            v(url, VERSION),
            Some((&patch, "application/json-patch+json")),
        )
        .await?;
        Ok(())
    }

    /// A comment on the work item, as markdown.
    pub async fn comment(&self, r: &ExternalRef, text: &str) -> Result<()> {
        let (inst, project) = self.of(&r.container)?;
        self.call(
            inst,
            Method::POST,
            v(
                format!(
                    "{}/comments?format=markdown",
                    Self::item_api(inst, project, r)
                ),
                COMMENTS_VERSION,
            ),
            Some((&json!({ "text": text }), "application/json")),
        )
        .await?;
        Ok(())
    }
}

/// What Azure said of a token it refused on the work items, and what to do: the right Work
/// Items, on a new token made on its page.
fn no_right(lang: Lang, said: &str, page: &str) -> String {
    tr_in!(
        lang,
        "{said}. Le jeton d'Azure DevOps doit avoir le droit Work Items (lecture et écriture) : crée-en un sur {page}, puis remplace-le dans Réglages › Intégrations › Hébergeurs de code.",
        "{said}. The Azure DevOps token needs the Work Items right (read and write): make one on {page}, then replace it in Settings › Integrations › Code hosts."
    )
}

/// The states of every type, by category, each once.
fn all_states(types: &[WorkType]) -> Vec<TypeState> {
    let mut out: Vec<TypeState> = Vec::new();
    for cat in CATEGORIES {
        for t in types {
            for s in t.states.iter().filter(|s| s.category == *cat) {
                if !out.iter().any(|o| o.name == s.name) {
                    out.push(s.clone());
                }
            }
        }
    }
    // A category Azure added since: after the others, unless removed.
    for t in types {
        for s in &t.states {
            if !CATEGORIES.contains(&s.category.as_str())
                && !closed(&s.category)
                && !out.iter().any(|o| o.name == s.name)
            {
                out.push(s.clone());
            }
        }
    }
    out
}

/// What each column gives by default: « En cours » and « À tester » an in-progress state (the
/// one named like it, else the first), « Terminé » a resolved one (« Resolved »), when the
/// process has one: the other columns leave the work item as it is until the ticket is done.
fn defaults_of(states: &[TypeState]) -> BTreeMap<Column, ExternalState> {
    let as_state = |s: &TypeState| ExternalState {
        id: s.name.clone(),
        name: s.name.clone(),
    };
    let mut out = BTreeMap::new();
    let doing: Vec<ExternalState> = states
        .iter()
        .filter(|s| s.category == "InProgress")
        .map(as_state)
        .collect();
    let by_name = super::text::default_states(&doing);
    if let Some(d) = by_name.get(&Column::Doing).or(doing.first()) {
        out.insert(Column::Doing, d.clone());
        out.insert(Column::Review, d.clone());
    }
    if let Some(r) = states.iter().find(|s| s.category == "Resolved") {
        out.insert(Column::Done, as_state(r));
    }
    out
}

/// The state a work item of type `kind` takes for `wanted`: itself when its type has it, else its
/// type's first state of the same category, else `wanted` (Azure says why it refuses it).
fn target_state(types: &[WorkType], kind: &str, wanted: &str) -> String {
    let Some(t) = types.iter().find(|t| t.name.eq_ignore_ascii_case(kind)) else {
        return wanted.to_string();
    };
    if t.states.iter().any(|s| s.name == wanted) {
        return wanted.to_string();
    }
    let category = types
        .iter()
        .flat_map(|t| t.states.iter())
        .find(|s| s.name == wanted)
        .map(|s| s.category.as_str());
    category
        .and_then(|c| t.states.iter().find(|s| s.category == c))
        .map_or_else(|| wanted.to_string(), |s| s.name.clone())
}

/// The WIQL query of the import: the project's work items of the backlog's types (or those of the
/// types' chips), not closed, assigned to me, carrying one of the chosen tags (or the automatic
/// import's), whose title holds the text or whose id it is; the latest changed first.
fn query_of(types: &[WorkType], q: &Query) -> String {
    let mut wh = vec!["[System.TeamProject] = @project".to_string()];
    let chosen: Vec<&str> = q
        .filters
        .iter()
        .filter_map(|f| f.strip_prefix(TYPE))
        .collect();
    let kinds: Vec<String> = if chosen.is_empty() {
        types.iter().map(|t| quoted(&t.name)).collect()
    } else {
        chosen.iter().map(|t| quoted(t)).collect()
    };
    if !kinds.is_empty() {
        wh.push(format!("[System.WorkItemType] IN ({})", kinds.join(", ")));
    }
    let mut done: Vec<String> = Vec::new();
    for t in types {
        for s in t.states.iter().filter(|s| closed(&s.category)) {
            let s = quoted(&s.name);
            if !done.contains(&s) {
                done.push(s);
            }
        }
    }
    if done.is_empty() {
        done = ["Closed", "Done", "Removed"].map(quoted).to_vec();
    }
    wh.push(format!("[System.State] NOT IN ({})", done.join(", ")));
    if q.filters.iter().any(|f| f == MINE) {
        wh.push("[System.AssignedTo] = @Me".into());
    }
    let mut tags: Vec<String> = q
        .filters
        .iter()
        .filter_map(|f| f.strip_prefix(TAG))
        .map(|t| format!("[System.Tags] CONTAINS {}", quoted(t)))
        .collect();
    if let Some(l) = q.label.as_deref().map(str::trim).filter(|l| !l.is_empty()) {
        tags.push(format!("[System.Tags] CONTAINS {}", quoted(l)));
    }
    if !tags.is_empty() {
        wh.push(format!("({})", tags.join(" OR ")));
    }
    let text = q.text.trim();
    let number = text.trim_start_matches('#');
    if !number.is_empty() && number.chars().all(|c| c.is_ascii_digit()) {
        wh.push(format!("[System.Id] = {number}"));
    } else if !text.is_empty() {
        wh.push(format!("[System.Title] CONTAINS {}", quoted(text)));
    }
    format!(
        "SELECT [System.Id] FROM WorkItems WHERE {} ORDER BY [System.ChangedDate] DESC",
        wh.join(" AND ")
    )
}

/// What the window colours a type by.
fn kind_code(kind: &str) -> &'static str {
    match kind {
        "Bug" => "bug",
        "Task" => "task",
        "User Story" | "Product Backlog Item" | "Requirement" => "story",
        "Epic" | "Feature" => "epic",
        "Issue" | "Impediment" => "issue",
        _ => "",
    }
}

/// Someone a field names (`System.AssignedTo`, `System.CreatedBy`): an identity, or its old text.
fn person(v: &Value) -> Option<String> {
    v["displayName"]
        .as_str()
        .or_else(|| v.as_str())
        .map(|s| s.split(" <").next().unwrap_or(s).trim().to_string())
        .filter(|s| !s.is_empty())
}

fn issue(lang: Lang, container: &str, web: &str, project: &str, w: &Value) -> ExternalIssue {
    let f = &w["fields"];
    let id = w["id"].as_u64().unwrap_or_default();
    let kind = f["System.WorkItemType"].as_str().unwrap_or("").to_string();
    let html = |name: &str| html_to_markdown(f[name].as_str().unwrap_or(""));
    let mut description = html(DESCRIPTION);
    if description.is_empty() {
        // A bug of the Agile, Scrum and CMMI processes is told by its steps.
        description = html(REPRO_STEPS);
    }
    let acceptance = html(ACCEPTANCE);
    let criteria = if acceptance.is_empty() {
        criteria_of(&description)
    } else {
        criteria_in(&acceptance)
    };
    let mut meta: Vec<String> = Vec::new();
    if let Some(s) = f["System.State"].as_str() {
        meta.push(s.to_string());
    }
    meta.extend(
        f["System.Tags"]
            .as_str()
            .unwrap_or("")
            .split(';')
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(str::to_string),
    );
    meta.push(
        person(&f["System.AssignedTo"])
            .unwrap_or_else(|| tr_in!(lang, "Non assigné", "Unassigned")),
    );
    if let Some(a) = person(&f["System.CreatedBy"]) {
        meta.push(tr_in!(lang, "par {a}", "by {a}"));
    }
    ExternalIssue {
        service: S,
        id: id.to_string(),
        key: format!("#{id}"),
        title: f["System.Title"].as_str().unwrap_or_default().to_string(),
        kind_code: kind_code(&kind).into(),
        kind,
        meta,
        url: format!("{web}/{}/_workitems/edit/{id}", encode(project)),
        criteria,
        description,
        container: container.to_string(),
        imported: false,
    }
}

#[cfg(test)]
#[path = "azure_tests.rs"]
mod tests;
