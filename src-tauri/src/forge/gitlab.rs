//! GitLab: gitlab.com and the self-hosted instances. Its token is a personal one (`api`), or the
//! `glab` CLI's. Its word is « MR ». What the core knows of it is declared here (its hosts, its
//! rights, its CLI): its client is in the plugin « GitLab » (`plugins/gitlab`,
//! ESC-56), which serves its accounts in its module; written from GitLab's documentation,
//! to try on a real instance later (docs/TODO-2.2.md).

use super::*;

pub struct GitlabProvider;

const NAME: &str = "GitLab";

impl Provider for GitlabProvider {
    fn kind(&self) -> String {
        "gitlab".into()
    }
    fn pull_fetch(&self, number: u64) -> Option<PullFetch> {
        Some(PullFetch::Head(format!(
            "refs/merge-requests/{number}/head"
        )))
    }

    fn name(&self) -> &'static str {
        NAME
    }

    fn request_word(&self) -> &'static str {
        "MR"
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            pull_requests: true,
            // A pipeline's failed jobs, in one call.
            rerun: true,
            // `Draft:` before the title.
            draft: true,
            // `merge_when_pipeline_succeeds` (`auto_merge` in later versions).
            auto_merge: true,
            checks: true,
            job_logs: true,
            reviews: true,
            review_threads: true,
            deployments: true,
            merge_strategies: vec![MergeStrategy::Merge, MergeStrategy::Squash],
        }
    }

    fn default_host(&self) -> Option<&'static str> {
        Some("gitlab.com")
    }

    fn own_host(&self, host: &str) -> Option<String> {
        matches!(host, "gitlab.com" | "www.gitlab.com" | "altssh.gitlab.com")
            .then(|| "gitlab.com".into())
    }

    /// Groups, subgroups, then the project.
    fn repo_of(&self, segments: &[String]) -> Option<RepoRef> {
        RepoRef::nested(segments)
    }

    /// `/api/v4/version` answers (401 without a token on recent versions).
    fn probe(&self) -> Option<Probe> {
        Some(Probe {
            path: "/api/v4/version".into(),
            statuses: vec![401],
            key_status: 200,
            key: "version".into(),
        })
    }

    fn scopes(&self) -> Vec<String> {
        ["api"].map(String::from).to_vec()
    }

    fn token_page(&self, url: &str) -> String {
        format!(
            "{}/-/user_settings/personal_access_tokens?name=Escouade&scopes=api",
            url.trim_end_matches('/')
        )
    }

    /// `glab` has no `auth token`: its status prints the token (`✓ Token: glpat-…`) when asked.
    fn cli(&self, host: &str) -> Option<CliToken> {
        Some(CliToken {
            program: "glab".into(),
            args: ["auth", "status", "--show-token", "--hostname", host]
                .map(str::to_string)
                .to_vec(),
            login: "glab auth login".into(),
            read: CliRead::After("Token:".into()),
        })
    }

    fn plugin(&self) -> Option<&'static str> {
        Some(crate::plugins::builtin::GITLAB)
    }

    fn client(&self, _http: reqwest::Client, _account: &Credentials) -> Box<dyn Forge> {
        Box::new(Unserved(NAME))
    }
}

/// The token in what `glab auth status --show-token` printed.
pub fn glab_token(text: &str) -> Option<String> {
    CliRead::After("Token:".into()).token(text, "")
}
