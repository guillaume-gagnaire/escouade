//! The data of the Kanban's sources (docs/PLUGINS.md §4.5, `tickets/*`) as a plugin writes it: containers, states,
//! tickets and their pages. The core reads them into its own types, which name the source by an id it interns
//! (`Service`); here it is the string the data holds (`github`, `gitlab`…), the rest is the same JSON.

use serde::{Deserialize, Serialize};

/// Where an account's tickets live: a repository, a project, a board.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Container {
    pub id: String,
    pub name: String,
    /// Where it is in its service's hierarchy, as the window groups it; none for most.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// The project's own repository (its remote's), which the import offers to link at once.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub own: bool,
}

/// A state of an external ticket: for GitHub "open", "closed" or `label:<name>`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ExternalState {
    pub id: String,
    pub name: String,
}

/// A filter of the import modal, shown as a chip.
#[derive(Debug, Clone, Serialize, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueFilter {
    pub id: String,
    pub label: String,
}

/// What the import asks of a source.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Query {
    /// Typed in the search field.
    pub text: String,
    /// The ids of the chips turned on (`IssueFilter::id`).
    pub filters: Vec<String>,
    /// Only the tickets that carry this label (automatic import).
    pub label: Option<String>,
    /// The page after the first one: `IssuePage::next` of the one before.
    pub page: Option<String>,
}

/// An external ticket as the import modal lists it, and as it is imported.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ExternalIssue {
    /// The source's id.
    pub service: String,
    /// What its API knows it by (`ExternalRef::id`).
    pub id: String,
    /// As shown: #142.
    pub key: String,
    pub title: String,
    /// As shown, in the interface's language.
    pub kind: String,
    /// What the window colours it by, the same in every language: `issue`, `bug`, `task`…
    pub kind_code: String,
    /// Its priority, assignee and state, those it has.
    pub meta: Vec<String>,
    pub url: String,
    /// As markdown.
    pub description: String,
    /// Those its description or checklists list.
    pub criteria: Vec<String>,
    pub container: String,
    /// Already on the project's Kanban (the core says it, not the plugin).
    pub imported: bool,
}

/// A page of a source's tickets for the import modal, and the chips that filter them.
#[derive(Debug, Clone, Serialize, PartialEq, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssuePage {
    pub issues: Vec<ExternalIssue>,
    pub filters: Vec<IssueFilter>,
    /// What asks for the next page (`Query::page`), when the service has more.
    pub next: Option<String>,
    /// How many tickets match in all, when the service says so.
    pub total: Option<u64>,
}

/// Where an imported ticket comes from.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ExternalRef {
    /// The source's id.
    pub service: String,
    /// What its API knows it by: the GitHub issue's number.
    pub id: String,
    /// As it is shown: #42.
    pub key: String,
    /// Its repository, project or board.
    pub container: String,
    pub url: String,
    /// Why its last sync failed, until one succeeds.
    pub error: Option<String>,
}
