//! « Lancer un agent sur cette PR » (ESC-65), from the project's « Pull requests » view: the pull
//! request's branch fetched from the project's remote, then an agent in a worktree on it, as
//! « Nouvel agent sur une branche… » makes one. A pull request opened from a fork is fetched by the
//! reference its host keeps of it (`refs/pull/<n>/head`, `refs/merge-requests/<n>/head`; on Azure
//! DevOps the head of `refs/pull/<n>/merge`; on Bitbucket Cloud, from the fork itself) into a branch
//! `pr-<n>` (`mr-<n>` on GitLab), which only ever moves forward: a `pr-<n>` already here with
//! commits of its own (an earlier agent's, the user's) is kept as it is, never overwritten. Nothing
//! is pushed nor written on the host: the pull request is read again through the code host's
//! interface (`forge`), its branch through git.

use crate::core::{AgentOptions, Core};
use crate::forge::{PullFetch, PullRequest};
use crate::git;
use crate::model::AgentView;
use anyhow::{anyhow, bail, Result};
use std::sync::Arc;
use tauri::Runtime;

/// How a pull request's branch is fetched, and the branch the agent's worktree is on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchPlan {
    /// What is fetched from: the project's remote's name, or a fork's address.
    pub from: String,
    pub refspec: String,
    /// The branch given to the worktree: a local one, or a remote one (`origin/feat`).
    pub branch: String,
    /// The fetched commit is a merge: the branch is set to its second parent (`FETCH_HEAD^2`).
    pub second_parent: bool,
}

/// The local branch of a pull request opened from a fork: `pr-12`, `mr-12` on GitLab.
pub fn fork_branch(word: &str, number: u64) -> String {
    format!("{}-{number}", word.to_ascii_lowercase())
}

/// How `pr`'s branch comes from the project's remote `remote` (on the instance at `instance`):
/// its own branch through the remote's tracking one (the local one of the same name when there is
/// one, `has_local`), or for a fork's the reference its host says (`how`).
pub fn fetch_plan(
    pr: &PullRequest,
    remote: &str,
    instance: &str,
    how: Option<PullFetch>,
    word: &str,
    has_local: bool,
) -> Result<FetchPlan> {
    if pr.head.is_empty() || pr.head.starts_with('-') {
        bail!(tr!(
            "La {word} #{n} ne dit pas sa branche.",
            "{word} #{n} doesn’t tell its branch.",
            n = pr.number
        ));
    }
    if !pr.fork {
        let head = &pr.head;
        return Ok(FetchPlan {
            from: remote.to_string(),
            refspec: format!("+refs/heads/{head}:refs/remotes/{remote}/{head}"),
            branch: if has_local {
                head.clone()
            } else {
                format!("{remote}/{head}")
            },
            second_parent: false,
        });
    }
    let local = fork_branch(word, pr.number);
    match how {
        Some(PullFetch::Head(r)) => Ok(FetchPlan {
            from: remote.to_string(),
            // Not forced (no `+`): a local branch of that name only moves forward.
            refspec: format!("{r}:refs/heads/{local}"),
            branch: local,
            second_parent: false,
        }),
        Some(PullFetch::Merge(r)) => Ok(FetchPlan {
            from: remote.to_string(),
            refspec: r,
            branch: local,
            second_parent: true,
        }),
        Some(PullFetch::ForkBranch) if !pr.head_repo.is_empty() => Ok(FetchPlan {
            from: format!("{}/{}.git", instance.trim_end_matches('/'), pr.head_repo),
            refspec: format!("refs/heads/{}:refs/heads/{local}", pr.head),
            branch: local,
            second_parent: false,
        }),
        _ => bail!(tr!(
            "Cette {word} vient d'un fork que l'hébergeur ne permet pas de récupérer.",
            "This {word} comes from a fork its host gives no way to fetch."
        )),
    }
}

impl<R: Runtime> Core<R> {
    /// « Lancer un agent sur cette PR »: the open pull request `number` of the project's remote,
    /// read again, its branch fetched, and an agent in a worktree on it, merged into the pull
    /// request's base when that branch is here. The refusals of an agent on a branch hold (the
    /// folder's branch, one another worktree has).
    pub async fn create_agent_on_pr(
        self: &Arc<Self>,
        project_id: &str,
        number: u64,
    ) -> Result<AgentView> {
        let (remote, provider, client) = self.project_host(project_id).await?;
        let word = provider.request_word();
        let pr = client
            .open_pull_requests(&remote.repo)
            .await?
            .into_iter()
            .find(|p| p.number == number)
            .ok_or_else(|| {
                anyhow!(tr!(
                    "La {word} #{number} n'est plus ouverte.",
                    "{word} #{number} isn’t open anymore."
                ))
            })?;
        let project = self.project(project_id)?;
        let root = self
            .toplevel(&project.path)
            .await
            .ok_or_else(crate::core::not_a_repo)?;
        let name = crate::forge::connect::project_remote_name(&project.path, &project.board.target)
            .await
            .ok_or_else(|| anyhow!("no remote"))?;
        let has_local = !pr.fork && git::branch_exists(&root, &pr.head).await;
        let plan = fetch_plan(
            &pr,
            &name,
            &remote.url,
            provider.pull_fetch(number),
            word,
            has_local,
        )?;
        fetch_pr(&root, &plan).await?;
        let base = git::branch_exists(&root, &pr.base)
            .await
            .then(|| pr.base.clone());
        self.create_agent_with(
            project_id,
            AgentOptions {
                select: true,
                branch: Some(plan.branch),
                base,
                ..Default::default()
            },
        )
        .await
    }
}

/// The pull request's branch, fetched as `plan` says. A fork's branch already here only moves
/// forward: one with commits of its own (an earlier agent's, the user's) is kept as it is (git
/// refuses to fetch over it, and its merge's second parent is not set over it), and so is one a
/// worktree holds; the agent on that one is then refused as for any branch held.
pub async fn fetch_pr(root: &str, plan: &FetchPlan) -> Result<()> {
    let exists = git::branch_exists(root, &plan.branch).await;
    if let Err(e) = git::fetch_ref(root, &plan.from, &plan.refspec).await {
        let into_branch = plan
            .refspec
            .ends_with(&format!(":refs/heads/{}", plan.branch));
        if into_branch && exists {
            return Ok(());
        }
        return Err(e);
    }
    if plan.second_parent {
        let local = format!("refs/heads/{}", plan.branch);
        let forward = !exists
            || git::run(
                root,
                &["merge-base", "--is-ancestor", &local, "FETCH_HEAD^2"],
            )
            .await
            .is_ok();
        if forward {
            if let Err(e) = git::run(root, &["branch", "-f", &plan.branch, "FETCH_HEAD^2"]).await {
                if !exists {
                    return Err(e);
                }
            }
        }
    }
    Ok(())
}
