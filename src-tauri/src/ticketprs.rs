//! The pull requests of the tickets (ESC-59): « Valider + PR » opens one through the code host's
//! API (its plugin, `Forge`), the ticket waits « En revue » until it is merged there, and its card
//! follows it. Without a webhook (the app has no public address), Escouade reads the ones it
//! follows: every minute while the window is in front, every 5 minutes else, at once after a push
//! of Escouade and when the window comes back, and on « Rafraîchir ». An error of the host is said
//! on the card and never moves a ticket.

use crate::board;
use crate::core::Core;
use crate::forge::{
    Capabilities, Check, CheckState, Forge, MergeOptions, MergeStrategy, PrState, PullRequest,
    RepoRef, Unsupported,
};
use crate::git;
use crate::i18n::{self, Lang};
use crate::model::{now_ms, BoardSettings, Column, FailedCheck, Service, Ticket, TicketPr};
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
            tp.failed = failed_of(list);
        }
        None if pr.checks.is_none() => {
            tp.checks_total = 0;
            tp.checks_failed = 0;
            tp.failed.clear();
        }
        // Not read this time: the counts of the last reading stay.
        None => {}
    }
    // Its CI runs again (or passed) since a fix: the next failure goes to its agent again.
    if pr.checks != Some(CheckState::Failure) {
        tp.awaiting_ci = false;
    }
    if pr.checks == Some(CheckState::Success) {
        tp.fix_error = None;
    }
    tp.approvals = pr.approvals;
    tp.changes_requested = pr.changes_requested;
    tp.open_threads = pr.open_threads;
    tp.mergeable = pr.mergeable;
    tp.auto_merge = pr.auto_merge;
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
    /// Approved and green while Escouade waited for it (« Merger quand c'est vert » without the
    /// host's auto-merge): Escouade merges it now with this strategy.
    MergeNow(MergeStrategy),
}

/// The Kanban's strategy (`BoardSettings::strategy`) as a host's.
pub fn strategy_of(board: &str) -> MergeStrategy {
    match board {
        "merge" => MergeStrategy::Merge,
        "rebase" => MergeStrategy::Rebase,
        _ => MergeStrategy::Squash,
    }
}

/// The strategy « Merger » uses: the Kanban's when the repository allows it, else the first it
/// allows; none when it allows none, or is not read yet.
pub fn pick_strategy(wanted: MergeStrategy, opts: Option<&MergeOptions>) -> Option<MergeStrategy> {
    let allowed = &opts?.strategies;
    if allowed.contains(&wanted) {
        Some(wanted)
    } else {
        allowed.first().copied()
    }
}

/// How a host lets a pull request merge when its plugin does not read the repository's settings:
/// what its provider declares.
pub fn options_of_capabilities(caps: &Capabilities) -> MergeOptions {
    MergeOptions {
        strategies: caps.merge_strategies.clone(),
        auto_merge: caps.auto_merge,
    }
}

/// Its CI passed, or it has none.
pub fn green(tp: &TicketPr) -> bool {
    matches!(tp.checks, None | Some(CheckState::Success))
}

/// Open, followed, not a draft, without a conflict with its target, and its agent not fixing its
/// CI (ESC-60): nothing merges before its fix is pushed.
fn open(tp: &TicketPr) -> bool {
    tp.state == PrState::Open
        && !tp.closed
        && !tp.draft
        && tp.mergeable != Some(false)
        && tp.fixing.is_none()
}

/// « Merger » is offered: approved (no change asked for), green, without a conflict.
pub fn merge_ready(tp: &TicketPr) -> bool {
    open(tp) && tp.approvals > 0 && !tp.changes_requested && green(tp)
}

/// « Merger quand c'est vert » is offered: its CI runs, and nothing merges it by itself yet.
pub fn can_wait_green(tp: &TicketPr) -> bool {
    open(tp)
        && !tp.changes_requested
        && tp.checks == Some(CheckState::Pending)
        && !tp.auto_merge
        && tp.merge_when_green.is_none()
}

pub fn merge_failed_note(lang: Lang, word: &str, number: u64) -> String {
    tr_in!(
        lang,
        "CI en échec : {word} #{number} n’est pas mergée, Escouade n’attend plus",
        "CI failed: {word} #{number} isn’t merged, Escouade no longer waits"
    )
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
            // Escouade waits for it (« Merger quand c'est vert », no auto-merge on the host).
            if let Some(strategy) = tp.merge_when_green {
                if tp.checks == Some(CheckState::Failure) || tp.changes_requested {
                    tp.merge_when_green = None;
                    tp.merge_note = Some(merge_failed_note(i18n::ui(), &word, n));
                } else if merge_ready(tp) {
                    return News::MergeNow(strategy);
                }
            }
            News::Same
        }
        PrState::Merged => {
            tp.closed = false;
            tp.merge_when_green = None;
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

// ---------- Its CI fixed by its agent (ESC-60) ----------

/// The checks of a reading that failed, each with whether its host keeps its log.
pub fn failed_of(list: &[Check]) -> Vec<FailedCheck> {
    list.iter()
        .filter(|c| c.state == CheckState::Failure)
        .map(|c| FailedCheck {
            name: c.name.clone(),
            url: c.url.clone(),
            log: c.job.is_some(),
        })
        .collect()
}

/// Its agent fixes its CI now: it holds a place of the board's agents in parallel.
pub fn fixing(t: &Ticket) -> bool {
    t.column == Column::Pr && t.pr.as_ref().is_some_and(|p| p.fixing.is_some())
}

/// Its CI may go to its agent (« L'agent corrige »): « En revue », its pull request open with
/// failed checks, an agent, and no fix under way.
pub fn fixable(t: &Ticket) -> bool {
    t.column == Column::Pr
        && t.agent_id.is_some()
        && t.pr.as_ref().is_some_and(|p| {
            !p.closed
                && p.state == PrState::Open
                && p.checks == Some(CheckState::Failure)
                && p.fixing.is_none()
        })
}

/// The autopilot gives it by itself: fixable, not given already since it last failed, within the
/// ticket's loops, and not a teammate's.
pub fn auto_fixable(t: &Ticket) -> bool {
    fixable(t)
        && t.blocked.is_none()
        && !crate::sharing::theirs(t)
        && t.pr
            .as_ref()
            .is_some_and(|p| !p.awaiting_ci && p.fixes < t.max_loops)
}

/// The tickets of `project_id` whose agents are at work and hold a place: « En cours » and not
/// blocked (not on a teammate's machine), and « En revue » while their agent fixes their CI.
pub fn busy(tickets: &[Ticket], project_id: &str) -> usize {
    tickets
        .iter()
        .filter(|t| t.project_id == project_id)
        .filter(|t| {
            (t.column == Column::Doing && t.blocked.is_none() && !crate::sharing::elsewhere(t))
                || fixing(t)
        })
        .count()
}

/// The tickets « En revue » of `project_id` whose CI the autopilot gives to their agent now, in
/// the board's order, while places are free.
pub fn to_fix(tickets: &[Ticket], project_id: &str, s: &BoardSettings) -> Vec<String> {
    if !s.autopilot {
        return Vec::new();
    }
    let free = (s.max_parallel.clamp(1, 6) as usize).saturating_sub(busy(tickets, project_id));
    tickets
        .iter()
        .filter(|t| t.project_id == project_id && auto_fixable(t))
        .take(free)
        .map(|t| t.id.clone())
        .collect()
}

/// A fix begins: counted (a loop of the ticket's), and what the last one left goes.
pub fn fix_begins(t: &mut Ticket, from: Option<String>, now: i64) {
    if let Some(tp) = t.pr.as_mut() {
        tp.fixing = Some(now);
        tp.fixes += 1;
        tp.awaiting_ci = false;
        tp.fix_error = None;
        tp.fix_from = from;
        t.loops += 1;
    }
}

/// A fix that did not go to its end: the card says why, the autopilot does not give the same
/// failure again (« L'agent corrige » does, by hand).
pub fn fix_failed(t: &mut Ticket, why: String) {
    if let Some(tp) = t.pr.as_mut() {
        tp.fixing = None;
        tp.fix_from = None;
        tp.awaiting_ci = true;
        tp.fix_error = Some(why);
    }
}

/// Fixed and pushed: back « En revue » as before, its CI to run again.
pub fn fix_done(t: &mut Ticket) {
    if let Some(tp) = t.pr.as_mut() {
        tp.fixing = None;
        tp.fix_from = None;
        tp.awaiting_ci = true;
        tp.fix_error = None;
    }
}

/// What the end of its agent's turn does to a ticket whose CI it fixes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FixEnd {
    /// Taken up by itself (the usage limit's resume, a sign-in): the fix goes on.
    Wait,
    /// Its turn finished: what it did is pushed.
    Finished,
    /// Stopped or failed: why.
    Stopped(String),
}

pub fn fix_end(end: &crate::board::TurnEnd) -> FixEnd {
    use crate::board::TurnEnd;
    match end {
        TurnEnd::Limited | TurnEnd::SignIn => FixEnd::Wait,
        TurnEnd::Finished(_) => FixEnd::Finished,
        TurnEnd::Interrupted => FixEnd::Stopped(crate::board::interrupted(i18n::ui())),
        TurnEnd::Error(e) => FixEnd::Stopped(crate::board::error_blocked(
            i18n::ui(),
            &crate::board::first_line(e),
        )),
    }
}

pub fn nothing_fixed(lang: Lang) -> String {
    tr_in!(
        lang,
        "L'agent n'a rien commité : la CI reste en échec",
        "The agent committed nothing: the CI still fails"
    )
}

pub fn not_committed(lang: Lang) -> String {
    tr_in!(
        lang,
        "Des changements de l'agent ne sont pas commités : rien n'est poussé",
        "Some of the agent’s changes aren’t committed: nothing was pushed"
    )
}

pub fn not_pushed(lang: Lang, why: &str) -> String {
    tr_in!(lang, "Push impossible : {why}", "Couldn’t push: {why}")
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
                        self.ticket_pr_read(id, pr, checks.as_deref(), &*client, &repo, &kind)
                            .await;
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
        client: &dyn Forge,
        repo: &RepoRef,
        kind: &str,
    ) {
        let claude = self.lang().claude;
        let news = self.edit_ticket(id, |t| Ok(settle(t, pr, checks, now_ms(), claude)));
        match news {
            Ok(News::Merged) => self.ticket_pr_merged(id).await,
            Ok(News::Closed) => self.notify_ticket(id, false),
            Ok(News::MergeNow(strategy)) => {
                let done = client.merge(repo, pr.number, strategy).await;
                self.ticket_pr_merge_done(id, pr.number, done, client, repo)
                    .await;
            }
            news => {
                // How the repository lets it merge, read once while it is open.
                let unread = matches!(news, Ok(News::Same))
                    && pr.state == PrState::Open
                    && self
                        .ticket(id)
                        .is_ok_and(|t| t.pr.is_some_and(|p| p.merge_options.is_none()));
                if unread {
                    if let Ok(opts) = self.merge_options(client, repo, pr.number, kind).await {
                        let _ = self.edit_ticket(id, |t| {
                            if let Some(tp) = t.pr.as_mut() {
                                tp.merge_options = Some(opts);
                            }
                            Ok(())
                        });
                    }
                }
                // Its CI failed: with the autopilot, its agent fixes it (`to_fix`, ESC-60).
                if self.ticket(id).is_ok_and(|t| auto_fixable(&t)) {
                    self.schedule();
                }
            }
        }
    }

    /// How the repository lets the pull request merge: as its plugin reads it, else as its
    /// provider declares (a plugin that does not read it).
    async fn merge_options(
        &self,
        client: &dyn Forge,
        repo: &RepoRef,
        number: u64,
        kind: &str,
    ) -> Result<MergeOptions> {
        match client.merge_options(repo, number).await {
            Err(e) if e.downcast_ref::<Unsupported>().is_some() => {
                let caps = self.forges.read().get(kind).map(|p| p.capabilities());
                Ok(options_of_capabilities(&caps.unwrap_or_default()))
            }
            other => other,
        }
    }

    /// A merge Escouade asked for went (the pull request read again, the ticket « Terminé » once
    /// the host says it merged), or did not (said on the card, the wait over).
    async fn ticket_pr_merge_done(
        self: &Arc<Self>,
        id: &str,
        number: u64,
        done: Result<()>,
        client: &dyn Forge,
        repo: &RepoRef,
    ) -> Option<anyhow::Error> {
        match done {
            Ok(()) => {
                let claude = self.lang().claude;
                let news = match client.pull_request(repo, number).await {
                    Ok(pr) => self.edit_ticket(id, |t| {
                        if let Some(tp) = t.pr.as_mut() {
                            tp.merge_when_green = None;
                            tp.merge_note = None;
                        }
                        Ok(settle(t, &pr, None, now_ms(), claude))
                    }),
                    Err(_) => Ok(News::Same),
                };
                if let Ok(News::Merged) = news {
                    self.ticket_pr_merged(id).await;
                } else {
                    // The host merges in a while (a merge queue, a long merge): read soon.
                    self.wake_ticket_prs();
                }
                None
            }
            Err(e) => {
                let label = self.ticket(id).ok().and_then(|t| t.pr).map(|tp| {
                    self.forges
                        .read()
                        .get(&tp.kind)
                        .map_or_else(|| tp.host.clone(), |p| p.name().to_string())
                });
                let why = merge_error_text(i18n::ui(), &label.unwrap_or_default(), &e);
                let _ = self.edit_ticket(id, |t| {
                    if let Some(tp) = t.pr.as_mut() {
                        tp.merge_when_green = None;
                        tp.merge_note = Some(why.clone());
                    }
                    Ok(())
                });
                Some(e)
            }
        }
    }

    /// « Merger » (when_green false) on a ticket « En revue » whose pull request is approved
    /// and green, with the Kanban's strategy when the repository allows it, else one it allows;
    /// « Merger quand c'est vert » (when_green) while its CI runs: the host's auto-merge when it
    /// has one for the repository, else Escouade merges it once it is approved and green, for as
    /// long as it runs. Never by itself: a gesture of the user.
    pub async fn ticket_pr_merge(self: &Arc<Self>, id: &str, when_green: bool) -> Result<MergeHow> {
        let t = self.ticket(id)?;
        let Some(tp) = t.pr.clone().filter(|_| t.column == Column::Pr) else {
            bail!(not_in_review());
        };
        if when_green && !can_wait_green(&tp) {
            bail!(not_waiting(&tp.word, tp.number));
        }
        if !when_green && !merge_ready(&tp) {
            bail!(not_ready(&tp.word, tp.number));
        }
        let board = self.project(&t.project_id)?.board.strategy;
        let (client, _) = self.forge_client(&tp.host).await?;
        let repo = RepoRef {
            namespace: tp.namespace.clone(),
            name: tp.name.clone(),
        };
        let opts = match tp.merge_options.clone() {
            Some(o) => o,
            None => {
                self.merge_options(&*client, &repo, tp.number, &tp.kind)
                    .await?
            }
        };
        let Some(strategy) = pick_strategy(strategy_of(&board), Some(&opts)) else {
            bail!(no_strategy(&tp.word, tp.number));
        };
        self.edit_ticket(id, |t| {
            if let Some(p) = t.pr.as_mut() {
                p.merge_options = Some(opts.clone());
                p.merge_note = None;
            }
            Ok(())
        })?;
        if !when_green {
            let done = client.merge(&repo, tp.number, strategy).await;
            return match self
                .ticket_pr_merge_done(id, tp.number, done, &*client, &repo)
                .await
            {
                Some(e) => Err(e),
                None => Ok(MergeHow::Merged),
            };
        }
        if opts.auto_merge {
            match client.auto_merge(&repo, tp.number, strategy).await {
                Ok(()) => {
                    self.edit_ticket(id, |t| {
                        if let Some(p) = t.pr.as_mut() {
                            p.auto_merge = true;
                        }
                        Ok(())
                    })?;
                    return Ok(MergeHow::Host);
                }
                // The plugin has no auto-merge after all: Escouade waits.
                Err(e) if e.downcast_ref::<Unsupported>().is_some() => {}
                Err(e) => return Err(e),
            }
        }
        self.edit_ticket(id, |t| {
            if let Some(p) = t.pr.as_mut() {
                p.merge_when_green = Some(strategy);
            }
            Ok(())
        })?;
        Ok(MergeHow::Escouade)
    }

    /// « Ne plus attendre »: Escouade no longer merges it once green.
    pub fn ticket_pr_stop_waiting(&self, id: &str) -> Result<()> {
        self.edit_ticket(id, |t| {
            if let Some(p) = t.pr.as_mut() {
                p.merge_when_green = None;
                p.merge_note = None;
            }
            Ok(())
        })
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

impl<R: tauri::Runtime> Core<R> {
    /// « L'agent corrige » on a ticket « En revue » whose CI failed (ESC-60), by hand or by the
    /// autopilot: its agent gets the failed checks and the end of their logs, restored first with
    /// its worktree made again from its branch when it went at the validation. Refused when there
    /// is nothing to fix or a fix is under way; what fails once it began is said on the card too.
    pub async fn ticket_pr_fix_ci(self: &Arc<Self>, id: &str) -> Result<()> {
        let t = self.ticket(id)?;
        let Some(tp) = t.pr.clone().filter(|_| t.column == Column::Pr) else {
            bail!(tr!(
                "Ce ticket n’est pas en revue.",
                "This ticket isn’t in review."
            ));
        };
        if tp.fixing.is_some() {
            bail!(tr!(
                "Son agent corrige déjà la CI.",
                "Its agent is already fixing the CI."
            ));
        }
        let Some(agent_id) = t.agent_id.clone().filter(|a| self.agent(a).is_ok()) else {
            bail!(tr!(
                "Ce ticket n’a plus d’agent pour corriger sa CI.",
                "This ticket no longer has an agent to fix its CI."
            ));
        };
        if !fixable(&t) {
            let (word, n) = (&tp.word, tp.number);
            bail!(tr!(
                "La CI de la {word} #{n} n'a aucun check en échec.",
                "The CI of {word} #{n} has no failed check."
            ));
        }
        if self
            .live_agent(&agent_id)
            .is_some_and(|m| m.status.is_active())
        {
            bail!(tr!(
                "Son agent travaille : attends la fin de son tour.",
                "Its agent is at work: wait for the end of its turn."
            ));
        }
        self.edit_ticket(id, |x| {
            // Checked again under the lock: a second click, or the autopilot meanwhile.
            if !fixable(x) || x.agent_id.as_deref() != Some(&agent_id) {
                bail!(tr!(
                    "Ce ticket a changé entre-temps.",
                    "This ticket changed meanwhile."
                ));
            }
            fix_begins(x, None, now_ms());
            Ok(())
        })?;
        match self.fix_ci_run(id, &t, &tp, &agent_id).await {
            Ok(()) => Ok(()),
            Err(e) => {
                let why = board::first_line(&format!("{e:#}"));
                let _ = self.edit_ticket(id, |x| {
                    if fixing(x) {
                        fix_failed(x, why.clone());
                    }
                    Ok(())
                });
                self.schedule();
                Err(e)
            }
        }
    }

    async fn fix_ci_run(
        self: &Arc<Self>,
        id: &str,
        t: &Ticket,
        tp: &TicketPr,
        agent_id: &str,
    ) -> Result<()> {
        let project = self.project(&t.project_id)?;
        let repo = RepoRef {
            namespace: tp.namespace.clone(),
            name: tp.name.clone(),
        };
        let (client, _) = self.forge_client(&tp.host).await?;
        let checks = client.checks(&repo, tp.number).await?;
        let mut failed = Vec::new();
        for c in checks
            .into_iter()
            .filter(|c| c.state == CheckState::Failure)
        {
            // A CI of another kind keeps no log here: its name and its address alone.
            let log = match &c.job {
                Some(job) => client.job_log(&repo, job).await.ok(),
                None => None,
            };
            failed.push((c.name, c.url, log));
        }
        if failed.is_empty() {
            let (word, n) = (&tp.word, tp.number);
            bail!(tr!(
                "La CI de la {word} #{n} n'a aucun check en échec.",
                "The CI of {word} #{n} has no failed check."
            ));
        }
        // Its branch gone from the project (deleted by hand): made again from its remote.
        if !git::branch_exists(&project.path, &tp.branch).await {
            if let Err(e) = git::fetch_branch(&project.path, &tp.branch).await {
                bail!(tr!(
                    "Branche {b} introuvable, ici comme sur le dépôt distant : {e:#}",
                    "Branch {b} not found, here or on the remote: {e:#}",
                    b = tp.branch
                ));
            }
        }
        let from = git::commit_of(&project.path, &tp.branch).await.ok();
        self.edit_ticket(id, |x| {
            if let Some(p) = x.pr.as_mut().filter(|p| p.fixing.is_some()) {
                p.fix_from = from.clone();
            }
            Ok(())
        })?;
        // Archived at the validation: restored, its worktree made again from its branch.
        let archived = self.agent(agent_id)?.lock().meta.archived;
        if archived {
            self.archive_agent(agent_id, false).await?;
        }
        let wt = self.agent(agent_id)?.lock().meta.worktree.clone();
        if !wt
            .as_ref()
            .is_some_and(|w| std::path::Path::new(&w.path).is_dir())
        {
            bail!(tr!(
                "Worktree non recréé depuis {b}",
                "Worktree not recreated from {b}",
                b = tp.branch
            ));
        }
        let pr = PullRequest {
            number: tp.number,
            url: tp.url.clone(),
            ..Default::default()
        };
        let message =
            crate::agentprs::checks_message(self.lang().claude, &tp.word, &pr, &tp.branch, &failed);
        self.send_message(agent_id, message, Vec::new()).await
    }

    /// The end of the turn of an agent that fixes its ticket's CI (ESC-60), when it is one: what it
    /// committed is pushed (it may have pushed already), the pull request read again, and the
    /// ticket is back « En revue » as before, its agent archived again and its worktree removed
    /// as after the validation. Stopped or failed, the card says why and the agent stays
    /// restored. False when the agent fixes no CI.
    pub(crate) async fn ci_fix_turn_ended(
        self: &Arc<Self>,
        ticket_id: &str,
        agent_id: &str,
        end: &board::TurnEnd,
    ) -> bool {
        let Ok(t) = self.ticket(ticket_id) else {
            return false;
        };
        if !fixing(&t) || t.agent_id.as_deref() != Some(agent_id) {
            return false;
        }
        let why = match fix_end(end) {
            FixEnd::Wait => return true,
            FixEnd::Stopped(why) => Some(why),
            FixEnd::Finished => None,
        };
        let tp = t.pr.clone().unwrap_or_default();
        let wt = self
            .agent(agent_id)
            .ok()
            .and_then(|h| h.lock().meta.worktree.clone());
        let why = match (why, &wt) {
            (Some(why), _) => Some(why),
            (None, None) => Some(not_pushed(i18n::ui(), "worktree")),
            (None, Some(wt)) => self.push_fix(&tp, wt).await.err(),
        };
        let done = why.is_none();
        let _ = self.edit_ticket(ticket_id, |x| {
            if fixing(x) && x.agent_id.as_deref() == Some(agent_id) {
                match &why {
                    Some(why) => fix_failed(x, why.clone()),
                    None => fix_done(x),
                }
            }
            Ok(())
        });
        if done {
            self.wake_ticket_prs();
            self.fix_cleanup(&t, agent_id, wt).await;
        } else {
            self.notify_ticket(ticket_id, false);
        }
        self.schedule();
        true
    }

    /// What its agent did goes to the host: committed (else nothing is pushed), new since the fix
    /// began, and pushed (again: it may have pushed it itself).
    async fn push_fix(&self, tp: &TicketPr, wt: &crate::model::Worktree) -> Result<(), String> {
        let lang = i18n::ui();
        if !git::status(&wt.path)
            .await
            .is_ok_and(|s| s.entries.is_empty())
        {
            return Err(not_committed(lang));
        }
        let now = git::commit_of(&wt.path, "HEAD").await.ok();
        if now.is_some() && now == tp.fix_from {
            return Err(nothing_fixed(lang));
        }
        git::push_branch(&wt.path, &wt.branch)
            .await
            .map(|_| ())
            .map_err(|e| not_pushed(lang, &board::first_line(&format!("{e:#}"))))
    }

    /// Fixed and pushed: its agent archived again, and its worktree removed when the board cleans
    /// up (its branch stays: the pull request's), as after its validation.
    async fn fix_cleanup(
        self: &Arc<Self>,
        t: &Ticket,
        agent_id: &str,
        wt: Option<crate::model::Worktree>,
    ) {
        let Ok(project) = self.project(&t.project_id) else {
            return;
        };
        let (proc, ports) = match self.agent(agent_id) {
            Ok(h) => {
                let rt = h.lock();
                (rt.proc.clone(), rt.meta.port_base)
            }
            Err(_) => return,
        };
        if let Err(e) = self.archive_agent(agent_id, true).await {
            log::warn!("ticket {}: its agent was not archived: {e:#}", t.key);
            return;
        }
        if let (true, Some(wt)) = (project.board.cleanup, wt) {
            self.remove_worktree_of(t, &project, &wt, proc, ports, true)
                .await;
        }
    }
}

/// How a merge asked for went: merged now, by the host's auto-merge, or Escouade waits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MergeHow {
    Merged,
    Host,
    Escouade,
}

/// Why a merge did not go, in words: a token refused, else the host's own words.
pub fn merge_error_text(lang: Lang, name: &str, e: &anyhow::Error) -> String {
    if crate::forge::refused(e) {
        return error_text(lang, name, e);
    }
    let why = format!("{e:#}");
    let why = why.lines().next().unwrap_or("").trim().to_string();
    tr_in!(lang, "Merge refusé : {why}", "Merge refused: {why}")
}

fn not_in_review() -> anyhow::Error {
    anyhow!(tr!(
        "Ce ticket n’attend pas en revue.",
        "This ticket isn’t waiting in review."
    ))
}

fn no_strategy(word: &str, number: u64) -> anyhow::Error {
    anyhow!(tr!(
        "Le dépôt n’autorise aucune façon de merger {word} #{number}.",
        "The repository allows no way to merge {word} #{number}."
    ))
}

fn not_ready(word: &str, number: u64) -> anyhow::Error {
    anyhow!(tr!(
        "{word} #{number} n’est pas encore approuvée et verte.",
        "{word} #{number} isn’t approved and green yet."
    ))
}

fn not_waiting(word: &str, number: u64) -> anyhow::Error {
    anyhow!(tr!(
        "{word} #{number} n’a pas de CI en cours, ou se merge déjà quand elle est verte.",
        "{word} #{number} has no CI running, or already merges once green."
    ))
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
