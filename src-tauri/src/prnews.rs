//! What the readings of the pull requests and the deployments tell the user (ESC-68): the CI that
//! fails, changes asked, an approval, a merge, a conflict, a review asked of the user. Each has its
//! own choice in the settings (`NotifyFor`). None of it asks the host anything: a notification is
//! the difference between two readings already made, by the follow of the tickets (`ticketprs`),
//! the reading of an agent shown (`agentprs`) and the list of the project's pull requests
//! (`prview`). What was already so at the first reading is no news. A text says a number, never a
//! title: a title is a third party's words, and a notification is read out of the app.

use crate::agent::NotifyKind;
use crate::core::Core;
use crate::forge::{CheckState, PrState, PullRequest, RateLimit};
use crate::i18n::{self, Lang};
use crate::model::{AgentMeta, TicketPr, UiEvent};
use crate::prview::ProjectPrs;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;
use tauri::Runtime;

/// What a reading of a pull request says that may be news.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Facts {
    pub red: bool,
    pub changes: bool,
    pub approved: bool,
    pub conflict: bool,
    pub merged: bool,
}

impl Facts {
    fn read(
        state: PrState,
        checks: Option<CheckState>,
        changes: bool,
        approvals: u32,
        mergeable: Option<bool>,
    ) -> Self {
        let open = state == PrState::Open;
        Self {
            red: open && checks == Some(CheckState::Failure),
            changes: open && changes,
            approved: open && approvals > 0 && !changes,
            conflict: open && mergeable == Some(false),
            merged: state == PrState::Merged,
        }
    }

    pub fn of_pr(p: &PullRequest) -> Self {
        Self::read(
            p.state,
            p.checks,
            p.changes_requested,
            p.approvals,
            p.mergeable,
        )
    }

    pub fn of_ticket(tp: &TicketPr) -> Self {
        Self::read(
            tp.state,
            tp.checks,
            tp.changes_requested,
            tp.approvals,
            tp.mergeable,
        )
    }
}

/// What came between two readings of a pull request, the most pressing first. Once merged, the
/// merge alone: what it says of the rest no longer matters.
pub fn news(before: &Facts, after: &Facts) -> Vec<NotifyKind> {
    if after.merged {
        return match before.merged {
            true => Vec::new(),
            false => vec![NotifyKind::PrMerged],
        };
    }
    [
        (before.conflict, after.conflict, NotifyKind::PrConflict),
        (before.red, after.red, NotifyKind::PrCiFailed),
        (before.changes, after.changes, NotifyKind::PrChanges),
        (before.approved, after.approved, NotifyKind::PrApproved),
    ]
    .into_iter()
    .filter(|(was, is, _)| *is && !*was)
    .map(|(_, _, kind)| kind)
    .collect()
}

/// What a notification says: the ticket's key when it has one, the host's word and the number,
/// then what came. Empty for a kind that is no pull request's news.
pub fn text(lang: Lang, kind: NotifyKind, word: &str, number: u64, key: Option<&str>) -> String {
    let what = match kind {
        NotifyKind::PrCiFailed => tr_in!(lang, "CI en échec", "CI failed"),
        NotifyKind::PrChanges => tr_in!(lang, "changements demandés", "changes requested"),
        NotifyKind::PrApproved => tr_in!(lang, "approuvée", "approved"),
        NotifyKind::PrMerged => tr_in!(lang, "mergée", "merged"),
        NotifyKind::PrConflict => {
            tr_in!(lang, "conflit avec sa cible", "conflicts with its target")
        }
        NotifyKind::PrReview => tr_in!(lang, "relecture demandée", "review requested"),
        _ => return String::new(),
    };
    let pr = format!("{word} #{number}");
    match key {
        Some(key) => tr_in!(lang, "{key} · {pr} : {what}", "{key} · {pr}: {what}"),
        None => tr_in!(lang, "{pr} : {what}", "{pr}: {what}"),
    }
}

/// `a` and `b` are the same account: a host's logins do not mind their case.
fn same(a: &str, b: &str) -> bool {
    !a.is_empty() && a.to_lowercase() == b.to_lowercase()
}

/// The pull requests whose review is asked of `me` and not given yet, not its own: the
/// « Relecture demandée » filter of the view (`prlist.ts`).
pub fn asked_of<'a>(prs: &'a [PullRequest], me: &str) -> Vec<&'a PullRequest> {
    prs.iter()
        .filter(|p| p.state == PrState::Open && p.reviewers.iter().any(|r| same(r, me)))
        .filter(|p| !same(&p.author, me))
        .collect()
}

/// The host's limit of calls is nearly reached (a tenth left, and not started over yet): a reading
/// that can wait, waits.
pub fn limit_near(limit: Option<RateLimit>, now_s: u64) -> bool {
    limit.is_some_and(|l| l.limit > 0 && l.remaining * 10 < l.limit && l.reset > now_s)
}

/// The list of a project's open pull requests as last read, which the overview and the tools take
/// again rather than ask the host.
#[derive(Default)]
pub(crate) struct PrList {
    /// When it was last read well (the clock's, and in ms since 1970), and what it said.
    pub(crate) read: Option<(Instant, i64, ProjectPrs)>,
    /// The host's limit of calls, as it last said it.
    pub(crate) limit: Option<RateLimit>,
    /// The pull requests whose review was asked of the user at its last reading; none before the
    /// first one, which tells nothing.
    asked: Option<HashSet<u64>>,
}

impl PrList {
    /// A reading of the list: the pull requests whose review is newly asked of the user.
    fn read_well(&mut self, now: i64, read: &ProjectPrs) -> Vec<u64> {
        let asked: Vec<u64> = asked_of(&read.prs, &read.me)
            .iter()
            .map(|p| p.number)
            .collect();
        let new = match &self.asked {
            None => Vec::new(),
            Some(before) => asked
                .iter()
                .copied()
                .filter(|n| !before.contains(n))
                .collect(),
        };
        self.asked = Some(asked.into_iter().collect());
        self.read = Some((Instant::now(), now, read.clone()));
        new
    }
}

impl<R: Runtime> Core<R> {
    /// The news of a ticket's pull request: its project's board shows it.
    pub(crate) fn tell_ticket_pr(self: &Arc<Self>, id: &str, kinds: &[NotifyKind]) {
        let Ok(t) = self.ticket(id) else { return };
        let Some(tp) = &t.pr else { return };
        let project = self
            .project(&t.project_id)
            .map(|p| p.name)
            .unwrap_or_default();
        for kind in kinds {
            self.alert(
                *kind,
                project.clone(),
                text(i18n::ui(), *kind, &tp.word, tp.number, Some(&t.key)),
                UiEvent::FocusBoard {
                    project_id: t.project_id.clone(),
                },
            );
        }
    }

    /// A ticket holds this pull request: its follow tells of it, not its agent's reading.
    fn ticket_holds(&self, project_id: &str, pr: &PullRequest) -> bool {
        self.tickets.read().iter().any(|t| {
            t.project_id == project_id
                && t.pr.as_ref().is_some_and(|tp| {
                    tp.number == pr.number
                        && (pr.host_repo.is_empty()
                            || pr
                                .host_repo
                                .eq_ignore_ascii_case(&format!("{}/{}", tp.namespace, tp.name)))
                })
        })
    }

    /// The news between two readings of an agent's pull requests: its conversation shows them.
    /// One of another branch than before is no news.
    pub(crate) fn tell_agent_prs(
        self: &Arc<Self>,
        meta: &AgentMeta,
        before: &[PullRequest],
        after: &[PullRequest],
    ) {
        let word = meta.pr_word.as_deref().unwrap_or("PR");
        for pr in after {
            let Some(was) = before.iter().find(|p| {
                p.number == pr.number
                    && p.repo == pr.repo
                    && p.host_repo.eq_ignore_ascii_case(&pr.host_repo)
            }) else {
                continue;
            };
            let kinds = news(&Facts::of_pr(was), &Facts::of_pr(pr));
            if kinds.is_empty() || self.ticket_holds(&meta.project_id, pr) {
                continue;
            }
            let project = self
                .project(&meta.project_id)
                .map(|p| p.name)
                .unwrap_or_default();
            for kind in kinds {
                self.alert(
                    kind,
                    format!("{project} · {}", meta.name),
                    text(i18n::ui(), kind, word, pr.number, None),
                    UiEvent::Focus {
                        project_id: meta.project_id.clone(),
                        agent_id: Some(meta.id.clone()),
                    },
                );
            }
        }
    }

    /// A reading of the project's list is kept, and a review newly asked of the user is told.
    pub(crate) fn pr_list_read(
        self: &Arc<Self>,
        project_id: &str,
        read: &ProjectPrs,
        limit: Option<RateLimit>,
        now: i64,
    ) {
        let new = {
            let mut lists = self.pr_lists.lock();
            let list = lists.entry(project_id.to_string()).or_default();
            list.limit = limit;
            list.read_well(now, read)
        };
        let project = self.project(project_id).map(|p| p.name).unwrap_or_default();
        for n in new {
            self.alert(
                NotifyKind::PrReview,
                project.clone(),
                text(i18n::ui(), NotifyKind::PrReview, &read.word, n, None),
                UiEvent::FocusPrs {
                    project_id: project_id.to_string(),
                },
            );
        }
    }
}

#[cfg(test)]
#[path = "prnews_tests.rs"]
mod tests;
