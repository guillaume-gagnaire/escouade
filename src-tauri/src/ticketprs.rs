//! The pull requests of the tickets (ESC-59): « Valider + PR » opens one through the code host's
//! API (its plugin, `Forge`), the ticket waits « En revue » until it is merged there, and its card
//! follows it. Without a webhook (the app has no public address), Escouade reads the ones it
//! follows: every minute while the window is in front, every 5 minutes else, at once after a push
//! of Escouade and when the window comes back, and on « Rafraîchir ». An error of the host is said
//! on the card and never moves a ticket.

use crate::board;
use crate::core::Core;
use crate::forge::{Check, CheckState, Forge, PrState, PullRequest, RepoRef};
use crate::git;
use crate::i18n::{self, Lang};
use crate::model::{now_ms, Column, Service, Ticket, TicketPr};
use anyhow::{anyhow, bail, Result};
use std::collections::BTreeMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

/// How often the pull requests followed are read, the window in front or not.
pub const IN_FRONT_EVERY: Duration = Duration::from_secs(60);
pub const BEHIND_EVERY: Duration = Duration::from_secs(300);

pub fn every(in_front: bool) -> Duration {
    if in_front {
        IN_FRONT_EVERY
    } else {
        BEHIND_EVERY
    }
}

/// The issue of the same repository the ticket was imported from, which its pull request closes
/// once merged (« Closes #12 »): a GitHub issue on GitHub, a GitLab one on GitLab. Jira's and
/// Linear's keys are in the title already; Azure Boards' work item goes by `link_work_item`.
pub fn closes_issue(t: &Ticket, kind: &str, repo: &RepoRef) -> Option<u64> {
    let ext = t.external.as_ref()?;
    let same_host = (ext.service == Service::GITHUB && kind == "github")
        || (ext.service == Service::GITLAB && kind == "gitlab");
    let same_repo = ext.container.eq_ignore_ascii_case(&repo.full());
    if !(same_host && same_repo) {
        return None;
    }
    ext.id.trim_start_matches('#').parse().ok()
}

/// The page of the host that opens a pull request of `branch` into `target`, filled in, for
/// when no account opens it through the API: GitHub's and Gitea's `/compare/`, GitLab's
/// `merge_requests/new`, Bitbucket's `pull-requests/new`, Azure Repos' `pullrequestcreate`.
/// `url` is the instance's address (`https://github.com`). None for a host it does not know.
pub fn creation_page(
    kind: &str,
    url: &str,
    repo: &RepoRef,
    target: &str,
    branch: &str,
    title: &str,
    body: &str,
) -> Option<String> {
    let url = url.trim_end_matches('/');
    let enc = |s: &str| board::encode(s, false);
    let path = |s: &str| board::encode(s, true);
    let short = |s: &str, max| crate::text::truncate(s, max);
    let full = repo.full();
    Some(match kind {
        "github" | "gitea" => board::compare_url(url, &full, target, branch, title, body),
        "gitlab" => format!(
            "{url}/{full}/-/merge_requests/new?merge_request%5Bsource_branch%5D={}\
             &merge_request%5Btarget_branch%5D={}&merge_request%5Btitle%5D={}\
             &merge_request%5Bdescription%5D={}",
            enc(branch),
            enc(target),
            enc(&short(title, board::COMPARE_TITLE_MAX)),
            enc(&short(body, board::COMPARE_BODY_MAX))
        ),
        // Bitbucket's page takes neither a title nor a description.
        "bitbucket" => format!(
            "{url}/{full}/pull-requests/new?source={}&dest={}",
            enc(branch),
            enc(target)
        ),
        // `url` is the organization's (`https://dev.azure.com/acme`); the repository is
        // `<project>/<repo>`.
        "azure" => format!(
            "{url}/{}/_git/{}/pullrequestcreate?sourceRef={}&targetRef={}",
            path(&repo.namespace),
            path(&repo.name),
            enc(branch),
            enc(target)
        ),
        _ => return None,
    })
}

/// What a ticket keeps of the pull request its validation opened.
#[allow(clippy::too_many_arguments)]
pub fn ticket_pr(
    pr: &PullRequest,
    kind: &str,
    host: &str,
    repo: &RepoRef,
    word: &str,
    target: &str,
    branch: &str,
    now: i64,
) -> TicketPr {
    let mut tp = TicketPr {
        number: pr.number,
        url: pr.url.clone(),
        kind: kind.to_string(),
        host: host.to_string(),
        namespace: repo.namespace.clone(),
        name: repo.name.clone(),
        target: target.to_string(),
        branch: branch.to_string(),
        word: word.to_string(),
        ..Default::default()
    };
    read_into(&mut tp, pr, None, now);
    tp
}

/// What the host said of a pull request, on the ticket's.
fn read_into(tp: &mut TicketPr, pr: &PullRequest, checks: Option<&[Check]>, now: i64) {
    if !pr.url.is_empty() {
        tp.url = pr.url.clone();
    }
    if !pr.base.is_empty() {
        tp.target = pr.base.clone();
    }
    tp.state = pr.state;
    tp.draft = pr.draft;
    tp.checks = pr.checks;
    match checks {
        Some(list) => {
            let counted: Vec<&Check> = list
                .iter()
                .filter(|c| c.state != CheckState::Skipped)
                .collect();
            tp.checks_total = counted.len() as u32;
            tp.checks_failed = counted
                .iter()
                .filter(|c| c.state == CheckState::Failure)
                .count() as u32;
        }
        None if pr.checks.is_none() => {
            tp.checks_total = 0;
            tp.checks_failed = 0;
        }
        // Not read this time: the counts of the last reading stay.
        None => {}
    }
    tp.approvals = pr.approvals;
    tp.changes_requested = pr.changes_requested;
    tp.open_threads = pr.open_threads;
    tp.mergeable = pr.mergeable;
    tp.checked_at = Some(now);
    tp.error = None;
    tp.error_since = None;
}

/// What a reading changed of a ticket « En revue ».
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum News {
    /// Still open (or the ticket is no longer « En revue »).
    Same,
    /// Merged on the host: the ticket is « Terminé ».
    Merged,
    /// Closed without a merge: the ticket is blocked.
    Closed,
}

/// A reading of its pull request on a ticket « En revue »: its card's state, and, merged, the
/// ticket « Terminé » (`claude`: the language of the texts for Claude, for the comment the team
/// reads); closed without a merge, blocked with « Remettre à faire » and « Terminer quand même ».
pub fn settle(
    t: &mut Ticket,
    pr: &PullRequest,
    checks: Option<&[Check]>,
    now: i64,
    claude: Lang,
) -> News {
    if t.column != Column::Pr {
        return News::Same;
    }
    let Some(tp) = t.pr.as_mut() else {
        return News::Same;
    };
    read_into(tp, pr, checks, now);
    let (word, n, target) = (tp.word.clone(), tp.number, tp.target.clone());
    match pr.state {
        PrState::Open => {
            // Opened again on the host after it was closed: followed again.
            if tp.closed {
                tp.closed = false;
                t.blocked = None;
            }
            News::Same
        }
        PrState::Merged => {
            tp.closed = false;
            t.column = Column::Done;
            t.blocked = None;
            t.conflict = false;
            t.outcome = Some(merged_outcome(i18n::ui(), &word, n, &target));
            t.outcome_claude = Some(merged_outcome(claude, &word, n, &target));
            t.outcome_url = Some(tp.url.clone());
            t.done_at = Some(now);
            News::Merged
        }
        PrState::Closed if tp.closed => News::Same,
        PrState::Closed => {
            tp.closed = true;
            t.blocked = Some(closed_reason(i18n::ui(), &word, n));
            News::Closed
        }
    }
}

/// A reading that failed: the card says why and since when, the ticket stays where it is.
pub fn failed(tp: &mut TicketPr, why: String, now: i64) {
    tp.error = Some(why);
    tp.error_since = tp.error_since.or(Some(now));
}

/// Why a reading failed, in words: a token refused, else the host out of reach.
pub fn error_text(lang: Lang, name: &str, e: &anyhow::Error) -> String {
    if crate::forge::refused(e) {
        return tr_in!(
            lang,
            "{name} : jeton refusé ou expiré, reconnecte le compte",
            "{name}: token refused or expired, connect the account again"
        );
    }
    if crate::plugins::adapters::is_down(e) {
        return tr_in!(
            lang,
            "{name} : son plugin est arrêté",
            "{name}: its plugin is stopped"
        );
    }
    let why = format!("{e:#}");
    let why = why.lines().next().unwrap_or("").trim().to_string();
    tr_in!(
        lang,
        "{name} injoignable : {why}",
        "{name} out of reach: {why}"
    )
}

pub fn opened_outcome(word: &str, number: u64, target: &str) -> String {
    format!("⇡ {word} #{number} → {target}")
}

pub fn merged_outcome(lang: Lang, word: &str, number: u64, target: &str) -> String {
    tr_in!(
        lang,
        "⤵ {word} #{number} mergée dans {target}",
        "⤵ {word} #{number} merged into {target}"
    )
}

pub fn closed_reason(lang: Lang, word: &str, number: u64) -> String {
    tr_in!(
        lang,
        "{word} #{number} fermée sans merge",
        "{word} #{number} closed without a merge"
    )
}

pub fn finished_anyway_outcome(lang: Lang, word: &str, number: u64) -> String {
    tr_in!(
        lang,
        "✓ Terminé quand même · {word} #{number} fermée",
        "✓ Finished anyway · {word} #{number} closed"
    )
}

/// Where a followed pull request is read: its instance's host, its host's kind, its repository
/// (namespace, name).
pub type Place = (String, String, String, String);

/// The followed pull requests of `tickets`, by the account and the repository they are read
/// with: their place → (ticket id, number).
pub fn followed(tickets: &[Ticket], project: Option<&str>) -> BTreeMap<Place, Vec<(String, u64)>> {
    let mut out: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for t in tickets {
        if t.column != Column::Pr || project.is_some_and(|p| p != t.project_id) {
            continue;
        }
        let Some(tp) = &t.pr else { continue };
        if tp.closed {
            continue;
        }
        out.entry((
            tp.host.clone(),
            tp.kind.clone(),
            tp.namespace.clone(),
            tp.name.clone(),
        ))
        .or_default()
        .push((t.id.clone(), tp.number));
    }
    out
}

impl<R: tauri::Runtime> Core<R> {
    /// The pull requests of the tickets « En revue », read by themselves for as long as the app
    /// runs (`IN_FRONT_EVERY`, `BEHIND_EVERY`), and at once when woken (`wake_ticket_prs`).
    pub(crate) fn follow_ticket_prs(self: Arc<Self>) {
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_secs(10)).await;
            loop {
                if let Err(e) = self.ticket_prs_refresh(None).await {
                    log::info!("tickets' pull requests not read: {e:#}");
                }
                let wait = every(self.window_in_front.load(Ordering::Acquire));
                tokio::select! {
                    _ = tokio::time::sleep(wait) => {}
                    _ = self.ticket_prs_wake.notified() => {}
                }
            }
        });
    }

    /// The pull requests of the tickets « En revue » are read now (after a push of Escouade).
    pub fn wake_ticket_prs(&self) {
        self.ticket_prs_wake.notify_one();
    }

    /// The window came in front (read at once) or went behind (read every 5 minutes).
    pub fn window_front(&self, in_front: bool) {
        let was = self.window_in_front.swap(in_front, Ordering::AcqRel);
        if in_front && !was {
            self.wake_ticket_prs();
        }
    }

    /// Reads the pull requests of the tickets « En revue » (of `project`, else of all), one call
    /// per repository when its host reads several at once: how many were read.
    pub async fn ticket_prs_refresh(self: &Arc<Self>, project: Option<&str>) -> Result<usize> {
        let groups = followed(&self.tickets.read(), project);
        let mut read = 0;
        for ((host, kind, namespace, name), list) in groups {
            let repo = RepoRef { namespace, name };
            let label = self
                .forges
                .read()
                .get(&kind)
                .map_or_else(|| host.clone(), |p| p.name().to_string());
            let client = match self.forge_client(&host).await {
                Ok((c, _)) => c,
                Err(e) => {
                    let why = error_text(i18n::ui(), &label, &e);
                    for (id, _) in &list {
                        self.ticket_pr_failed(id, why.clone());
                    }
                    continue;
                }
            };
            let numbers: Vec<u64> = list.iter().map(|(_, n)| *n).collect();
            match read_prs(&*client, &repo, &numbers).await {
                Err(e) => {
                    let why = error_text(i18n::ui(), &label, &e);
                    for (id, _) in &list {
                        self.ticket_pr_failed(id, why.clone());
                    }
                }
                Ok(prs) => {
                    for (id, n) in &list {
                        let Some(pr) = prs.iter().find(|p| p.number == *n) else {
                            let e = anyhow!(tr!("#{n} introuvable", "#{n} not found"));
                            self.ticket_pr_failed(id, error_text(i18n::ui(), &label, &e));
                            continue;
                        };
                        // The count of its checks, when it has some (one call more).
                        let checks = if pr.state == PrState::Open && pr.checks.is_some() {
                            client.checks(&repo, *n).await.ok()
                        } else {
                            None
                        };
                        self.ticket_pr_read(id, pr, checks.as_deref()).await;
                        read += 1;
                    }
                }
            }
        }
        Ok(read)
    }

    fn ticket_pr_failed(&self, id: &str, why: String) {
        let _ = self.edit_ticket(id, |t| {
            if t.column == Column::Pr {
                if let Some(tp) = t.pr.as_mut() {
                    failed(tp, why, now_ms());
                }
            }
            Ok(())
        });
    }

    async fn ticket_pr_read(
        self: &Arc<Self>,
        id: &str,
        pr: &PullRequest,
        checks: Option<&[Check]>,
    ) {
        let claude = self.lang().claude;
        let news = self.edit_ticket(id, |t| Ok(settle(t, pr, checks, now_ms(), claude)));
        match news {
            Ok(News::Merged) => self.ticket_pr_merged(id).await,
            Ok(News::Closed) => self.notify_ticket(id, false),
            _ => {}
        }
    }

    /// Merged on its host: its local branch goes when the board cleans up, a fetch brings the
    /// merge to the target (fast-forwarded when it can be), then the tickets that came after it
    /// may start.
    async fn ticket_pr_merged(self: &Arc<Self>, id: &str) {
        let Ok(t) = self.ticket(id) else { return };
        let Some(tp) = t.pr.clone() else { return };
        self.facts.emit(
            crate::facts::Body::TicketMerged {
                target: tp.target.clone(),
                commit: None,
                url: Some(tp.url.clone()),
            },
            crate::facts::About::ticket(id),
        );
        if let Ok(project) = self.project(&t.project_id) {
            let repo = project.path.clone();
            if project.board.cleanup && t.branch.is_empty() && !tp.branch.is_empty() {
                if let Err(e) = git::run(&repo, &["branch", "-D", "--", &tp.branch]).await {
                    log::info!("ticket {}: branch {} kept: {e:#}", t.key, tp.branch);
                }
            }
            if !git::remotes(&repo).await.is_empty() {
                match git::fetch(&repo, true).await {
                    Ok(()) => {
                        if let Some(up) = git::upstream_of(&repo, &tp.target).await {
                            let moved =
                                git::fast_forward_branch(&repo, &tp.target, &up.tracking).await;
                            // Checked out in the project's folder: fast-forwarded there, when
                            // nothing of the folder's is in the way.
                            if moved.is_err() && git::current_branch(&repo).await == tp.target {
                                let _ =
                                    git::run(&repo, &["merge", "--ff-only", &up.tracking]).await;
                            }
                        }
                        self.refresh_repo(&repo).await;
                    }
                    Err(e) => log::info!("ticket {}: fetch after its merge failed: {e:#}", t.key),
                }
            }
        }
        self.notify_ticket(id, false);
        self.schedule();
    }

    /// « Terminer quand même »: its pull request closed without a merge, the ticket is done.
    pub fn ticket_finish_anyway(self: &Arc<Self>, id: &str) -> Result<()> {
        let claude = self.lang().claude;
        self.edit_ticket(id, |t| {
            let Some(tp) = t.pr.as_ref().filter(|p| p.closed && t.column == Column::Pr) else {
                bail!(not_closed());
            };
            let (word, n) = (tp.word.clone(), tp.number);
            t.column = Column::Done;
            t.blocked = None;
            t.outcome = Some(finished_anyway_outcome(i18n::ui(), &word, n));
            t.outcome_claude = Some(finished_anyway_outcome(claude, &word, n));
            t.done_at = Some(now_ms());
            Ok(())
        })?;
        self.schedule();
        Ok(())
    }

    /// « Remettre à faire »: its pull request closed without a merge, the ticket goes back to do
    /// from scratch, taking its branch up again when it is still here (its work stays).
    pub async fn ticket_pr_back_to_todo(self: &Arc<Self>, id: &str) -> Result<()> {
        let t = self.ticket(id)?;
        let Some(tp) = t.pr.clone().filter(|p| p.closed && t.column == Column::Pr) else {
            bail!(not_closed());
        };
        let path = self.project(&t.project_id)?.path;
        let keep = !tp.branch.is_empty() && git::branch_exists(&path, &tp.branch).await;
        self.edit_ticket(id, |t| {
            if !t.pr.as_ref().is_some_and(|p| p.closed) || t.column != Column::Pr {
                bail!(not_closed());
            }
            board::back_to_todo(t);
            t.pr = None;
            t.outcome = None;
            t.outcome_claude = None;
            t.outcome_url = None;
            if keep && t.branch.is_empty() {
                t.branch = tp.branch.clone();
            }
            Ok(())
        })?;
        self.schedule();
        Ok(())
    }
}

fn not_closed() -> anyhow::Error {
    anyhow!(tr!(
        "Ce ticket n’a pas de PR fermée sans merge.",
        "This ticket has no pull request closed without a merge."
    ))
}

/// The pull requests `numbers` of `repo`: in one call when the host reads several at once, else
/// one by one.
async fn read_prs(client: &dyn Forge, repo: &RepoRef, numbers: &[u64]) -> Result<Vec<PullRequest>> {
    if numbers.len() > 1 {
        match client.pull_requests(repo, numbers).await {
            Ok(prs) => return Ok(prs),
            Err(e) if e.downcast_ref::<crate::forge::Unsupported>().is_some() => {}
            Err(e) => return Err(e),
        }
    }
    let mut out = Vec::new();
    for n in numbers {
        out.push(client.pull_request(repo, *n).await?);
    }
    Ok(out)
}

#[cfg(test)]
#[path = "ticketprs_tests.rs"]
mod tests;
