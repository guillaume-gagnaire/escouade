//! What the view of a pull request writes on its host in the user's name (ESC-474, PR-4 of the PR in Escouade): a
//! comment of the conversation, a comment on lines of the diff, an answer in a thread, a thread resolved or opened
//! again. Each is a click of the user in the window, through its command (`commands::pr_write`) and nothing else: no
//! tool of the MCP server, no agent, nothing by itself. The host's account is the user's.
//!
//! - **By capability**: a write the host does not declare (`Capabilities::writing`) is refused before it is asked.
//! - **Its failure** is told in the window's words: the token refused, without the right to write (`Forbidden`), the
//!   thread or the comment deleted since it was read (`Gone`), the host's limit, the host out of reach.
//! - **Done**, the pull request read whole is read again at the next ask (its kept copy is stale), the window asks it.

use crate::core::Core;
use crate::forge::{DiffSide, LineComment, Unsupported, Writing};
use crate::integrations::HttpError;
use crate::prview::{failure, Failure, FailureKind, Read};
use anyhow::{anyhow, Result};
use serde::Deserialize;
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
}

impl PrWrite {
    /// Whether the host writes it.
    pub fn allowed(&self, w: &Writing) -> bool {
        match self {
            Self::Comment { .. } => w.comment,
            Self::LineComment { .. } => w.line_comment,
            Self::Reply { .. } => w.reply,
            Self::Resolve { .. } => w.resolve,
            Self::Reopen { .. } => w.reopen,
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

    /// What it lacks to be asked: an empty text, a text too long, a line or a thread unnamed.
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
        let unnamed = match self {
            Self::LineComment {
                path,
                line,
                start_line,
                ..
            } => path.is_empty() || *line == 0 || start_line.is_some_and(|s| s == 0 || s > *line),
            Self::Reply { comment, .. } => comment.is_empty(),
            Self::Resolve { thread } | Self::Reopen { thread } => thread.is_empty(),
            Self::Comment { .. } => false,
        };
        if unnamed {
            anyhow::bail!(tr!(
                "Rien à quoi écrire : ligne ou fil manquant.",
                "Nothing to write to: a line or a thread is missing."
            ));
        }
        Ok(())
    }
}

/// What the host's refusal of a write is, in the window's words: `failure`'s, with the right to write and a thread
/// gone told apart.
pub fn write_failure(e: &anyhow::Error, limit: Option<crate::forge::RateLimit>) -> Failure {
    let mut f = failure(e, limit);
    if e.downcast_ref::<Unsupported>().is_some() {
        f.kind = FailureKind::Other;
        return f;
    }
    if f.kind == FailureKind::Other {
        match e.downcast_ref::<HttpError>().map(|h| h.status) {
            Some(403) => f.kind = FailureKind::Forbidden,
            Some(404 | 410) => f.kind = FailureKind::Gone,
            _ => {}
        }
    }
    f
}

/// A failure that is not the host's: the write was refused before it was asked.
fn refused_here(e: anyhow::Error) -> Read<bool> {
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
        if !write.allowed(&can.writing) {
            return refused_here(anyhow!(tr!(
                "L'hébergeur ne fait pas cela depuis Escouade.",
                "The host does not do this from Escouade."
            )));
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
        };
        match done {
            Ok(()) => {
                self.pr_contents_stale(project_id, number, host_repo);
                Read::ok(true)
            }
            Err(e) => Read {
                data: None,
                failure: Some(write_failure(&e, client.rate_limit())),
            },
        }
    }
}
