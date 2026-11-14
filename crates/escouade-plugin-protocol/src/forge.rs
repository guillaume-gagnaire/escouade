//! The data of the code hosts (docs/PLUGINS-DESIGN.md §4.5): what a host's pull requests, checks, reviews and deployments are,
//! as the core and the plugins that serve a host (`forge/*`) both know them. They travel as JSON, in camelCase; a plugin
//! written in another language writes the same.

use serde::{Deserialize, Serialize};

/// The most open pull requests of a repository a host lists (`forge/openPullRequests`).
pub const OPEN_PRS: usize = 30;

fn is_zero(n: &u64) -> bool {
    *n == 0
}

/// A repository on a code host: its namespace (owner, organization, GitLab group and subgroups,
/// Bitbucket workspace) and its name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoRef {
    pub namespace: String,
    pub name: String,
}

impl RepoRef {
    /// `namespace/name`.
    pub fn full(&self) -> String {
        format!("{}/{}", self.namespace, self.name)
    }

    /// Exactly two segments: an owner and a repository (GitHub, Bitbucket, Gitea).
    pub fn two(segments: &[String]) -> Option<Self> {
        match segments {
            [owner, name] => Some(Self {
                namespace: owner.clone(),
                name: name.clone(),
            }),
            _ => None,
        }
    }

    /// Two segments or more: groups and subgroups, then the project (GitLab).
    pub fn nested(segments: &[String]) -> Option<Self> {
        match segments {
            [groups @ .., name] if !groups.is_empty() => Some(Self {
                namespace: groups.join("/"),
                name: name.clone(),
            }),
            _ => None,
        }
    }
}

/// How a pull request merges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MergeStrategy {
    Merge,
    Squash,
    Rebase,
    /// Rebased onto its base, then merged with a merge commit (Azure DevOps' « semi-linear »).
    SemiLinear,
}

/// How a pull request of a repository may merge, as the repository's settings say (ESC-62): the strategies they allow,
/// and whether the host merges by itself once the checks pass (GitHub's « Allow auto-merge », Gitea's protected
/// branch). Read by `Forge::merge_options`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct MergeOptions {
    pub strategies: Vec<MergeStrategy>,
    pub auto_merge: bool,
}

/// A pull request (a merge request on GitLab) to open.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PullRequestDraft {
    pub title: String,
    pub body: String,
    /// The branch that brings the changes.
    pub head: String,
    /// The branch it merges into.
    pub base: String,
    pub draft: bool,
    /// Who is asked for a review (logins).
    pub reviewers: Vec<String>,
    pub labels: Vec<String>,
    /// The issue the pull request closes once merged: « Closes #n » ends its description.
    pub closes: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum PrState {
    #[default]
    Open,
    Closed,
    Merged,
}

/// A pull request as its host tells it (kept with an agent's data: `AgentMeta::prs`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct PullRequest {
    pub number: u64,
    pub url: String,
    pub title: String,
    pub state: PrState,
    pub draft: bool,
    pub head: String,
    pub base: String,
    /// Conflicts with its base: `Some(false)`; not known yet: none.
    pub mergeable: Option<bool>,
    /// Reviewers whose last say is an approval.
    pub approvals: u32,
    /// A reviewer's last say asks for changes.
    pub changes_requested: bool,
    /// Threads of comments on lines not resolved yet.
    pub open_threads: u32,
    /// The host merges it by itself once the checks pass.
    pub auto_merge: bool,
    /// Who opened it (a login, a name on Azure DevOps); empty when the host does not say.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub author: String,
    /// When it was opened (seconds since 1970); 0 when the host does not say.
    #[serde(skip_serializing_if = "is_zero")]
    pub created: u64,
    /// The checks of its last commit, in one word; none when it has none.
    pub checks: Option<CheckState>,
    /// Its branch is a fork's: another repository's branch that only bears the same name.
    pub fork: bool,
    /// Of an agent of several repositories (ESC-169): the repository it was opened in.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub repo: String,
    /// The repository on the host its branch is in (`ada/demo`), when the host tells it: a
    /// fork's, for one opened from a fork.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub head_repo: String,
    /// The repository on the host it is in (`acme/demo`), when it was read elsewhere than on the
    /// remote of the project's target: on the remote the agent's branch tracks, or on the
    /// repository that one was forked from. Empty: the target's.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub host_repo: String,
    /// Merged: the commit its merge made on its base (the merge commit, the squash commit, or the
    /// last commit rebased), when the host tells it (ESC-67): what says in which environments a
    /// ticket merged by its pull request is deployed.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub merge_commit: String,
    /// Whose review is asked and not given yet (logins; names on Azure DevOps), when the host
    /// says it: the « Relecture demandée » filter of the project's pull requests (ESC-65).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub reviewers: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum CheckState {
    #[default]
    Pending,
    Success,
    Failure,
    Skipped,
}

/// A check or a status of a pull request's last commit (a CI job).
#[derive(Debug, Clone, PartialEq, Serialize, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    pub name: String,
    pub state: CheckState,
    pub url: Option<String>,
    /// What `Forge::job_log` asks for, when the host keeps its log.
    pub job: Option<String>,
    /// When it started (seconds since 1970), when the host says it: a pending one that started
    /// is running, the others wait.
    pub started: Option<u64>,
    /// When it ended, when the host says it.
    pub finished: Option<u64>,
    /// Stopped before its end: a failure to the rest of Escouade, cancelled to the window.
    pub cancelled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReviewState {
    Approved,
    ChangesRequested,
    Commented,
}

/// A comment on a line of a review to publish.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReviewLine {
    pub path: String,
    pub line: u32,
    pub body: String,
}

/// A review to publish on a pull request, as the user validated it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReviewDraft {
    pub state: ReviewState,
    pub body: String,
    pub lines: Vec<ReviewLine>,
}

/// How much of its calls a host leaves, as its answers say.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimit {
    pub limit: u32,
    pub remaining: u32,
    /// When it starts over (seconds since 1970).
    pub reset: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Review {
    pub author: String,
    pub state: ReviewState,
    pub body: String,
    /// When it was given (seconds since 1970); 0 when the host does not say.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub created: u64,
}

/// A comment of a pull request, on one of its lines or not.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewComment {
    pub id: String,
    pub author: String,
    pub body: String,
    pub path: Option<String>,
    pub line: Option<u32>,
    pub resolved: bool,
    /// When it was written (seconds since 1970); 0 when the host does not say.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub created: u64,
    /// The lines of the diff it was written on, as the host keeps them (GitHub's and Gitea's
    /// `diff_hunk`); none when the host does not say.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub excerpt: Option<String>,
}

/// A deployment in one of the repository's environments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Deployment {
    pub environment: String,
    /// Its last state, in the host's word lowered: `success`, `failure`, `error`, `pending`,
    /// `in_progress`, `queued`, `inactive` (a success a newer one replaced)…
    pub state: String,
    /// `deployments`: the address of the environment, else of its job. `environments`: the
    /// environment's address alone (`log` holds the job's).
    pub url: Option<String>,
    /// The commit it deployed (its full id), when the host tells it (ESC-67).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub sha: String,
    /// When it was made (seconds since 1970); 0 when the host does not say.
    #[serde(skip_serializing_if = "is_zero")]
    pub created: u64,
    /// Who made it (a login); empty when the host does not say.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub author: String,
    /// The address of the job that deployed it, to read why it failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log: Option<String>,
}

/// Who an account is on its host, as its token says.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Identity {
    /// The login.
    pub user: String,
    /// As the window shows it: "@ada".
    pub label: String,
    /// The rights the token has, when the host tells them (GitHub's classic tokens, GitLab);
    /// none when it does not (fine-grained tokens, Bitbucket, Gitea).
    pub scopes: Option<Vec<String>>,
}

// A pull request read whole (ESC-452, the PR in Escouade): its description, its commits, its files and its threads, each a
// call of its own (`forge/description`, `forge/commits`, `forge/files`, `forge/threads`) that a host serves or refuses
// (`unsupported`). What they say was written by whoever can open a pull request: the core keeps it as a third party's.

/// The description of a pull request and what changes when it moves (`forge/description`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct PrDescription {
    /// Its text, in the host's Markdown; empty when it has none.
    pub body: String,
    /// The commit at its tip and the one of its base, when the host says them: what tells a push.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub head_sha: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub base_sha: String,
    /// When it last changed (seconds since 1970: a comment, a push, a review); 0 when the host does not say.
    #[serde(skip_serializing_if = "is_zero")]
    pub updated: u64,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,
    /// Its lines added and removed, its files and its commits, as the host counts them; 0 when it does not say.
    #[serde(skip_serializing_if = "is_zero")]
    pub additions: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub deletions: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub changed_files: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub commits: u64,
}

/// A commit of a pull request, the oldest first (`forge/commits`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct PrCommit {
    /// Its full id.
    pub sha: String,
    /// Its whole message: its title is its first line.
    pub message: String,
    /// Who wrote it: the host's login, else the name git keeps.
    pub author: String,
    /// When it was written (seconds since 1970); 0 when the host does not say.
    #[serde(skip_serializing_if = "is_zero")]
    pub created: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// What a pull request does to a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum FileStatus {
    Added,
    #[default]
    Modified,
    Removed,
    Renamed,
    Copied,
}

/// A file a pull request changes, as its host counts it (`forge/files`). Its lines are not here: the core reads the
/// diff from git.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct PrFile {
    pub path: String,
    /// Renamed or copied: where it was.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_path: Option<String>,
    pub status: FileStatus,
    pub additions: u64,
    pub deletions: u64,
}

/// Where a thread stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ThreadState {
    #[default]
    Open,
    Resolved,
    /// Not resolved, but its lines changed since it was written: it speaks of a former version.
    Outdated,
}

/// The side of the diff a thread's lines are on: the base's (`Left`, lines removed) or the tip's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum DiffSide {
    Left,
    #[default]
    Right,
}

/// A comment of a thread: its first, or an answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ThreadComment {
    /// What `Forge::reply` answers under.
    pub id: String,
    pub author: String,
    pub body: String,
    /// When it was written, and last edited (seconds since 1970); 0 when the host does not say.
    #[serde(skip_serializing_if = "is_zero")]
    pub created: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub updated: u64,
}

/// A thread of a pull request (`forge/threads`): on lines of a file, with its answers and its state; or, without a
/// path, a comment of the conversation under the description, which has no line and does not resolve.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ReviewThread {
    /// The host's id of the thread (what resolves it); empty when it has none.
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Its last line in the version it is on, and its first when it holds several; none when the host lost it (an
    /// outdated one), or of the conversation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_line: Option<u32>,
    /// Its line in the version it was written on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_line: Option<u32>,
    pub side: DiffSide,
    pub state: ThreadState,
    /// Its first comment, then its answers, the oldest first.
    pub comments: Vec<ThreadComment>,
    /// The lines of the diff it was written on, as the host keeps them (GitHub's `diffHunk`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excerpt: Option<String>,
}

impl ReviewThread {
    /// On lines of a file, not of the conversation.
    pub fn on_lines(&self) -> bool {
        self.path.is_some()
    }
}

#[cfg(test)]
mod read_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_thread_travels_in_camel_case_and_without_what_it_lacks() {
        let t = ReviewThread {
            id: "T1".into(),
            path: Some("src/a.rs".into()),
            line: Some(4),
            start_line: Some(2),
            state: ThreadState::Outdated,
            comments: vec![ThreadComment {
                id: "1".into(),
                author: "bob".into(),
                body: "Pourquoi ?".into(),
                created: 10,
                updated: 0,
            }],
            ..Default::default()
        };
        let v = serde_json::to_value(&t).unwrap();
        assert_eq!(v["startLine"], 2);
        assert_eq!(v["state"], "outdated");
        assert_eq!(v["side"], "right");
        assert!(v.get("originalLine").is_none());
        assert!(v["comments"][0].get("updated").is_none());
        assert!(t.on_lines());
        // What a plugin of another language writes, with what it leaves out.
        let back: ReviewThread =
            serde_json::from_value(json!({ "id": "", "comments": [], "state": "resolved" }))
                .unwrap();
        assert_eq!(back.state, ThreadState::Resolved);
        assert!(!back.on_lines());
    }

    #[test]
    fn a_description_a_file_and_a_commit_read_with_their_defaults() {
        let d: PrDescription =
            serde_json::from_value(json!({ "body": "x", "headSha": "abc" })).unwrap();
        assert_eq!(
            (d.body.as_str(), d.head_sha.as_str(), d.updated),
            ("x", "abc", 0)
        );
        let f: PrFile = serde_json::from_value(
            json!({ "path": "a", "status": "renamed", "previousPath": "b" }),
        )
        .unwrap();
        assert_eq!(f.status, FileStatus::Renamed);
        assert_eq!(f.previous_path.as_deref(), Some("b"));
        let c: PrCommit =
            serde_json::from_value(json!({ "sha": "1", "message": "feat: x\n\ncorps" })).unwrap();
        assert_eq!(c.author, "");
    }
}
