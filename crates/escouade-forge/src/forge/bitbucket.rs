//! Bitbucket Cloud (bitbucket.org): an Atlassian API token with scopes, and the e-mail of its
//! account. Bitbucket Data Center, whose API differs, waits for another version. What the core knows of it is
//! declared here (its host, its rights, its capabilities: no auto-merge, its API has none): its client is in the
//! plugin « Bitbucket » (`plugins/bitbucket`, ESC-57), written from its documentation and
//! tried on a fake server only: to try for real later.

use super::*;

pub struct BitbucketProvider;

const NAME: &str = "Bitbucket";

impl Provider for BitbucketProvider {
    fn kind(&self) -> String {
        "bitbucket".into()
    }
    fn pull_fetch(&self, _number: u64) -> Option<PullFetch> {
        Some(PullFetch::ForkBranch)
    }

    fn name(&self) -> &'static str {
        NAME
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            pull_requests: true,
            rerun: false,
            draft: true,
            // No call merges once the checks pass (BCLOUD-22055, BCLOUD-22062): Escouade waits for the checks and
            // merges itself.
            auto_merge: false,
            checks: true,
            job_logs: true,
            reviews: true,
            review_threads: true,
            deployments: true,
            merge_strategies: vec![
                MergeStrategy::Merge,
                MergeStrategy::Squash,
                MergeStrategy::Rebase,
                MergeStrategy::SemiLinear,
            ],
        }
    }

    fn default_host(&self) -> Option<&'static str> {
        Some("bitbucket.org")
    }

    fn own_host(&self, host: &str) -> Option<String> {
        matches!(
            host,
            "bitbucket.org" | "www.bitbucket.org" | "altssh.bitbucket.org"
        )
        .then(|| "bitbucket.org".into())
    }

    fn needs_email(&self) -> bool {
        true
    }

    fn scopes(&self) -> Vec<String> {
        [
            "read:user:bitbucket",
            "read:repository:bitbucket",
            "read:pullrequest:bitbucket",
            "write:pullrequest:bitbucket",
            "read:pipeline:bitbucket",
        ]
        .map(String::from)
        .to_vec()
    }

    fn token_page(&self, _url: &str) -> String {
        "https://id.atlassian.com/manage-profile/security/api-tokens".into()
    }

    fn plugin(&self) -> Option<&'static str> {
        Some(crate::plugins::builtin::BITBUCKET)
    }

    fn client(&self, _http: reqwest::Client, _account: &Credentials) -> Box<dyn Forge> {
        Box::new(Unserved(NAME))
    }
}
