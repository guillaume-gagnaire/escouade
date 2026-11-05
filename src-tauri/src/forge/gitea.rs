//! Gitea and Forgejo (Codeberg, gitea.com and the self-hosted instances): an access token with
//! scopes. What the core knows of them is declared here (their hosts, their rights, what they can
//! do): their client is in the plugin « Gitea / Forgejo » (`plugins/gitea`,
//! INT-8.4, ESC-58), which serves their accounts in a process of its own.

use super::*;

pub struct GiteaProvider;

const NAME: &str = "Gitea";

impl Provider for GiteaProvider {
    fn kind(&self) -> String {
        "gitea".into()
    }
    fn pull_fetch(&self, number: u64) -> Option<PullFetch> {
        Some(PullFetch::Head(format!("refs/pull/{number}/head")))
    }

    fn name(&self) -> &'static str {
        "Gitea / Forgejo"
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            pull_requests: true,
            // Gitea's Actions only, from its 1.26; the plugin says `Unsupported` to an instance
            // that cannot (and to every Forgejo).
            rerun: true,
            // A title that starts with `WIP:`.
            draft: true,
            // `merge_when_checks_succeed`, on a protected branch only.
            auto_merge: true,
            checks: true,
            // From Gitea 1.25 and Forgejo 12: the plugin says `Unsupported` before.
            job_logs: true,
            reviews: true,
            review_threads: true,
            // Neither has an API for deployments: absent.
            deployments: false,
            merge_strategies: vec![
                MergeStrategy::Merge,
                MergeStrategy::Squash,
                MergeStrategy::Rebase,
            ],
        }
    }

    fn default_host(&self) -> Option<&'static str> {
        None
    }

    fn own_host(&self, host: &str) -> Option<String> {
        matches!(host, "codeberg.org" | "gitea.com").then(|| host.to_string())
    }

    /// `/api/v1/version` answers without a token, with the version.
    fn probe(&self) -> Option<Probe> {
        Some(Probe {
            path: "/api/v1/version".into(),
            key_status: 200,
            key: "version".into(),
            ..Default::default()
        })
    }

    fn scopes(&self) -> Vec<String> {
        ["write:repository", "write:issue", "read:user"]
            .map(String::from)
            .to_vec()
    }

    fn token_page(&self, url: &str) -> String {
        format!("{}/user/settings/applications", url.trim_end_matches('/'))
    }

    fn plugin(&self) -> Option<&'static str> {
        Some(crate::plugins::builtin::GITEA)
    }

    fn client(&self, _http: reqwest::Client, _account: &Credentials) -> Box<dyn Forge> {
        Box::new(Unserved(NAME))
    }
}
