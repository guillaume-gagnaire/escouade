//! GitHub: github.com, GHE.com (`<name>.ghe.com`) and GitHub Enterprise Server. Its token is the
//! `gh` CLI's, else one the user makes. What the core knows of it is declared here (its hosts, its
//! rights, its CLI): its client is in the plugin « GitHub » (`plugins/github`,
//! INT-8.1), which serves its accounts in a process of its own.

use super::*;

pub struct GithubProvider;

const NAME: &str = "GitHub";

/// Where an instance's API is: `ESCOUADE_GITHUB_API` (a fake one) or api.github.com for
/// github.com, `api.<name>.ghe.com` for GHE.com, `/api/v3` of a GitHub Enterprise Server.
pub fn api_base(url: &str) -> String {
    let host = host_of(url);
    if host == "github.com" {
        return crate::integrations::Bases::from_env().github;
    }
    if host.ends_with(".ghe.com") {
        return format!("https://api.{host}");
    }
    format!("{}/api/v3", url.trim_end_matches('/'))
}

impl Provider for GithubProvider {
    fn kind(&self) -> String {
        "github".into()
    }
    fn pull_fetch(&self, number: u64) -> Option<PullFetch> {
        Some(PullFetch::Head(format!("refs/pull/{number}/head")))
    }

    fn name(&self) -> &'static str {
        NAME
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            pull_requests: true,
            draft: true,
            auto_merge: true,
            checks: true,
            job_logs: true,
            reviews: true,
            review_threads: true,
            deployments: true,
            rerun: true,
            merge_strategies: vec![
                MergeStrategy::Merge,
                MergeStrategy::Squash,
                MergeStrategy::Rebase,
            ],
        }
    }

    fn default_host(&self) -> Option<&'static str> {
        Some("github.com")
    }

    fn own_host(&self, host: &str) -> Option<String> {
        match host {
            "github.com" | "www.github.com" | "ssh.github.com" => Some("github.com".into()),
            h if h.ends_with(".ghe.com") => Some(h.strip_prefix("ssh.").unwrap_or(h).into()),
            _ => None,
        }
    }

    /// A classic token's: the PR, its checks, its Actions' logs and the deployments. A
    /// fine-grained one does not say its rights (pull requests and contents in writing; checks,
    /// statuses, Actions and deployments in reading).
    fn scopes(&self) -> Vec<String> {
        ["repo"].map(String::from).to_vec()
    }

    fn token_page(&self, url: &str) -> String {
        let host = host_of(url);
        if host == "github.com" || host.ends_with(".ghe.com") {
            format!("https://{host}/settings/personal-access-tokens/new")
        } else {
            format!("{}/settings/tokens/new", url.trim_end_matches('/'))
        }
    }

    fn cli(&self, host: &str) -> Option<CliToken> {
        Some(CliToken {
            program: "gh".into(),
            args: ["auth", "token", "--hostname", host]
                .map(str::to_string)
                .to_vec(),
            login: "gh auth login".into(),
            read: CliRead::Stdout,
        })
    }

    fn plugin(&self) -> Option<&'static str> {
        Some(crate::plugins::builtin::GITHUB)
    }

    fn client(&self, _http: reqwest::Client, _account: &Credentials) -> Box<dyn Forge> {
        Box::new(Unserved(NAME))
    }
}
