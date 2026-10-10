//! The integrations on the core: the accounts saved apart (their secrets in the system's
//! keychain, `secrets`), what the window asks of the services (containers, states, tickets), the
//! import (by hand or by label), and the sync of an imported ticket's external one when it changes
//! column or begins a loop: its operations kept until they go through, tried again later when one
//! fails.

use super::secrets;
use super::text::{column_comment, default_states, loop_comment};
use super::*;
use crate::core::Core;
use crate::i18n::{self, Lang};
use crate::paths;
use crate::tickets::TicketDraft;
use crate::{board, git};
use std::path::Path;
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

/// What an external ticket is told: one call (or a few) to its service.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub(crate) enum SyncOp {
    /// It is given `state`; `mapped`: every state the link gives a column (GitHub takes the labels
    /// of the others off).
    State {
        state: ExternalState,
        mapped: Vec<ExternalState>,
        /// The column whose state it is (none in a file of before: then only a move back to
        /// « À faire » takes it away, `SyncQueue::moved`).
        #[serde(default)]
        column: Option<Column>,
    },
    Comment {
        text: String,
    },
}

/// An operation of an imported ticket not through yet: its turn has not come, it failed and waits
/// for its next try, or it is set aside. It carries what it needs (the external ticket, the text
/// as it was written then), so a restart tries it again as it was.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PendingSync {
    pub ticket_id: String,
    pub external: ExternalRef,
    pub op: SyncOp,
    /// How many times it failed.
    #[serde(default)]
    pub tries: u32,
    /// When it first failed (ms): it is tried by itself for a day from then.
    #[serde(default)]
    pub failed_at: Option<i64>,
    /// When it is tried again by itself.
    #[serde(default)]
    pub retry_at: Option<i64>,
    /// Refused for good, or failing for a day: no longer tried by itself nor holding its ticket's
    /// next ones back, kept for « Resynchroniser ».
    #[serde(default)]
    pub set_aside: bool,
    /// Why it last failed (the ticket's ⚠).
    #[serde(default)]
    pub error: Option<String>,
}

impl PendingSync {
    /// `op` of the ticket `t`, whose external one is `ext`, not tried yet.
    fn new(t: &Ticket, ext: &ExternalRef, op: SyncOp) -> Self {
        Self {
            ticket_id: t.id.clone(),
            external: ExternalRef {
                error: None,
                ..ext.clone()
            },
            op,
            tries: 0,
            failed_at: None,
            retry_at: None,
            set_aside: false,
            error: None,
        }
    }
}

/// The waits after each failure of an operation, in minutes; the last one is kept from then on.
const RETRY_AFTER_MIN: [i64; 4] = [1, 5, 15, 60];
/// How long after its first failure an operation is still tried by itself.
const RETRY_FOR_MS: i64 = 24 * 60 * 60 * 1000;

/// When an operation that has just failed for the `tries`th time (the first one at `failed_at`)
/// is tried again; none once that would be more than a day after its first failure (set aside).
pub(crate) fn next_try(tries: u32, failed_at: i64, now: i64) -> Option<i64> {
    let i = (tries.max(1) as usize - 1).min(RETRY_AFTER_MIN.len() - 1);
    let at = now + RETRY_AFTER_MIN[i] * 60_000;
    (at <= failed_at + RETRY_FOR_MS).then_some(at)
}

/// The operations of the imported tickets not through yet, in the order of their changes, kept in
/// `sync-queue.json`. A ticket's go one after the other: one that failed holds the next ones back
/// until it goes through (or is set aside), so its external ticket hears of them in order; the
/// other tickets' go on meanwhile.
#[derive(Debug)]
pub(crate) struct SyncQueue {
    pub ops: Vec<PendingSync>,
    /// What the file holds, so that it is not written again for nothing.
    saved: Vec<u8>,
}

impl SyncQueue {
    /// The operations as read from the file.
    pub fn new(ops: Vec<PendingSync>) -> Self {
        let saved = serde_json::to_vec_pretty(&ops).unwrap_or_default();
        Self { ops, saved }
    }

    /// A ticket's new operations, after those of it still waiting. A transition takes the place
    /// of the ticket's one not through yet (set aside or not): going there now would only undo
    /// it.
    pub fn push(&mut self, ops: Vec<PendingSync>) {
        for op in &ops {
            if matches!(op.op, SyncOp::State { .. }) {
                self.ops.retain(|p| {
                    p.ticket_id != op.ticket_id || !matches!(p.op, SyncOp::State { .. })
                });
            }
        }
        self.ops.extend(ops);
    }

    /// The ticket came back into `to`, a column before the one of its transition not through yet
    /// (« À faire » always is): that transition goes (set aside or not), even when `to` gives the
    /// external ticket no state in its place. Sent later, it would put the external ticket where
    /// the ticket no longer is. Moved on into a column without a state, the ticket keeps it: the
    /// external ticket would be there had the service answered. True when one went.
    pub fn moved(&mut self, ticket_id: &str, to: Column) -> bool {
        let back = |p: &PendingSync| match &p.op {
            SyncOp::State { column, .. } => {
                to == Column::Todo || column.is_some_and(|from| to < from)
            }
            SyncOp::Comment { .. } => false,
        };
        let before = self.ops.len();
        self.ops.retain(|p| p.ticket_id != ticket_id || !back(p));
        self.ops.len() != before
    }

    /// Where the ticket's `k`th operation is in the queue.
    fn position(&self, ticket_id: &str, k: usize) -> Option<usize> {
        self.ops
            .iter()
            .enumerate()
            .filter(|(_, p)| p.ticket_id == ticket_id)
            .nth(k)
            .map(|(i, _)| i)
    }

    /// The ticket's `k`th operation, set aside or not.
    pub fn nth(&self, ticket_id: &str, k: usize) -> Option<PendingSync> {
        self.position(ticket_id, k).map(|i| self.ops[i].clone())
    }

    /// The ticket's next operation to go: its first one not set aside.
    pub fn head(&self, ticket_id: &str) -> Option<PendingSync> {
        self.ops
            .iter()
            .find(|p| p.ticket_id == ticket_id && !p.set_aside)
            .cloned()
    }

    /// The ticket's `k`th operation went through.
    pub fn done(&mut self, ticket_id: &str, k: usize) {
        if let Some(i) = self.position(ticket_id, k) {
            self.ops.remove(i);
        }
    }

    /// The ticket's `k`th operation failed at `now`, for `why`. It waits for its next try, unless
    /// it is set aside (true): its refusal is `lasting`, its first failure is a day old, or it
    /// was set aside already (« Resynchroniser » tried it again). Set aside, it lets its ticket's
    /// next ones go.
    pub fn failed(
        &mut self,
        ticket_id: &str,
        k: usize,
        now: i64,
        why: &str,
        lasting: bool,
    ) -> bool {
        let Some(i) = self.position(ticket_id, k) else {
            return false;
        };
        let p = &mut self.ops[i];
        p.tries += 1;
        p.error = Some(why.to_string());
        let first = *p.failed_at.get_or_insert(now);
        p.retry_at = if p.set_aside || lasting {
            None
        } else {
            next_try(p.tries, first, now)
        };
        p.set_aside = p.retry_at.is_none();
        p.set_aside
    }

    /// Why the ticket's first operation still here that failed did (the ⚠ it keeps).
    pub fn error_of(&self, ticket_id: &str) -> Option<String> {
        self.ops
            .iter()
            .filter(|p| p.ticket_id == ticket_id)
            .find_map(|p| p.error.clone())
    }

    /// The ticket's operations go (its card is gone).
    pub fn forget(&mut self, ticket_id: &str) {
        self.ops.retain(|p| p.ticket_id != ticket_id);
    }

    /// The tickets with operations waiting, in the order of their first one.
    pub fn tickets(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for p in &self.ops {
            if !out.contains(&p.ticket_id) {
                out.push(p.ticket_id.clone());
            }
        }
        out
    }

    /// The tickets whose next operation, which failed, is to be tried again by `now`.
    pub fn due(&self, now: i64) -> Vec<String> {
        self.tickets()
            .into_iter()
            .filter(|id| {
                self.head(id)
                    .and_then(|p| p.retry_at)
                    .is_some_and(|at| at <= now)
            })
            .collect()
    }

    /// Writes the operations to `path` when they changed since the file was last written.
    pub fn save(&mut self, path: &Path) {
        let Ok(bytes) = serde_json::to_vec_pretty(&self.ops) else {
            return;
        };
        if bytes == self.saved {
            return;
        }
        match paths::write_atomic(path, &bytes) {
            Ok(()) => self.saved = bytes,
            Err(e) => log::error!("cannot save {}: {e}", path.display()),
        }
    }
}

/// How long `gh auth token` may take.
const GH_TOKEN_LIMIT: Duration = Duration::from_secs(15);

/// The ticket of an external one, as the import makes it: its description says where it comes
/// from, its criteria are those found (when asked for), else the defaults.
pub(crate) fn draft_of(i: &ExternalIssue, max_loops: u32, extract: bool) -> TicketDraft {
    let mut description = i.description.trim().to_string();
    let (service, key, url) = (i.service.label(), &i.key, &i.url);
    let from = tr!(
        "Ticket {service} {key} : {url}",
        "{service} ticket {key}: {url}"
    );
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
        branch: String::new(),
    }
}

/// How a project remembers an imported ticket (`ProjectIntegrations::imported`).
fn imported_key(i: &ExternalIssue) -> String {
    format!("{}|{}", i.service.label().to_lowercase(), i.id)
}

/// What the sync's worker is asked to do.
pub(crate) enum SyncJob {
    /// Tell an imported ticket's external one of a change, after the ticket's operations still
    /// waiting.
    Change(Box<Ticket>, Change),
    /// Try again the operations whose next try has come (the app's timer).
    Due,
    /// Try again every operation left waiting (the app starts).
    All,
    /// "Resynchroniser": try again a ticket's operations at once, and say how it went.
    Resync(String, tokio::sync::oneshot::Sender<Result<()>>),
}

/// The sending side of the sync's worker.
pub(crate) type SyncSender = tokio::sync::mpsc::UnboundedSender<SyncJob>;

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
pub(crate) fn imported_label(lang: Lang, tickets: &[Ticket]) -> String {
    let (n, from) = (tickets.len(), sources(lang, tickets));
    tr_n_in!(
        lang,
        n,
        "{n} ticket importé depuis {from}",
        "{n} tickets importés depuis {from}",
        "{n} ticket imported from {from}",
        "{n} tickets imported from {from}"
    )
}

/// What an automatic import tells: « 3 tickets importés depuis Jira dans demo ». The label is a
/// whole clause, followed by where the tickets went in both languages.
pub(crate) fn imported_into(lang: Lang, tickets: &[Ticket], project: &str) -> String {
    let label = imported_label(lang, tickets);
    tr_in!(lang, "{label} dans {project}", "{label} into {project}")
}

/// The services the tickets come from, each once, as a list of `lang` (« Jira, GitHub et
/// Trello », “Jira, GitHub, and Trello”, as the window's `fList`).
fn sources(lang: Lang, tickets: &[Ticket]) -> String {
    let mut services: Vec<Service> = Vec::new();
    for t in tickets {
        if let Some(e) = &t.external {
            if !services.contains(&e.service) {
                services.push(e.service);
            }
        }
    }
    let names: Vec<&str> = services.iter().map(|s| s.label()).collect();
    match names.as_slice() {
        [] => String::new(),
        [one] => one.to_string(),
        [a, b] => tr_in!(lang, "{a} et {b}", "{a} and {b}"),
        [rest @ .., last] => {
            let rest = rest.join(", ");
            tr_in!(lang, "{rest} et {last}", "{rest}, and {last}")
        }
    }
}

/// What the form of a service's account lacks, in a sentence: Jira's site, its e-mail, the token,
/// Trello's API key. None when nothing is missing (GitHub's token may be the CLI's).
fn missing_field(lang: Lang, service: Service, account: &Account) -> Option<String> {
    Some(match service {
        Service::Jira if account.site.is_empty() => {
            tr_in!(
                lang,
                "Indique l'adresse du site.",
                "Give the site’s address."
            )
        }
        Service::Jira if account.email.is_empty() => {
            tr_in!(lang, "Indique l'e-mail.", "Give the email.")
        }
        Service::Jira | Service::Trello if account.token.is_empty() => {
            tr_in!(lang, "Indique le jeton.", "Give the token.")
        }
        Service::Trello if account.key.is_empty() => {
            tr_in!(lang, "Indique la clé d'API.", "Give the API key.")
        }
        _ => return None,
    })
}

impl<R: Runtime> Core<R> {
    fn save_accounts(&self) -> Result<()> {
        secrets::save_accounts(&self.data, &self.accounts.read())
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
        let account = self.accounts.read().get(service).cloned().ok_or_else(|| {
            let name = service.label();
            anyhow!(tr!(
                "Aucun compte {name} connecté",
                "No {name} account connected"
            ))
        })?;
        // Without its secrets, a call would be refused, or for GitHub go out with the CLI's
        // token, as whoever it is logged in as.
        if account.unread {
            let name = service.label();
            bail!(tr!(
                "Trousseau du système illisible : relance Escouade ou reconnecte le compte {name}.",
                "The system keychain can’t be read: restart Escouade or connect the {name} account again."
            ));
        }
        self.client_for(service, &account).await
    }

    /// The GitHub CLI's token (`gh auth token`), asked once.
    async fn gh_token(&self) -> Result<String> {
        if let Some(t) = self.gh_token.lock().clone() {
            return Ok(t);
        }
        let none = || {
            anyhow!(tr!(
                "GitHub : donne un jeton, ou connecte la CLI gh (gh auth login)",
                "GitHub: give a token, or log the gh CLI in (gh auth login)"
            ))
        };
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
    /// of them (label, user): their secrets into the system's keychain first, so that the file
    /// leaves them out only once it holds them.
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
        if let Some(missing) = missing_field(i18n::ui(), service, &account) {
            bail!(missing);
        }
        if service == Service::Jira && !jira::secure(&account.site) {
            bail!(tr!(
                "Le site Jira doit être en https (http seulement sur cette machine).",
                "The Jira site must use https (http only on this machine)."
            ));
        }
        if service == Service::Github && account.token.is_empty() {
            // The CLI may have been logged in again since its token was read.
            *self.gh_token.lock() = None;
        }
        let (mut label, user) = self.client_for(service, &account).await?.me().await?;
        if service == Service::Github && account.token.is_empty() {
            label.push_str(VIA_GH);
        }
        account.label = label;
        account.user = user;
        let left = !secrets::place(&*self.secrets, service, &mut account);
        {
            let mut accounts = self.accounts.write();
            // The entry is this account's now: an earlier « Déconnecter » must not take it.
            accounts.to_forget.retain(|&s| s != service);
            // Through the CLI, the token of a former account goes at the next start.
            if left {
                accounts.to_forget.push(service);
            }
            accounts.set(service, Some(account));
        }
        self.save_accounts()?;
        Ok(self
            .integration_accounts()
            .into_iter()
            .find(|a| a.service == service)
            .expect("every service has a view"))
    }

    /// "Déconnecter": the account is forgotten, its secrets with it (the projects keep their links,
    /// unused until one is connected again). An entry the keychain keeps goes at the next start
    /// (`Accounts::to_forget`); the window is told when it held the account's token.
    pub fn integration_disconnect(&self, service: Service) -> Result<Vec<AccountView>> {
        // Not for GitHub through the CLI (no token of its own), nor a token the keychain refused
        // (in the file, which is written again without it).
        let in_keychain = self
            .accounts
            .read()
            .get(service)
            .is_some_and(|a| !secrets::through_gh(service, a) && !a.in_file);
        let forgotten = secrets::forget(&*self.secrets, service);
        if !forgotten && in_keychain {
            let name = service.label();
            self.hub.emit(UiEvent::Toast {
                text: tr!(
                    "Le jeton {name} n'a pas pu être retiré du trousseau du système : Escouade réessaiera au prochain démarrage, ou retire-le à la main (son nom contient « escouade »).",
                    "The {name} token couldn’t be removed from the system keychain: Escouade will try again at its next start, or remove it by hand (its name holds “escouade”)."
                ),
            });
        }
        {
            let mut accounts = self.accounts.write();
            if !forgotten && !accounts.to_forget.contains(&service) {
                accounts.to_forget.push(service);
            }
            accounts.set(service, None);
        }
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
                        name: tr!(
                            "{repo} (dépôt du projet)",
                            "{repo} (the project’s repository)"
                        ),
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
        let link = project.integrations.link(service).ok_or_else(|| {
            let name = service.label();
            anyhow!(tr!(
                "Aucune source {name} liée à ce projet",
                "No {name} source linked to this project"
            ))
        })?;
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
                    let text = imported_into(i18n::ui(), &made, &p.name);
                    log::info!("automatic import: {text}");
                    self.hub.emit(UiEvent::Toast { text });
                }
                Ok(_) => {}
                Err(e) => log::warn!("automatic import into {}: {e:#}", p.name),
            }
        }
    }

    /// Tells the ticket's external one of `change`, in the background.
    pub(crate) fn sync_external(&self, ticket: Ticket, change: Change) {
        self.sync_job(SyncJob::Change(Box::new(ticket), change));
    }

    /// The failed operations whose next try has come go again (the app's timer looks every so
    /// often: the waits are minutes long).
    pub(crate) fn retry_due_syncs(&self) {
        let due = !self.pending_syncs.lock().due(self.sync_clock()).is_empty();
        if due {
            self.sync_job(SyncJob::Due);
        }
    }

    /// At the app's start, every operation left waiting goes again (those set aside stay so): the
    /// service may be back. Those of tickets deleted since go.
    pub(crate) fn retry_all_syncs(&self) {
        let any = !self.pending_syncs.lock().ops.is_empty();
        if any {
            self.sync_job(SyncJob::All);
        }
    }

    /// "Resynchroniser": the ticket's operations not through yet go again at once, those set
    /// aside included; with none left (a ⚠ an earlier version left), its external one is given
    /// its column's state again. Err: why one still fails (its ticket says so too).
    pub async fn integration_resync(&self, ticket_id: &str) -> Result<()> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.sync_job(SyncJob::Resync(ticket_id.to_string(), tx));
        rx.await
            .map_err(|_| anyhow!(tr!("Synchro interrompue", "Sync interrupted")))?
    }

    /// One worker takes the jobs one at a time, in the order they were asked for: an external
    /// ticket hears of its ticket's changes in the order they were made.
    fn sync_job(&self, job: SyncJob) {
        let mut queue = self.sync_queue.lock();
        if queue.as_ref().is_none_or(|q| q.is_closed()) {
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<SyncJob>();
            // The worker does not keep the core alive: it ends with it.
            let me = self.weak();
            tauri::async_runtime::spawn(async move {
                while let Some(job) = rx.recv().await {
                    let Some(c) = me.upgrade() else { break };
                    c.run_sync_job(job).await;
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
            let _ = q.send(job);
        }
    }

    async fn run_sync_job(self: &Arc<Self>, job: SyncJob) {
        match job {
            SyncJob::Change(t, change) => {
                // Back into a column without a state, its transition not through yet goes; into one
                // with a state, the new transition takes its place (`SyncQueue::push`).
                let moved = match change {
                    Change::Column(to) => self.pending_syncs.lock().moved(&t.id, to),
                    Change::Loop => false,
                };
                let ops = self.ops_of(&t, change);
                if ops.is_empty() && !moved {
                    return;
                }
                self.pending_syncs.lock().push(ops);
                if self.pending_syncs.lock().head(&t.id).is_some() {
                    // Those of the ticket still waiting go first, at once: the service may be
                    // back. Those held behind a transition that went go now too.
                    let _ = self.run_ticket_syncs(&t.id, false).await;
                } else {
                    // Only a transition went: the ⚠ says what is still set aside, if anything.
                    self.save_pending_syncs();
                    let error = self.pending_syncs.lock().error_of(&t.id);
                    self.note_sync_error(&t.id, error);
                }
            }
            SyncJob::Due => {
                let due = self.pending_syncs.lock().due(self.sync_clock());
                for id in due {
                    let _ = self.run_ticket_syncs(&id, false).await;
                }
            }
            SyncJob::All => {
                let all = self.pending_syncs.lock().tickets();
                for id in all {
                    let _ = self.run_ticket_syncs(&id, false).await;
                }
            }
            SyncJob::Resync(id, reply) => {
                let _ = reply.send(self.resync(&id).await);
            }
        }
    }

    /// What an imported ticket's external one is told of `change`: its column's state, then the
    /// comment the project asks for in that column; or the loop's summary. Written now, as the
    /// ticket is: a later try sends the same.
    fn ops_of(&self, t: &Ticket, change: Change) -> Vec<PendingSync> {
        let Some(ext) = t.external.as_ref() else {
            return Vec::new();
        };
        let Ok(project) = self.project(&t.project_id) else {
            return Vec::new();
        };
        let lang = self.lang().claude;
        let mut ops = Vec::new();
        match change {
            Change::Column(column) => {
                ops.extend(self.state_op(&project, ext, column));
                if project.integrations.comments.contains(&column) {
                    ops.push(SyncOp::Comment {
                        text: column_comment(lang, t, column, Some(&self.branch_of_ticket(t))),
                    });
                }
            }
            Change::Loop if self.settings.read().integrations.loop_comments => {
                ops.push(SyncOp::Comment {
                    text: loop_comment(lang, t),
                })
            }
            Change::Loop => {}
        }
        ops.into_iter()
            .map(|op| PendingSync::new(t, ext, op))
            .collect()
    }

    /// The state the project's link gives `column`, when the states are synced.
    fn state_op(&self, project: &Project, ext: &ExternalRef, column: Column) -> Option<SyncOp> {
        if !self.settings.read().integrations.sync_states {
            return None;
        }
        let link = project
            .integrations
            .link(ext.service)
            .filter(|l| l.container == ext.container)?;
        let state = link.states.get(&column).filter(|st| !st.id.is_empty())?;
        Some(SyncOp::State {
            state: state.clone(),
            mapped: link.states.values().cloned().collect(),
            column: Some(column),
        })
    }

    /// The ticket's operations in order, until one fails and waits for its next try (the ones
    /// after it with it) or all went through; one set aside lets the next ones go, and is only
    /// tried again with `set_aside` (« Resynchroniser »). The ticket's ⚠ says the last failure,
    /// else that of an operation still set aside, else goes. Err: the last failure.
    async fn run_ticket_syncs(self: &Arc<Self>, ticket_id: &str, set_aside: bool) -> Result<()> {
        let Ok(ticket) = self.ticket(ticket_id) else {
            // Its card is gone: nothing left to show them on, nor to try them again from.
            self.pending_syncs.lock().forget(ticket_id);
            self.save_pending_syncs();
            return Ok(());
        };
        let mut client = None;
        let mut tried = false;
        let mut failure = None;
        // The ticket's operations are taken by their rank among its own: those that stay (set
        // aside) are stepped over.
        let mut k = 0;
        loop {
            let next = self.pending_syncs.lock().nth(ticket_id, k);
            let Some(p) = next else { break };
            if p.set_aside && !set_aside {
                k += 1;
                continue;
            }
            tried = true;
            let held = match self.run_sync_op(&p, &mut client).await {
                Ok(()) => {
                    self.pending_syncs.lock().done(ticket_id, k);
                    false
                }
                Err(e) => {
                    let (service, key) = (p.external.service.label(), &p.external.key);
                    log::warn!("sync of {} with {service} {key}: {e:#}", ticket.key);
                    let why = board::first_line(&format!("{e:#}"));
                    let now = self.sync_clock();
                    let aside =
                        self.pending_syncs
                            .lock()
                            .failed(ticket_id, k, now, &why, lasting(&e));
                    if aside {
                        log::warn!("sync of {} with {service} {key} set aside", ticket.key);
                        k += 1;
                    }
                    failure = Some(e);
                    !aside
                }
            };
            self.save_pending_syncs();
            if held {
                break;
            }
        }
        if !tried {
            return Ok(());
        }
        let error = match &failure {
            Some(e) => Some(board::first_line(&format!("{e:#}"))),
            None => self.pending_syncs.lock().error_of(ticket_id),
        };
        self.note_sync_error(ticket_id, error);
        failure.map_or(Ok(()), Err)
    }

    /// One operation; the service's client made for the first one of the ticket.
    async fn run_sync_op(&self, p: &PendingSync, client: &mut Option<Client>) -> Result<()> {
        if client.is_none() {
            *client = Some(self.client(p.external.service).await?);
        }
        let c = client.as_ref().expect("made above");
        match &p.op {
            SyncOp::State { state, mapped, .. } => c.set_state(&p.external, state, mapped).await,
            SyncOp::Comment { text } => c.comment(&p.external, text).await,
        }
    }

    /// See `integration_resync`.
    async fn resync(self: &Arc<Self>, ticket_id: &str) -> Result<()> {
        let t = self.ticket(ticket_id)?;
        let Some(ext) = t.external.as_ref() else {
            return Ok(());
        };
        let kept = self.pending_syncs.lock().nth(ticket_id, 0).is_some();
        if !kept {
            let project = self.project(&t.project_id)?;
            let Some(op) = self.state_op(&project, ext, t.column) else {
                // Nothing to give it again (a comment is not written twice): the ⚠ goes.
                self.note_sync_error(ticket_id, None);
                return Ok(());
            };
            self.pending_syncs
                .lock()
                .push(vec![PendingSync::new(&t, ext, op)]);
        }
        self.run_ticket_syncs(ticket_id, true).await
    }

    /// The ticket's ⚠ says `error` (none: it goes), against the ticket as it is now.
    fn note_sync_error(&self, ticket_id: &str, error: Option<String>) {
        let now = self
            .ticket(ticket_id)
            .ok()
            .and_then(|x| x.external)
            .map(|e| e.error);
        if now.is_some_and(|now| now != error) {
            let _ = self.edit_ticket(ticket_id, |x| {
                if let Some(e) = x.external.as_mut() {
                    e.error = error;
                }
                Ok(())
            });
        }
    }

    fn save_pending_syncs(&self) {
        self.pending_syncs.lock().save(&self.data.sync_queue_file());
    }

    /// The time the retries go by: the real one (ahead by `clock_ahead` in tests).
    fn sync_clock(&self) -> i64 {
        #[cfg(test)]
        return now_ms() + self.clock_ahead.load(std::sync::atomic::Ordering::SeqCst);
        #[cfg(not(test))]
        now_ms()
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

    #[test]
    fn the_import_says_in_english_how_many_and_from_where() {
        use crate::i18n::Lang::En;
        let t = |s: Service| Ticket {
            external: Some(ExternalRef {
                service: s,
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(
            imported_label(En, &[t(Service::Jira)]),
            "1 ticket imported from Jira"
        );
        assert_eq!(
            imported_label(En, &[t(Service::Jira), t(Service::Trello)]),
            "2 tickets imported from Jira and Trello"
        );
        // As the window's list, with the comma of English before « and ».
        assert_eq!(
            imported_into(
                En,
                &[t(Service::Github), t(Service::Jira), t(Service::Trello)],
                "demo"
            ),
            "3 tickets imported from GitHub, Jira, and Trello into demo"
        );
        assert_eq!(
            missing_field(En, Service::Trello, &Account::default()).as_deref(),
            Some("Give the token.")
        );
    }

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

    const MIN: i64 = 60_000;

    fn pending(ticket: &str, op: SyncOp) -> PendingSync {
        PendingSync {
            ticket_id: ticket.into(),
            external: ExternalRef::default(),
            op,
            tries: 0,
            failed_at: None,
            retry_at: None,
            set_aside: false,
            error: None,
        }
    }

    /// The ticket's next operation failed at `now`, as a service down fails.
    fn fail(q: &mut SyncQueue, ticket: &str, now: i64) -> bool {
        q.failed(ticket, 0, now, "Trello : erreur 500", false)
    }

    /// A transition to `list`, its column not known (as a file of before has it).
    fn move_to(list: &str) -> SyncOp {
        SyncOp::State {
            state: ExternalState {
                id: list.into(),
                name: list.into(),
            },
            mapped: Vec::new(),
            column: None,
        }
    }

    /// The transition to `list` that `column` gives.
    fn move_in(list: &str, column: Column) -> SyncOp {
        match move_to(list) {
            SyncOp::State { state, mapped, .. } => SyncOp::State {
                state,
                mapped,
                column: Some(column),
            },
            op => op,
        }
    }

    fn say(text: &str) -> SyncOp {
        SyncOp::Comment { text: text.into() }
    }

    #[test]
    fn a_failed_operation_is_tried_again_after_1_5_15_then_60_minutes_for_a_day() {
        assert_eq!(next_try(1, 0, 0), Some(MIN));
        assert_eq!(next_try(2, 0, MIN), Some(6 * MIN));
        assert_eq!(next_try(3, 0, 6 * MIN), Some(21 * MIN));
        assert_eq!(next_try(4, 0, 21 * MIN), Some(81 * MIN));
        assert_eq!(next_try(12, 0, 561 * MIN), Some(621 * MIN));
        // The day counts from its first failure, not from the last.
        assert_eq!(next_try(2, 10 * MIN, 11 * MIN), Some(16 * MIN));
        assert_eq!(next_try(27, 0, 1380 * MIN), Some(1440 * MIN));
        assert_eq!(next_try(27, 0, 1401 * MIN), None);
    }

    #[test]
    fn a_tickets_operations_go_in_order_and_wait_behind_one_that_failed() {
        let mut q = SyncQueue::new(Vec::new());
        q.push(vec![
            pending("t1", move_to("l2")),
            pending("t1", say("pris")),
        ]);
        q.push(vec![pending("t2", say("deux"))]);
        assert_eq!(q.tickets(), ["t1", "t2"]);
        assert_eq!(q.head("t1").unwrap().op, move_to("l2"));
        assert!(!fail(&mut q, "t1", 0));
        // It holds its ticket's next ones back until its next try, not the other ticket's.
        let head = q.head("t1").unwrap();
        assert_eq!(
            (head.op, head.tries, head.failed_at, head.retry_at),
            (move_to("l2"), 1, Some(0), Some(MIN))
        );
        assert_eq!(q.error_of("t1").as_deref(), Some("Trello : erreur 500"));
        assert_eq!(q.head("t2").unwrap().op, say("deux"));
        assert!(q.due(MIN - 1).is_empty());
        assert_eq!(q.due(MIN), ["t1"]);
        q.done("t1", 0);
        assert_eq!(q.head("t1").unwrap().op, say("pris"));
        assert_eq!(q.error_of("t1"), None);
        // Never tried: not due by itself (it goes as soon as its turn comes).
        assert!(q.due(i64::MAX).is_empty());
        q.done("t1", 0);
        q.done("t2", 0);
        assert!(q.ops.is_empty() && q.head("t1").is_none());
    }

    #[test]
    fn a_transition_takes_the_place_of_its_tickets_one_not_through_yet() {
        let mut q = SyncQueue::new(Vec::new());
        q.push(vec![pending("t1", move_to("l2"))]);
        fail(&mut q, "t1", 0);
        q.push(vec![pending("t1", say("boucle 2"))]);
        q.push(vec![pending("t2", move_to("l2"))]);
        q.push(vec![
            pending("t1", move_to("l3")),
            pending("t1", say("prêt")),
        ]);
        let ops: Vec<(&str, &SyncOp)> = q
            .ops
            .iter()
            .map(|p| (p.ticket_id.as_str(), &p.op))
            .collect();
        assert_eq!(
            ops,
            [
                ("t1", &say("boucle 2")),
                ("t2", &move_to("l2")),
                ("t1", &move_to("l3")),
                ("t1", &say("prêt"))
            ]
        );
        // A comment never replaces another.
        q.push(vec![pending("t1", say("prêt"))]);
        assert_eq!(q.ops.len(), 5);
        // A transition set aside is replaced as well.
        q.failed("t2", 0, 0, "Trello : introuvable (404)", true);
        q.push(vec![pending("t2", move_to("l3"))]);
        assert_eq!(q.nth("t2", 0).unwrap().op, move_to("l3"));
        assert_eq!(q.nth("t2", 1), None);
    }

    #[test]
    fn a_move_back_takes_its_tickets_transition_away_and_a_move_on_keeps_it() {
        let mut q = SyncQueue::new(Vec::new());
        q.push(vec![
            pending("t1", move_in("l3", Column::Review)),
            pending("t1", say("prêt")),
        ]);
        q.push(vec![pending("t2", move_in("l2", Column::Doing))]);
        // On into « Terminé » left « — inchangé »: the card would be in l3 had the service
        // answered.
        assert!(!q.moved("t1", Column::Done));
        assert_eq!(q.ops.len(), 3);
        // Back into « En cours », set aside or not: it goes, the comment and the other ticket's
        // stay.
        q.failed("t1", 0, 0, "Trello : introuvable (404)", true);
        assert!(q.moved("t1", Column::Doing));
        let ops: Vec<(&str, &SyncOp)> = q
            .ops
            .iter()
            .map(|p| (p.ticket_id.as_str(), &p.op))
            .collect();
        assert_eq!(
            ops,
            [("t1", &say("prêt")), ("t2", &move_in("l2", Column::Doing))]
        );
        assert!(!q.moved("t1", Column::Todo));
        // « À faire » is always back, even for a transition whose column is not known.
        q.push(vec![pending("t2", move_to("l2"))]);
        assert!(!q.moved("t2", Column::Review));
        assert!(q.moved("t2", Column::Todo));
        assert_eq!(q.ops.len(), 1);
    }

    #[test]
    fn an_operation_failing_for_a_day_is_set_aside_and_the_next_ones_go_on() {
        let mut q = SyncQueue::new(Vec::new());
        q.push(vec![pending("t1", say("un")), pending("t1", say("deux"))]);
        let mut now = 0;
        let mut tries = 1;
        while !fail(&mut q, "t1", now) {
            now = q.head("t1").unwrap().retry_at.unwrap();
            tries += 1;
        }
        assert_eq!((now, tries), (1401 * MIN, 27));
        let next = q.head("t1").unwrap();
        assert_eq!((next.op, next.tries), (say("deux"), 0));
        // Kept for « Resynchroniser », never tried by itself.
        let aside = q.nth("t1", 0).unwrap();
        assert_eq!(
            (aside.op, aside.set_aside, aside.retry_at),
            (say("un"), true, None)
        );
        assert!(q.due(i64::MAX).is_empty());
        assert_eq!(q.error_of("t1").as_deref(), Some("Trello : erreur 500"));
    }

    #[test]
    fn a_refusal_that_would_not_change_is_set_aside_at_once_and_stays_so_until_it_goes() {
        let mut q = SyncQueue::new(Vec::new());
        q.push(vec![pending("t1", say("un")), pending("t1", move_to("l2"))]);
        assert!(q.failed("t1", 0, 0, "Trello : introuvable (404)", true));
        // It holds nothing back.
        assert_eq!(q.head("t1").unwrap().op, move_to("l2"));
        assert_eq!(
            q.error_of("t1").as_deref(),
            Some("Trello : introuvable (404)")
        );
        // Tried again by hand, even failing as a service down fails, it stays aside.
        assert!(q.failed("t1", 0, MIN, "Trello : erreur 500", false));
        let aside = q.nth("t1", 0).unwrap();
        assert_eq!(
            (aside.set_aside, aside.retry_at, aside.tries),
            (true, None, 2)
        );
        assert_eq!(q.error_of("t1").as_deref(), Some("Trello : erreur 500"));
        q.done("t1", 0);
        assert_eq!(q.nth("t1", 0).unwrap().op, move_to("l2"));
        assert_eq!(q.error_of("t1"), None);
    }

    #[test]
    fn the_queue_is_written_when_it_changes_and_read_back_from_a_file_without_its_tries() {
        let path = crate::paths::test_dir("sync-queue-file").join("sync-queue.json");
        let mut q = SyncQueue::new(Vec::new());
        q.save(&path);
        // Nothing waits: no file for nothing.
        assert!(!path.exists());
        q.push(vec![
            pending("t1", say("un")),
            pending("t1", move_in("l2", Column::Doing)),
        ]);
        fail(&mut q, "t1", 0);
        q.save(&path);
        let read: Vec<PendingSync> =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(read, q.ops);
        // As it was: not written again.
        std::fs::remove_file(&path).unwrap();
        q.save(&path);
        assert!(!path.exists());
        let old: Vec<PendingSync> = serde_json::from_str(
            r#"[{ "ticketId": "t1", "external": { "service": "jira", "id": "ATL-7" }, "op": { "kind": "comment", "text": "un" } }]"#,
        )
        .unwrap();
        assert_eq!(
            (old[0].op.clone(), old[0].tries, old[0].retry_at),
            (say("un"), 0, None)
        );
        assert_eq!((old[0].set_aside, &old[0].error), (false, &None));
        // A transition of before, without its column.
        let old: PendingSync = serde_json::from_str(
            r#"{ "ticketId": "t1", "external": { "service": "trello", "id": "c1" }, "op": { "kind": "state", "state": { "id": "l2", "name": "l2" }, "mapped": [] } }"#,
        )
        .unwrap();
        assert_eq!(old.op, move_to("l2"));
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
            imported_label(crate::i18n::Lang::Fr, &[t(Service::Jira)]),
            "1 ticket importé depuis Jira"
        );
        assert_eq!(
            imported_label(
                crate::i18n::Lang::Fr,
                &[t(Service::Jira), t(Service::Trello), t(Service::Jira)]
            ),
            "3 tickets importés depuis Jira et Trello"
        );
        assert_eq!(
            imported_label(
                crate::i18n::Lang::Fr,
                &[t(Service::Github), t(Service::Jira), t(Service::Trello)]
            ),
            "3 tickets importés depuis GitHub, Jira et Trello"
        );
    }
}
