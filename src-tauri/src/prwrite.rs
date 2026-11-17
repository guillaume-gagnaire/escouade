//! What the view of a pull request writes on its host in the user's name (ESC-474, PR-4 of the PR in Escouade): a
//! comment of the conversation, a comment on lines of the diff, an answer in a thread, a thread resolved or opened
//! again; and its control (ESC-488, PR-8): a draft made ready or ready made a draft, reviewers and labels asked and
//! taken back, the branch brought up to date, one check run again, the host's auto-merge turned on or off, and the
//! merge. Each is a click of the user in the window, through its commands (`commands::pr_write`, `commands::pr_merge`)
//! and nothing else: no tool of the MCP server, no agent, nothing by itself. The host's account is the user's.
//!
//! - **By capability**: a write the host does not declare (`Capabilities::writing`, its merge strategies and its
//!   auto-merge) is refused before it is asked.
//! - **Its failure** is told in the window's words: the token refused, without the right to write (`Forbidden`), the
//!   thread or the comment deleted since it was read (`Gone`), a conflict appeared since (`Conflict`), the rules of the
//!   protected branch (`Protected`), a check it requires (`CheckRequired`), the host's limit, the host out of reach.
//! - **Done**, the pull request read whole is read again at the next ask (its kept copy is stale), the window asks it.

use crate::core::Core;
use crate::forge::{
    Candidates, DiffSide, LineComment, MergeOptions, MergeStrategy, PrControl, PrState,
    Unsupported, UpdateMethod, Writing,
};
use crate::integrations::HttpError;
use crate::leftovers::Cleaned;
use crate::model::{Column, Ticket, UiEvent};
use crate::prview::{failure, Can, Failure, FailureKind, Read};
use crate::ticketprs::{options_of_capabilities, pick_strategy, strategy_of};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// The longest text a comment may have: GitHub's (65 536 characters).
pub const BODY_MAX: usize = 65_536;

/// A write the user asked by a click.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PrWrite {
    /// A comment of the conversation, under the description.
    Comment { body: String },
    /// A comment on lines of the diff (`line`, and `start_line` for several), on the base's side for a line removed.
    #[serde(rename_all = "camelCase")]
    LineComment {
        path: String,
        line: u32,
        #[serde(default)]
        start_line: Option<u32>,
        #[serde(default)]
        side: DiffSide,
        body: String,
    },
    /// An answer in a thread, under its first comment (`ThreadComment::id`).
    Reply { comment: String, body: String },
    /// A thread resolved (`ReviewThread::id`).
    Resolve { thread: String },
    /// A resolved thread opened again.
    Reopen { thread: String },
    /// A draft made ready for review (`draft` false), or the pull request made a draft again (ESC-488).
    SetDraft { draft: bool },
    /// Reviews asked of these logins.
    RequestReviewers { logins: Vec<String> },
    /// A review asked taken back.
    RemoveReviewer { login: String },
    /// Labels put on it.
    AddLabels { labels: Vec<String> },
    /// A label taken off.
    RemoveLabel { label: String },
    /// Its branch brought up to date with its base by the host.
    UpdateBranch { method: UpdateMethod },
    /// One check's job run again (`Check::job`).
    RerunCheck { job: String },
    /// The host merges it by itself once what it waits for passes, with this strategy.
    AutoMerge { strategy: MergeStrategy },
    /// The host's auto-merge turned off.
    CancelAutoMerge,
}

impl PrWrite {
    /// Whether the host writes it, as `writing` says (the auto-merge is `Can::auto_merge`'s: `allowed_in`).
    pub fn allowed(&self, w: &Writing) -> bool {
        match self {
            Self::Comment { .. } => w.comment,
            Self::LineComment { .. } => w.line_comment,
            Self::Reply { .. } => w.reply,
            Self::Resolve { .. } => w.resolve,
            Self::Reopen { .. } => w.reopen,
            Self::SetDraft { .. } => w.draft,
            Self::RequestReviewers { .. } | Self::RemoveReviewer { .. } => w.reviewers,
            Self::AddLabels { .. } | Self::RemoveLabel { .. } => w.labels,
            Self::UpdateBranch { method } => match method {
                UpdateMethod::Merge => w.update_merge,
                UpdateMethod::Rebase => w.update_rebase,
            },
            Self::RerunCheck { .. } => w.rerun_check,
            Self::AutoMerge { .. } => true,
            Self::CancelAutoMerge => w.cancel_auto_merge,
        }
    }

    /// Whether the host does it, all its capabilities told.
    pub fn allowed_in(&self, can: &Can) -> bool {
        match self {
            Self::AutoMerge { strategy } => {
                can.auto_merge && can.merge_strategies.contains(strategy)
            }
            other => other.allowed(&can.writing),
        }
    }

    /// Its text, for those that have one.
    fn body(&self) -> Option<&str> {
        match self {
            Self::Comment { body } | Self::LineComment { body, .. } | Self::Reply { body, .. } => {
                Some(body)
            }
            _ => None,
        }
    }

    /// What it lacks to be asked: an empty text, a text too long, a line, a thread, a person or a label unnamed.
    pub fn check(&self) -> Result<()> {
        if let Some(body) = self.body() {
            if body.trim().is_empty() {
                anyhow::bail!(tr!("Le commentaire est vide.", "The comment is empty."));
            }
            if body.chars().count() > BODY_MAX {
                anyhow::bail!(tr!(
                    "Le commentaire dépasse {BODY_MAX} caractères.",
                    "The comment is longer than {BODY_MAX} characters."
                ));
            }
        }
        let blank =
            |names: &[String]| names.is_empty() || names.iter().any(|n| n.trim().is_empty());
        let unnamed = match self {
            Self::LineComment {
                path,
                line,
                start_line,
                ..
            } => path.is_empty() || *line == 0 || start_line.is_some_and(|s| s == 0 || s > *line),
            Self::Reply { comment, .. } => comment.is_empty(),
            Self::Resolve { thread } | Self::Reopen { thread } => thread.is_empty(),
            Self::RequestReviewers { logins } => blank(logins),
            Self::AddLabels { labels } => blank(labels),
            Self::RemoveReviewer { login: name } | Self::RemoveLabel { label: name } => {
                name.trim().is_empty()
            }
            Self::RerunCheck { job } => job.is_empty(),
            Self::Comment { .. }
            | Self::SetDraft { .. }
            | Self::UpdateBranch { .. }
            | Self::AutoMerge { .. }
            | Self::CancelAutoMerge => false,
        };
        if unnamed {
            anyhow::bail!(tr!(
                "Rien à quoi écrire : ligne, fil, personne ou étiquette manquant.",
                "Nothing to write to: a line, a thread, a person or a label is missing."
            ));
        }
        Ok(())
    }
}

/// What the host's refusal of a write is, in the window's words: `failure`'s, with the right to write, a thread gone,
/// a conflict, the rules of a protected branch and a check required told apart.
pub fn write_failure(e: &anyhow::Error, limit: Option<crate::forge::RateLimit>) -> Failure {
    let mut f = failure(e, limit);
    if e.downcast_ref::<Unsupported>().is_some() {
        f.kind = FailureKind::Other;
        return f;
    }
    let status = e.downcast_ref::<HttpError>().map(|h| h.status);
    if f.kind == FailureKind::Other {
        if let Some(kind) = refusal_kind(status, &f.message) {
            f.kind = kind;
        }
    }
    f
}

/// What a host's refusal says by its status and its words (GitHub's 405 of a merge, 409 of a branch moved, 422 of
/// GraphQL): none when it says nothing more than its status.
fn refusal_kind(status: Option<u16>, message: &str) -> Option<FailureKind> {
    let m = message.to_lowercase();
    let says = |words: &[&str]| words.iter().any(|w| m.contains(w));
    if says(&["status check", "required check", "checks have not passed"]) {
        return Some(FailureKind::CheckRequired);
    }
    if says(&[
        "approving review",
        "review is required",
        "reviews are required",
        "protected branch",
        "branch protection",
        "rule violation",
        "base branch policy",
        "not up to date",
    ]) {
        return Some(FailureKind::Protected);
    }
    if status == Some(409) || says(&["conflict", "not mergeable", "was modified"]) {
        return Some(FailureKind::Conflict);
    }
    match status {
        Some(403) => Some(FailureKind::Forbidden),
        Some(404 | 410) => Some(FailureKind::Gone),
        _ => None,
    }
}

/// A failure that is not the host's: the write was refused before it was asked.
fn refused_here<T>(e: anyhow::Error) -> Read<T> {
    Read {
        data: None,
        failure: Some(Failure {
            kind: FailureKind::Other,
            message: format!("{e:#}"),
            reset: None,
            settings: false,
        }),
    }
}

fn not_here() -> anyhow::Error {
    anyhow!(tr!(
        "L'hébergeur ne fait pas cela depuis Escouade.",
        "The host does not do this from Escouade."
    ))
}

/// The merge the user confirmed (ESC-488): its strategy, and what goes with it once merged.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeAsk {
    pub strategy: MergeStrategy,
    /// Its branch deleted on the host.
    #[serde(default)]
    pub delete_branch: bool,
    /// The worktree of the agent that holds its branch removed (the agent archived), as « Nettoyer » removes one.
    #[serde(default)]
    pub delete_worktree: bool,
}

/// What a merge did.
#[derive(Debug, Clone, PartialEq, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Merged {
    /// The host says it merged; not yet when it merges in a while (a merge queue).
    pub merged: bool,
    /// The key of its ticket, now « Terminé ».
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ticket: Option<String>,
    /// The branch deleted on the host.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// Why the branch was not deleted, when it was asked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_kept: Option<String>,
    /// The agent whose worktree went.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worktree: Option<String>,
    /// Why the worktree was kept, when it was asked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worktree_kept: Option<String>,
}

/// What the merge box reads (ESC-488): where the pull request stands for its merge, how the repository lets it merge,
/// the Kanban's strategy among those, and what the host writes.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrControlRead {
    /// None when the host does not read it (`Writing::control`): the box says what the pull request alone says.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub control: Option<PrControl>,
    /// The strategies the repository allows that the host merges with, and whether its auto-merge is there.
    pub options: MergeOptions,
    /// The Kanban's strategy when the repository allows it, else the first it allows.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred: Option<MergeStrategy>,
    pub writing: Writing,
}

impl Core {
    /// The write the user clicked, on the pull request `number` of the project (through whoever holds it, as its view
    /// reads it): done, or why not. Never called but by the window's command.
    pub async fn pr_write(
        self: &Arc<Self>,
        project_id: &str,
        number: u64,
        host_repo: &str,
        write: PrWrite,
    ) -> Read<bool> {
        if let Err(e) = write.check() {
            return refused_here(e);
        }
        let (client, repo, can) = match self.pr_place(project_id, number, host_repo).await {
            Ok(t) => t,
            Err(e) => return Read::failed(&e, None),
        };
        if !write.allowed_in(&can) {
            return refused_here(not_here());
        }
        // A comment on lines goes on the commit whose diff the user read.
        let tip = self.pr_contents_tip(project_id, number, host_repo);
        let done = match &write {
            PrWrite::Comment { body } => client.comment(&repo, number, body).await,
            PrWrite::LineComment {
                path,
                line,
                start_line,
                side,
                body,
            } => {
                let c = LineComment {
                    path: path.clone(),
                    line: *line,
                    start_line: start_line.filter(|s| s < line),
                    side: *side,
                    commit: tip,
                    body: body.clone(),
                };
                client.line_comment(&repo, number, &c).await
            }
            PrWrite::Reply { comment, body } => client.reply(&repo, number, comment, body).await,
            PrWrite::Resolve { thread } => client.resolve(&repo, number, thread).await,
            PrWrite::Reopen { thread } => client.unresolve(&repo, number, thread).await,
            PrWrite::SetDraft { draft } => client.set_draft(&repo, number, *draft).await,
            PrWrite::RequestReviewers { logins } => {
                client.request_reviewers(&repo, number, logins).await
            }
            PrWrite::RemoveReviewer { login } => client.remove_reviewer(&repo, number, login).await,
            PrWrite::AddLabels { labels } => client.add_labels(&repo, number, labels).await,
            PrWrite::RemoveLabel { label } => client.remove_label(&repo, number, label).await,
            PrWrite::UpdateBranch { method } => client.update_branch(&repo, number, *method).await,
            PrWrite::RerunCheck { job } => client.rerun_check(&repo, job).await,
            PrWrite::AutoMerge { strategy } => client.auto_merge(&repo, number, *strategy).await,
            PrWrite::CancelAutoMerge => client.cancel_auto_merge(&repo, number).await,
        };
        match done {
            Ok(()) => {
                self.pr_contents_stale(project_id, number, host_repo);
                self.pr_written(project_id, number);
                Read::ok(true)
            }
            Err(e) => Read {
                data: None,
                failure: Some(write_failure(&e, client.rate_limit())),
            },
        }
    }

    /// Where the pull request stands for its merge and how it may merge, for its merge box (ESC-488). A host that does
    /// not read its control leaves the box to what the pull request says; how it merges is then its provider's.
    pub async fn pr_control(
        self: &Arc<Self>,
        project_id: &str,
        number: u64,
        host_repo: &str,
    ) -> Read<PrControlRead> {
        let (client, repo, can) = match self.pr_place(project_id, number, host_repo).await {
            Ok(t) => t,
            Err(e) => return Read::failed(&e, None),
        };
        let control = match can.writing.control {
            true => match client.control(&repo, number).await {
                Ok(c) => Some(c),
                Err(e) if e.downcast_ref::<Unsupported>().is_some() => None,
                Err(e) => return Read::failed(&e, client.rate_limit()),
            },
            false => None,
        };
        let declared = MergeOptions {
            strategies: can.merge_strategies.clone(),
            auto_merge: can.auto_merge,
        };
        // As the repository's settings say, among what the host merges with; its provider's when not read.
        let mut options = match client.merge_options(&repo, number).await {
            Ok(o) => o,
            Err(_) => options_of_capabilities(&crate::forge::Capabilities {
                merge_strategies: declared.strategies.clone(),
                auto_merge: declared.auto_merge,
                ..Default::default()
            }),
        };
        options
            .strategies
            .retain(|s| declared.strategies.contains(s));
        options.auto_merge &= declared.auto_merge;
        let board = self
            .project(project_id)
            .map(|p| p.board.strategy)
            .unwrap_or_default();
        Read::ok(PrControlRead {
            preferred: pick_strategy(strategy_of(&board), Some(&options)),
            control,
            options,
            writing: can.writing,
        })
    }

    /// Who may be asked for a review and which labels the pull request may take: what the pickers offer (ESC-488).
    pub async fn pr_candidates(
        self: &Arc<Self>,
        project_id: &str,
        number: u64,
        host_repo: &str,
    ) -> Read<Candidates> {
        let (client, repo, can) = match self.pr_place(project_id, number, host_repo).await {
            Ok(t) => t,
            Err(e) => return Read::failed(&e, None),
        };
        if !(can.writing.reviewers || can.writing.labels) {
            return refused_here(not_here());
        }
        match client.candidates(&repo).await {
            Ok(k) => Read::ok(k),
            Err(e) => Read::failed(&e, client.rate_limit()),
        }
    }

    /// The ticket whose pull request it is, in the project.
    fn ticket_of_pr(&self, project_id: &str, number: u64, host_repo: &str) -> Option<Ticket> {
        self.tickets.read().iter().find_map(|t| {
            let tp = t.pr.as_ref().filter(|_| t.project_id == project_id)?;
            let full = format!("{}/{}", tp.namespace, tp.name);
            let here = host_repo.is_empty() || full.eq_ignore_ascii_case(host_repo);
            (tp.number == number && here).then(|| t.clone())
        })
    }

    /// The agent that holds the pull request's branch: the one that reads it, else its ticket's.
    fn holder_of_pr(&self, project_id: &str, number: u64, host_repo: &str) -> Option<String> {
        let reader = self.agents.read().values().find_map(|h| {
            let rt = h.lock();
            let held = rt.meta.project_id == project_id
                && rt
                    .meta
                    .prs
                    .iter()
                    .any(|p| p.number == number && p.host_repo.eq_ignore_ascii_case(host_repo));
            held.then(|| rt.meta.id.clone())
        });
        reader.or_else(|| {
            self.ticket_of_pr(project_id, number, host_repo)
                .and_then(|t| t.agent_id)
        })
    }

    /// « Merger », confirmed by the user in the merge box (ESC-488): merged by the host with the strategy chosen, its
    /// ticket « Terminé » as ESC-62 does it, then its branch deleted on the host and its agent's worktree removed when
    /// asked and once merged. Never called but by the window's command.
    pub async fn pr_merge(
        self: &Arc<Self>,
        project_id: &str,
        number: u64,
        host_repo: &str,
        ask: MergeAsk,
    ) -> Read<Merged> {
        let (client, repo, can) = match self.pr_place(project_id, number, host_repo).await {
            Ok(t) => t,
            Err(e) => return Read::failed(&e, None),
        };
        if !can.merge_strategies.contains(&ask.strategy) {
            return refused_here(not_here());
        }
        let pr = match client.pull_request(&repo, number).await {
            Ok(pr) => pr,
            Err(e) => return Read::failed(&e, client.rate_limit()),
        };
        if pr.state != PrState::Open {
            return refused_here(anyhow!(tr!(
                "La PR #{number} n'est plus ouverte : relis-la.",
                "Pull request #{number} is no longer open: read it again."
            )));
        }
        if let Err(e) = client.merge(&repo, number, ask.strategy).await {
            return Read {
                data: None,
                failure: Some(write_failure(&e, client.rate_limit())),
            };
        }
        self.pr_contents_stale(project_id, number, host_repo);
        let mut out = Merged::default();
        // A ticket's pull request: « Terminé » once the host says it merged, its target fetched (ESC-62).
        if let Some(t) = self
            .ticket_of_pr(project_id, number, host_repo)
            .filter(|t| t.column == Column::Pr)
        {
            let _ = self
                .ticket_pr_merge_done(&t.id, number, Ok(()), &*client, &repo)
                .await;
            if self.ticket(&t.id).is_ok_and(|t| t.column == Column::Done) {
                out.ticket = Some(t.key.clone());
            }
        }
        out.merged = match client.pull_request(&repo, number).await {
            Ok(p) => p.state == PrState::Merged,
            Err(_) => false,
        };
        if ask.delete_branch {
            self.merged_branch(&mut out, &pr, &repo, &*client, &can)
                .await;
        }
        if ask.delete_worktree {
            match (out.merged, self.holder_of_pr(project_id, number, host_repo)) {
                (false, _) => out.worktree_kept = Some(not_merged_yet()),
                (true, Some(id)) => self.merged_worktree(&mut out, &id).await,
                (true, None) => {}
            }
        }
        self.pr_written(project_id, number);
        Read::ok(out)
    }

    /// Written on its host (ESC-488): the following of the pull requests reads it soon (a ticket's, its auto-merge, its
    /// draft), and the window is told it changed: its view and the list « Pull requests » read it again at once.
    fn pr_written(&self, project_id: &str, number: u64) {
        self.wake_ticket_prs();
        self.hub.emit(UiEvent::PrChanged {
            project_id: project_id.to_string(),
            number,
        });
    }

    /// The branch of a pull request just merged, deleted on its host: not a fork's, not before it merged.
    async fn merged_branch(
        &self,
        out: &mut Merged,
        pr: &crate::forge::PullRequest,
        repo: &crate::forge::RepoRef,
        client: &dyn crate::forge::Forge,
        can: &Can,
    ) {
        if !can.writing.delete_branch {
            out.branch_kept = Some(not_here().to_string());
        } else if pr.fork {
            out.branch_kept = Some(tr!(
                "c'est la branche d'un fork : elle n'est pas à toi de la supprimer",
                "it is a fork's branch: not yours to delete"
            ));
        } else if !out.merged {
            out.branch_kept = Some(not_merged_yet());
        } else {
            match client.delete_branch(repo, &pr.head).await {
                Ok(()) => out.branch = Some(pr.head.clone()),
                // Deleted already: the repository deletes a branch once merged.
                Err(e)
                    if e.downcast_ref::<HttpError>()
                        .is_some_and(|h| h.status == 422 || h.status == 404) =>
                {
                    out.branch = Some(pr.head.clone())
                }
                Err(e) => out.branch_kept = Some(format!("{e:#}")),
            }
        }
    }

    /// The worktree of the agent that held the branch, removed as « Nettoyer » removes one: the agent archived, its work
    /// kept when nothing else keeps it.
    async fn merged_worktree(self: &Arc<Self>, out: &mut Merged, id: &str) {
        let Ok(h) = self.agent(id) else { return };
        let (name, has, archived) = {
            let rt = h.lock();
            (
                rt.meta.name.clone(),
                rt.meta.worktree.is_some(),
                rt.meta.archived,
            )
        };
        if !has {
            return;
        }
        if !archived {
            if let Err(e) = self.archive_agent(id, true).await {
                out.worktree_kept = Some(format!("{e:#}"));
                return;
            }
        }
        match self.clean_agent_worktree(id, None, None).await {
            Cleaned::Removed => out.worktree = Some(name),
            Cleaned::Kept(why) => out.worktree_kept = Some(why),
            Cleaned::Nothing => {}
        }
    }
}

fn not_merged_yet() -> String {
    tr!(
        "l'hébergeur ne l'a pas encore mergée (file de merge ?) : rien n'est supprimé",
        "the host has not merged it yet (a merge queue?): nothing is deleted"
    )
}
