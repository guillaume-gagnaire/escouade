//! The deployments (ESC-67): after the merge, Escouade follows the environments of the project's
//! code host (GitHub's deployments, GitLab's environments, Bitbucket Pipelines' deployments; Gitea
//! has none). A done ticket is in an environment once a deployment that succeeded there holds the
//! commit of its merge (`git merge-base --is-ancestor`, after a fetch when that commit is not here
//! yet): its card says it, and the view « Environnements » says what runs where, the tickets
//! delivered in each and those that wait for production. A deployment that fails is notified once,
//! with its job's link. The pull request of a ticket « En revue » gets its preview's address
//! (Vercel, Netlify, Cloudflare Pages…). Read by the same loop as the tickets' pull requests
//! (`ticketprs`), without a webhook; nothing is ever published on the host.

use crate::agent::NotifyKind;
use crate::core::Core;
use crate::forge::{Check, Deployment};
use crate::git;
use crate::i18n::{self, Lang};
use crate::model::{now_ms, Column, Ticket, TicketEnv, UiEvent};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

/// How long after it is done a ticket is still followed in its environments (ms): 30 days.
pub const FOLLOWED_FOR: i64 = 30 * 24 * 3600 * 1000;

/// How long the view « Environnements » keeps its project followed once open (ms): 15 minutes.
pub const VIEWED_FOR: i64 = 15 * 60 * 1000;

/// An environment of the project's host, as the view « Environnements » shows it.
#[derive(Debug, Clone, PartialEq, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Environment {
    pub name: String,
    /// Its address: the newest one a deployment there gave.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Production (`production`, `prod`, `prod-eu`…): what the tickets wait for.
    pub production: bool,
    /// What runs there: its newest deployment that succeeded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub running: Option<Deployment>,
    /// Its newest deployment, when it is not the one that runs (under way, or failed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest: Option<Deployment>,
    /// Its newest deployment failed: the view draws a mark, with its job's link.
    pub failed: bool,
}

/// The environments of a project, as last read.
#[derive(Debug, Clone, PartialEq, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProjectEnvironments {
    pub project_id: String,
    pub environments: Vec<Environment>,
    /// When they were last read well (ms).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_at: Option<i64>,
    /// Why the last reading failed (no remote, no account, the host out of reach).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// The host has no deployments (Gitea, Forgejo, Azure DevOps): the view says so.
    pub unsupported: bool,
    /// The host's name (« GitHub »).
    pub host: String,
}

/// A pull request's preview or a review app, not one of the repository's environments: Vercel's
/// « Preview », GitLab's `review/<branch>`, Netlify's `deploy-preview-12`, `pr-12`.
pub fn is_preview(name: &str) -> bool {
    let n = name.to_lowercase();
    n.contains("preview")
        || n.starts_with("review/")
        || n.starts_with("review-")
        || (n.starts_with("pr-") && n[3..].chars().all(|c| c.is_ascii_digit()))
}

/// Production, by its name: `production`, `prod`, `prod-eu`, `Production – web`.
pub fn is_production(name: &str) -> bool {
    let n = name.to_lowercase();
    n == "prod" || n.starts_with("prod-") || n.starts_with("prod_") || n.contains("production")
}

fn succeeded(d: &Deployment) -> bool {
    d.state == "success"
}

fn failed(d: &Deployment) -> bool {
    matches!(d.state.as_str(), "failure" | "error")
}

/// The environments that `environments` gave (newest first, of each one down to its newest
/// success), previews left out; production last, the others by their name.
pub fn summarize(deployments: &[Deployment]) -> Vec<Environment> {
    let mut by: BTreeMap<String, Environment> = BTreeMap::new();
    for d in deployments.iter().filter(|d| !is_preview(&d.environment)) {
        let e = by
            .entry(d.environment.clone())
            .or_insert_with(|| Environment {
                name: d.environment.clone(),
                production: is_production(&d.environment),
                ..Default::default()
            });
        if e.url.is_none() {
            e.url = d.url.clone().filter(|u| !u.is_empty());
        }
        if e.running.is_some() {
            continue;
        }
        if succeeded(d) {
            e.running = Some(d.clone());
        } else if e.latest.is_none() {
            e.failed = failed(d);
            e.latest = Some(d.clone());
        }
    }
    let mut out: Vec<Environment> = by.into_values().collect();
    out.sort_by_key(|e| e.production);
    out
}

/// The environments a done ticket is in: `contains(sha)` says whether the commit `sha` holds the
/// commit of its merge (None: not known, as not deployed).
pub fn ticket_environments(
    environments: &[Environment],
    contains: impl Fn(&str) -> Option<bool>,
) -> Vec<TicketEnv> {
    environments
        .iter()
        .map(|e| TicketEnv {
            name: e.name.clone(),
            url: e.url.clone(),
            deployed: e
                .running
                .as_ref()
                .filter(|d| !d.sha.is_empty())
                .and_then(|d| contains(&d.sha))
                .unwrap_or(false),
        })
        .collect()
}

/// The environments whose newest deployment failed and was not known to have failed: each is
/// notified once.
pub fn new_failures<'a>(before: &[Environment], after: &'a [Environment]) -> Vec<&'a Environment> {
    after
        .iter()
        .filter(|e| e.failed)
        .filter(|e| {
            !before
                .iter()
                .any(|b| b.name == e.name && b.failed && b.latest == e.latest)
        })
        .collect()
}

/// The productions where a new deployment succeeded since `before` (an environment that was not
/// there counts): each is notified once. A success older than what was known is not news: a
/// reading of the host goes back as far as it can.
pub fn new_productions<'a>(
    before: &[Environment],
    after: &'a [Environment],
) -> Vec<&'a Environment> {
    after
        .iter()
        .filter(|e| e.production)
        .filter(|e| {
            let Some(new) = &e.running else { return false };
            let Some(was) = before.iter().find(|b| b.name == e.name) else {
                return true;
            };
            match (&was.running, &was.latest) {
                (Some(old), _) => old.sha != new.sha && new.created >= old.created,
                (None, Some(old)) => new.created >= old.created,
                (None, None) => true,
            }
        })
        .collect()
}

/// What the notification of a deployment in production says.
pub fn success_text(lang: Lang, e: &Environment) -> String {
    let name = &e.name;
    let sha: String = e
        .running
        .as_ref()
        .map(|d| d.sha.chars().take(7).collect())
        .unwrap_or_default();
    match sha.is_empty() {
        true => tr_in!(lang, "Déployé : {name}", "Deployed: {name}"),
        false => tr_in!(lang, "Déployé : {name} ({sha})", "Deployed: {name} ({sha})"),
    }
}

/// What the notification of a failed deployment says.
pub fn failure_text(lang: Lang, e: &Environment) -> String {
    let name = &e.name;
    let sha: String = e
        .latest
        .as_ref()
        .map(|d| d.sha.chars().take(7).collect())
        .unwrap_or_default();
    match sha.is_empty() {
        true => tr_in!(
            lang,
            "Déploiement en échec : {name}",
            "Deployment failed: {name}"
        ),
        false => tr_in!(
            lang,
            "Déploiement en échec : {name} ({sha})",
            "Deployment failed: {name} ({sha})"
        ),
    }
}

/// A check that hosts a preview, by its name: Vercel, Netlify, Cloudflare Pages, Render, Surge…
fn preview_check(name: &str) -> bool {
    let n = name.to_lowercase();
    [
        "vercel",
        "netlify",
        "cloudflare",
        "pages",
        "preview",
        "render",
        "surge",
        "amplify",
    ]
    .iter()
    .any(|k| n.contains(k))
}

/// The preview of a pull request: a deployment of its branch that succeeded with an address (a
/// preview's first, production's last), else a check of a preview's host with one.
pub fn preview_of(deployments: &[Deployment], checks: &[Check]) -> Option<String> {
    let mut ok: Vec<&Deployment> = deployments
        .iter()
        .filter(|d| succeeded(d) && d.url.as_deref().is_some_and(|u| !u.is_empty()))
        .collect();
    ok.sort_by_key(|d| (!is_preview(&d.environment), is_production(&d.environment)));
    ok.first().and_then(|d| d.url.clone()).or_else(|| {
        checks
            .iter()
            .filter(|c| c.state == crate::forge::CheckState::Success && preview_check(&c.name))
            .find_map(|c| c.url.clone().filter(|u| u.starts_with("http")))
    })
}

/// The done tickets followed in their environments: merged (the commit of their merge is known),
/// done for less than `FOLLOWED_FOR`.
pub fn followed(t: &Ticket, now: i64) -> bool {
    t.column == Column::Done
        && t.outcome_commit.as_deref().is_some_and(|c| !c.is_empty())
        && t.done_at.is_some_and(|d| now - d < FOLLOWED_FOR)
}

/// Whether the commit `commit` is here (fetched).
async fn known(repo: &str, commit: &str) -> bool {
    git::run(
        repo,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{commit}^{{commit}}"),
        ],
    )
    .await
    .is_ok()
}

impl<R: tauri::Runtime> Core<R> {
    /// The environments of the projects followed (a ticket done and merged lately, or their view
    /// open), read with the tickets' pull requests (`follow_ticket_prs`).
    pub async fn deploys_refresh(self: &Arc<Self>, project: Option<&str>) {
        let now = now_ms();
        let mut ids: Vec<String> = self
            .tickets
            .read()
            .iter()
            .filter(|t| followed(t, now))
            .map(|t| t.project_id.clone())
            .collect();
        ids.extend(
            self.deploys_viewed
                .lock()
                .iter()
                .filter(|(_, at)| now - **at < VIEWED_FOR)
                .map(|(id, _)| id.clone()),
        );
        ids.sort();
        ids.dedup();
        for id in ids
            .iter()
            .filter(|id| project.is_none_or(|p| p == id.as_str()))
        {
            self.project_environments_read(id).await;
        }
    }

    /// The environments of a project, as last read (the view « Environnements » opens on them),
    /// and the project followed while the view is open.
    pub fn project_environments(&self, project_id: &str) -> Option<ProjectEnvironments> {
        self.deploys_viewed
            .lock()
            .insert(project_id.to_string(), now_ms());
        self.deploys.lock().get(project_id).cloned()
    }

    /// Reads the environments of a project now, places its done tickets in them and tells the
    /// window.
    pub async fn project_environments_read(
        self: &Arc<Self>,
        project_id: &str,
    ) -> ProjectEnvironments {
        let before = self.deploys.lock().get(project_id).cloned();
        let mut out = ProjectEnvironments {
            project_id: project_id.to_string(),
            ..before.clone().unwrap_or_default()
        };
        out.project_id = project_id.to_string();
        match self.project_host(project_id).await {
            Err(e) => {
                out.error = Some(format!("{e:#}"));
            }
            Ok((remote, provider, client)) => {
                out.host = provider.name().to_string();
                if !provider.capabilities().deployments {
                    out.unsupported = true;
                    out.environments.clear();
                    out.error = None;
                } else {
                    out.unsupported = false;
                    match client.environments(&remote.repo).await {
                        Err(e) => {
                            out.error = Some(crate::ticketprs::error_text(
                                i18n::ui(),
                                provider.name(),
                                &e,
                            ))
                        }
                        Ok(list) => {
                            out.environments = summarize(&list);
                            out.read_at = Some(now_ms());
                            out.error = None;
                        }
                    }
                }
            }
        }
        if out.error.is_none() {
            self.place_tickets(project_id, &out.environments).await;
            let old = before.map(|b| b.environments).unwrap_or_default();
            // The first reading of a project tells nothing: what failed before is not news.
            if out.read_at.is_some() && !old.is_empty() {
                let name = self.project(project_id).map(|p| p.name).unwrap_or_default();
                let failures = new_failures(&old, &out.environments)
                    .into_iter()
                    .map(|e| (NotifyKind::DeployFailed, failure_text(i18n::ui(), e)));
                let successes = new_productions(&old, &out.environments)
                    .into_iter()
                    .map(|e| (NotifyKind::DeployOk, success_text(i18n::ui(), e)));
                for (kind, text) in failures.chain(successes) {
                    self.alert(
                        kind,
                        name.clone(),
                        text,
                        UiEvent::FocusBoard {
                            project_id: project_id.to_string(),
                        },
                    );
                }
            }
        }
        self.deploys
            .lock()
            .insert(project_id.to_string(), out.clone());
        self.hub.emit(UiEvent::Environments {
            environments: out.clone(),
        });
        out
    }

    /// The done tickets of a project placed in its environments: the commits that run there are
    /// fetched when they are not here yet, and each ticket's merge is looked for in them.
    async fn place_tickets(&self, project_id: &str, environments: &[Environment]) {
        let Ok(project) = self.project(project_id) else {
            return;
        };
        let repo = project.path.clone();
        let now = now_ms();
        let tickets: Vec<(String, String)> = self
            .tickets
            .read()
            .iter()
            .filter(|t| t.project_id == project_id && followed(t, now))
            .filter_map(|t| Some((t.id.clone(), t.outcome_commit.clone()?)))
            .collect();
        if tickets.is_empty() {
            return;
        }
        let shas: Vec<String> = environments
            .iter()
            .filter_map(|e| e.running.as_ref().map(|d| d.sha.clone()))
            .filter(|s| !s.is_empty())
            .collect();
        let mut here: HashMap<String, bool> = HashMap::new();
        for s in &shas {
            here.insert(s.clone(), known(&repo, s).await);
        }
        // Fetched once for a commit not here: one still missing after it is not fetched for again.
        let worth = here
            .iter()
            .any(|(s, k)| !k && !self.deploys_missing.lock().contains(s));
        if worth && !git::remotes(&repo).await.is_empty() {
            match git::fetch(&repo, true).await {
                Ok(()) => {
                    for s in &shas {
                        let k = known(&repo, s).await;
                        if !k {
                            self.deploys_missing.lock().insert(s.clone());
                        }
                        here.insert(s.clone(), k);
                    }
                }
                Err(e) => log::info!("environments of {project_id}: fetch failed: {e:#}"),
            }
        }
        for (id, commit) in tickets {
            let mut contains: HashMap<String, Option<bool>> = HashMap::new();
            for s in &shas {
                let key = (commit.clone(), s.clone());
                let cached = self.deploys_contains.lock().get(&key).copied();
                let v = match cached {
                    Some(v) => Some(v),
                    None if here.get(s) == Some(&true) && known(&repo, &commit).await => {
                        let v = git::run(&repo, &["merge-base", "--is-ancestor", &commit, s])
                            .await
                            .is_ok();
                        self.deploys_contains.lock().insert(key, v);
                        Some(v)
                    }
                    None => None,
                };
                contains.insert(s.clone(), v);
            }
            let envs = ticket_environments(environments, |s| contains.get(s).copied().flatten());
            let same = self
                .tickets
                .read()
                .iter()
                .any(|t| t.id == id && t.environments == envs);
            if !same {
                let _ = self.edit_ticket(&id, |t| {
                    t.environments = envs;
                    Ok(())
                });
            }
        }
    }
}

#[cfg(test)]
#[path = "deploys_tests.rs"]
mod tests;
