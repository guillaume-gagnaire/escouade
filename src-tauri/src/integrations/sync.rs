//! The integrations on the core: the accounts saved apart, what the window asks of the services
//! (containers, states, tickets), the import (by hand or by label), and the sync of an imported
//! ticket's external one when it changes column or begins a loop.

use super::text::{column_comment, default_states, loop_comment};
use super::*;
use crate::core::Core;
use crate::paths::{self, DataDir};
use crate::tickets::TicketDraft;
use crate::{board, git};
use anyhow::Context;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tauri::Runtime;

/// What an imported ticket's external one hears of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Change {
    /// It came into this column.
    Column(Column),
    /// It began its next loop.
    Loop,
}

/// What changed of an imported ticket that its external one hears of, if anything.
pub(crate) fn change_of(before: &Ticket, after: &Ticket) -> Option<Change> {
    after.external.as_ref()?;
    if before.column != after.column {
        Some(Change::Column(after.column))
    } else if after.column == Column::Doing
        && before.iteration > 0
        && after.iteration > before.iteration
    {
        Some(Change::Loop)
    } else {
        None
    }
}

/// The accounts as saved; none when the file is missing or unreadable (logged, set aside).
pub(crate) fn load_accounts(data: &DataDir) -> Accounts {
    let path = data.integrations_file();
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Accounts::default();
    };
    serde_json::from_str(&text).unwrap_or_else(|e| {
        log::error!("invalid {}: {e}", path.display());
        let _ = std::fs::copy(&path, path.with_extension("broken.json"));
        Accounts::default()
    })
}

/// How long `gh auth token` may take.
const GH_TOKEN_LIMIT: Duration = Duration::from_secs(15);

/// The ticket of an external one, as the import makes it: its description says where it comes
/// from, its criteria are those found (when asked for), else the defaults.
pub(crate) fn draft_of(i: &ExternalIssue, max_loops: u32, extract: bool) -> TicketDraft {
    let mut description = i.description.trim().to_string();
    let from = format!("Ticket {} {} : {}", i.service.label(), i.key, i.url);
    if !description.is_empty() {
        description.push_str("\n\n");
    }
    description.push_str(&from);
    TicketDraft {
        title: i.title.clone(),
        description,
        criteria: if extract {
            i.criteria.clone()
        } else {
            Vec::new()
        },
        max_loops,
        after: Vec::new(),
    }
}

/// How a project remembers an imported ticket (`ProjectIntegrations::imported`).
fn imported_key(i: &ExternalIssue) -> String {
    format!("{}|{}", i.service.label().to_lowercase(), i.id)
}

/// The sending side of the sync's worker.
pub(crate) type SyncSender = tokio::sync::mpsc::UnboundedSender<(Ticket, Change)>;

fn external_of(i: &ExternalIssue) -> ExternalRef {
    ExternalRef {
        service: i.service,
        id: i.id.clone(),
        key: i.key.clone(),
        container: i.container.clone(),
        url: i.url.clone(),
        error: None,
    }
}

/// "3 tickets importés depuis Jira et Trello".
pub(crate) fn imported_label(tickets: &[Ticket]) -> String {
    let mut services: Vec<Service> = Vec::new();
    for t in tickets {
        if let Some(e) = &t.external {
            if !services.contains(&e.service) {
                services.push(e.service);
            }
        }
    }
    let names: Vec<&str> = services.iter().map(|s| s.label()).collect();
    let n = tickets.len();
    format!(
        "{n} {} depuis {}",
        if n > 1 {
            "tickets importés"
        } else {
            "ticket importé"
        },
        match names.as_slice() {
            [] => String::new(),
            [one] => one.to_string(),
            [rest @ .., last] => format!("{} et {last}", rest.join(", ")),
        }
    )
}

impl<R: Runtime> Core<R> {
    fn save_accounts(&self) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(&*self.accounts.read())?;
        paths::write_atomic(&self.data.integrations_file(), &bytes)
            .context("enregistrement des comptes")?;
        Ok(())
    }

    pub fn integration_accounts(&self) -> Vec<AccountView> {
        self.accounts.read().views()
    }

    /// A client of `service` for `account` (GitHub's token asked of its CLI when none was given).
    async fn client_for(&self, service: Service, account: &Account) -> Result<Client> {
        let http = crate::usage::http_client(&self.settings.read())?;
        let mut account = account.clone();
        if service == Service::Github && account.token.trim().is_empty() {
            account.token = self.gh_token().await?;
        }
        let bases = self.bases.read().clone();
        Ok(Client::new(service, &account, http, &bases))
    }

    async fn client(&self, service: Service) -> Result<Client> {
        let account = self
            .accounts
            .read()
            .get(service)
            .cloned()
            .ok_or_else(|| anyhow!("Aucun compte {} connecté", service.label()))?;
        self.client_for(service, &account).await
    }

    /// The GitHub CLI's token (`gh auth token`), asked once.
    async fn gh_token(&self) -> Result<String> {
        if let Some(t) = self.gh_token.lock().clone() {
            return Ok(t);
        }
        let none = || anyhow!("GitHub : donne un jeton, ou connecte la CLI gh (gh auth login)");
        let gh = self.gh_cli().ok_or_else(none)?;
        let mut cmd = tokio::process::Command::new(gh);
        cmd.args(["auth", "token"])
            .env("GH_PROMPT_DISABLED", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        #[cfg(windows)]
        cmd.creation_flags(crate::claude::CREATE_NO_WINDOW);
        let out = tokio::time::timeout(GH_TOKEN_LIMIT, cmd.output())
            .await
            .map_err(|_| none())??;
        let token = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !out.status.success() || token.is_empty() {
            return Err(none());
        }
        *self.gh_token.lock() = Some(token.clone());
        Ok(token)
    }

    /// "Connecter…": the credentials are checked with the service, then saved with what it said
    /// of them (label, user).
    pub async fn integration_connect(
        &self,
        service: Service,
        form: Account,
    ) -> Result<AccountView> {
        let mut account = Account {
            site: if service == Service::Jira {
                jira::site(&form.site)
            } else {
                String::new()
            },
            email: form.email.trim().to_string(),
            key: form.key.trim().to_string(),
            token: form.token.trim().to_string(),
            ..Default::default()
        };
        let missing = match service {
            Service::Jira if account.site.is_empty() => Some("l'adresse du site"),
            Service::Jira if account.email.is_empty() => Some("l'e-mail"),
            Service::Jira | Service::Trello if account.token.is_empty() => Some("le jeton"),
            Service::Trello if account.key.is_empty() => Some("la clé d'API"),
            _ => None,
        };
        if let Some(what) = missing {
            bail!("Indique {what}.");
        }
        if service == Service::Jira && !jira::secure(&account.site) {
            bail!("Le site Jira doit être en https (http seulement sur cette machine).");
        }
        if service == Service::Github && account.token.is_empty() {
            // The CLI may have been logged in again since its token was read.
            *self.gh_token.lock() = None;
        }
        let (mut label, user) = self.client_for(service, &account).await?.me().await?;
        if service == Service::Github && account.token.is_empty() {
            label.push_str(" · via gh");
        }
        account.label = label;
        account.user = user;
        self.accounts.write().set(service, Some(account));
        self.save_accounts()?;
        Ok(self
            .integration_accounts()
            .into_iter()
            .find(|a| a.service == service)
            .expect("every service has a view"))
    }

    /// "Déconnecter": the account is forgotten (the projects keep their links, unused until one is
    /// connected again).
    pub fn integration_disconnect(&self, service: Service) -> Result<Vec<AccountView>> {
        self.accounts.write().set(service, None);
        if service == Service::Github {
            *self.gh_token.lock() = None;
        }
        self.save_accounts()?;
        Ok(self.integration_accounts())
    }

    /// The account's containers; for GitHub, the project's own repository first.
    pub async fn integration_containers(
        &self,
        service: Service,
        project_id: Option<&str>,
    ) -> Result<Vec<Container>> {
        let mut list = self.client(service).await?.containers().await?;
        if service == Service::Github {
            if let Some(repo) = match project_id {
                Some(id) => self.project_repo(id).await,
                None => None,
            } {
                list.retain(|c| c.id != repo);
                list.insert(
                    0,
                    Container {
                        name: format!("{repo} (dépôt du projet)"),
                        id: repo,
                    },
                );
            }
        }
        Ok(list)
    }

    /// `owner/repo` of the project's GitHub remote (origin, else the only one), if it has one.
    async fn project_repo(&self, project_id: &str) -> Option<String> {
        let path = self.project(project_id).ok()?.path;
        let remotes = git::remotes(&path).await;
        let remote = git::sync_remote(&remotes, None)?;
        let url = git::remote_url(&path, remote).await?;
        board::github_repo(&url).map(|(o, r)| format!("{o}/{r}"))
    }

    pub async fn integration_states(
        &self,
        service: Service,
        container: &str,
    ) -> Result<StatesView> {
        let states = self.client(service).await?.states(container).await?;
        Ok(StatesView {
            defaults: default_states(&states),
            states,
        })
    }

    /// The tickets of the project's source for `service`, those already imported marked.
    pub async fn integration_issues(
        &self,
        project_id: &str,
        service: Service,
        q: Query,
    ) -> Result<IssuePage> {
        let project = self.project(project_id)?;
        let link = project
            .integrations
            .link(service)
            .ok_or_else(|| anyhow!("Aucune source {} liée à ce projet", service.label()))?;
        let mut page = self
            .client(service)
            .await?
            .issues(&link.container, &q)
            .await?;
        let have = self.imported_ids(project_id, service);
        for i in &mut page.issues {
            i.imported = have.contains(&i.id);
        }
        Ok(page)
    }

    /// The external ids of `service` the project's tickets come from.
    fn imported_ids(&self, project_id: &str, service: Service) -> Vec<String> {
        self.tickets
            .read()
            .iter()
            .filter(|t| t.project_id == project_id)
            .filter_map(|t| t.external.as_ref())
            .filter(|e| e.service == service)
            .map(|e| e.id.clone())
            .collect()
    }

    /// The external tickets come into "À faire", each once (one already there is skipped), and
    /// the project remembers them.
    pub async fn integration_import(
        self: &Arc<Self>,
        project_id: &str,
        issues: Vec<ExternalIssue>,
        max_loops: u32,
    ) -> Result<Vec<Ticket>> {
        self.import(project_id, issues, max_loops, false).await
    }

    /// `integration_import`; `new_only`: none ever imported into the project, even one deleted
    /// since (the automatic import).
    async fn import(
        self: &Arc<Self>,
        project_id: &str,
        issues: Vec<ExternalIssue>,
        max_loops: u32,
        new_only: bool,
    ) -> Result<Vec<Ticket>> {
        let _one = self.import_lock.lock().await;
        let extract = self.settings.read().integrations.extract_criteria;
        let seen = self.project(project_id)?.integrations.imported;
        let mut made = Vec::new();
        for i in issues {
            let key = imported_key(&i);
            if i.id.is_empty()
                || self.imported_ids(project_id, i.service).contains(&i.id)
                || (new_only && seen.contains(&key))
            {
                continue;
            }
            let draft = draft_of(&i, max_loops, extract);
            made.push(
                self.ticket_create_with(project_id, draft, Some(external_of(&i)))
                    .await?,
            );
            let project = {
                let mut projects = self.projects.write();
                projects.iter_mut().find(|p| p.id == project_id).map(|p| {
                    if !p.integrations.imported.contains(&key) {
                        p.integrations.imported.push(key);
                    }
                    p.clone()
                })
            };
            if let Some(project) = project {
                self.hub.emit(UiEvent::Project { project });
                self.request_save();
            }
        }
        Ok(made)
    }

    /// The automatic import's pass: in each project, the open tickets of its sources that carry
    /// the label, not yet imported.
    pub async fn auto_import(self: &Arc<Self>) {
        let s = self.settings.read().integrations.clone();
        let label = s.import_label.trim().to_string();
        if !s.auto_import || label.is_empty() {
            return;
        }
        let projects = self.projects.read().clone();
        for p in projects {
            let mut found = Vec::new();
            for link in &p.integrations.links {
                if self.accounts.read().get(link.service).is_none() {
                    continue;
                }
                let q = Query {
                    label: Some(label.clone()),
                    ..Default::default()
                };
                let page = match self.client(link.service).await {
                    Ok(c) => c.issues(&link.container, &q).await,
                    Err(e) => Err(e),
                };
                match page {
                    Ok(page) => found.extend(page.issues),
                    Err(e) => log::warn!(
                        "automatic import of {} from {}: {e:#}",
                        p.name,
                        link.service.label()
                    ),
                }
            }
            if found.is_empty() {
                continue;
            }
            match self.import(&p.id, found, 5, true).await {
                Ok(made) if !made.is_empty() => {
                    let text = format!("{} dans {}", imported_label(&made), p.name);
                    log::info!("automatic import: {text}");
                    self.hub.emit(UiEvent::Toast { text });
                }
                Ok(_) => {}
                Err(e) => log::warn!("automatic import into {}: {e:#}", p.name),
            }
        }
    }

    /// Tells the ticket's external one of `change`, in the background: one worker takes the
    /// changes one at a time, in the order they were made.
    pub(crate) fn sync_external(&self, ticket: Ticket, change: Change) {
        let mut queue = self.sync_queue.lock();
        if queue.as_ref().is_none_or(|q| q.is_closed()) {
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<(Ticket, Change)>();
            // The worker does not keep the core alive: it ends with it.
            let me = self.weak();
            tauri::async_runtime::spawn(async move {
                while let Some((ticket, change)) = rx.recv().await {
                    let Some(c) = me.upgrade() else { break };
                    c.sync_now(&ticket, change).await;
                    #[cfg(test)]
                    c.syncs_queued
                        .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                }
            });
            *queue = Some(tx);
        }
        #[cfg(test)]
        self.syncs_queued
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if let Some(q) = queue.as_ref() {
            let _ = q.send((ticket, change));
        }
    }

    async fn sync_now(self: &Arc<Self>, t: &Ticket, change: Change) {
        let Some(ext) = t.external.clone() else {
            return;
        };
        let Ok(project) = self.project(&t.project_id) else {
            return;
        };
        let s = self.settings.read().integrations.clone();
        let link = project
            .integrations
            .link(ext.service)
            .filter(|l| l.container == ext.container);
        let mut actions: Vec<(Option<ExternalState>, Option<String>)> = Vec::new();
        match change {
            Change::Column(column) => {
                let state = link
                    .filter(|_| s.sync_states)
                    .and_then(|l| l.states.get(&column))
                    .filter(|st| !st.id.is_empty())
                    .cloned();
                let comment = project
                    .integrations
                    .comments
                    .contains(&column)
                    .then(|| column_comment(t, column, Some(&self.branch_of_ticket(t))));
                actions.push((state, comment));
            }
            Change::Loop if s.loop_comments => actions.push((None, Some(loop_comment(t)))),
            Change::Loop => {}
        }
        if actions.iter().all(|(st, c)| st.is_none() && c.is_none()) {
            return;
        }
        let mapped: Vec<ExternalState> = link
            .map(|l| l.states.values().cloned().collect())
            .unwrap_or_default();
        let result = async {
            let client = self.client(ext.service).await?;
            for (state, comment) in &actions {
                if let Some(st) = state {
                    client.set_state(&ext, st, &mapped).await?;
                }
                if let Some(text) = comment {
                    client.comment(&ext, text).await?;
                }
            }
            anyhow::Ok(())
        }
        .await;
        let error = match result {
            Ok(()) => None,
            Err(e) => {
                log::warn!(
                    "sync of {} with {} {}: {e:#}",
                    t.key,
                    ext.service.label(),
                    ext.key
                );
                Some(board::first_line(&format!("{e:#}")))
            }
        };
        // Against the ticket as it is now: an earlier sync may have changed its error since the
        // change was queued (syncs run one at a time, so nothing else does meanwhile).
        let now = self
            .ticket(&t.id)
            .ok()
            .and_then(|x| x.external)
            .map(|e| e.error);
        if now.is_some_and(|now| now != error) {
            let _ = self.edit_ticket(&t.id, |x| {
                if let Some(e) = x.external.as_mut() {
                    e.error = error;
                }
                Ok(())
            });
        }
    }

    /// The branch of the ticket's agent's worktree; while it starts (no agent yet), the one it is
    /// given.
    fn branch_of_ticket(&self, t: &Ticket) -> String {
        let agent = self
            .ticket(&t.id)
            .ok()
            .and_then(|x| x.agent_id)
            .or(t.agent_id.clone());
        agent
            .and_then(|a| self.agent(&a).ok())
            .and_then(|h| h.lock().meta.worktree.as_ref().map(|w| w.branch.clone()))
            .unwrap_or_else(|| board::branch_of(&t.key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn imported(column: Column, iteration: u32) -> Ticket {
        Ticket {
            column,
            iteration,
            external: Some(ExternalRef::default()),
            ..Default::default()
        }
    }

    #[test]
    fn a_column_or_a_new_loop_of_an_imported_ticket_is_told() {
        let todo = imported(Column::Todo, 0);
        let doing = imported(Column::Doing, 1);
        assert_eq!(
            change_of(&todo, &doing),
            Some(Change::Column(Column::Doing))
        );
        assert_eq!(
            change_of(&doing, &imported(Column::Doing, 2)),
            Some(Change::Loop)
        );
        assert_eq!(change_of(&doing, &doing), None);
        // Sent back from loop 3 to loop 1, it comes from "À tester": a column.
        assert_eq!(
            change_of(&imported(Column::Review, 3), &doing),
            Some(Change::Column(Column::Doing))
        );
        let mine = Ticket {
            column: Column::Doing,
            ..Default::default()
        };
        assert_eq!(change_of(&Ticket::default(), &mine), None);
    }

    fn issue() -> ExternalIssue {
        ExternalIssue {
            service: Service::Jira,
            id: "ATL-7".into(),
            key: "ATL-7".into(),
            title: "Pagination".into(),
            url: "https://x.atlassian.net/browse/ATL-7".into(),
            description: "Paginer GET /users.\n".into(),
            criteria: vec!["Limite 100".into()],
            container: "ATL".into(),
            ..Default::default()
        }
    }

    #[test]
    fn an_imported_ticket_says_where_it_comes_from() {
        let d = draft_of(&issue(), 3, true);
        assert_eq!(d.title, "Pagination");
        assert_eq!(
            d.description,
            "Paginer GET /users.\n\nTicket Jira ATL-7 : https://x.atlassian.net/browse/ATL-7"
        );
        assert_eq!(
            (d.criteria.as_slice(), d.max_loops),
            (["Limite 100".to_string()].as_slice(), 3)
        );
        // Without extraction, the defaults (board::criteria_from gives them for none).
        assert!(draft_of(&issue(), 5, false).criteria.is_empty());
        let bare = ExternalIssue {
            description: " ".into(),
            ..issue()
        };
        assert_eq!(
            draft_of(&bare, 5, true).description,
            "Ticket Jira ATL-7 : https://x.atlassian.net/browse/ATL-7"
        );
    }

    #[test]
    fn the_import_says_how_many_and_from_where() {
        let t = |s: Service| Ticket {
            external: Some(ExternalRef {
                service: s,
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(
            imported_label(&[t(Service::Jira)]),
            "1 ticket importé depuis Jira"
        );
        assert_eq!(
            imported_label(&[t(Service::Jira), t(Service::Trello), t(Service::Jira)]),
            "3 tickets importés depuis Jira et Trello"
        );
        assert_eq!(
            imported_label(&[t(Service::Github), t(Service::Jira), t(Service::Trello)]),
            "3 tickets importés depuis GitHub, Jira et Trello"
        );
    }
}
