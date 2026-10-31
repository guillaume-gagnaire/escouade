//! Bitbucket Cloud (bitbucket.org): an Atlassian API token with scopes, and the e-mail of its
//! account. Bitbucket Data Center, whose API differs, waits for another version. The PR's calls
//! come with its client (ESC-57), written from its documentation: to try for real later.

use super::*;

pub struct BitbucketProvider;

const NAME: &str = "Bitbucket";

/// Where its API is: `ESCOUADE_BITBUCKET_API` (a fake one) or api.bitbucket.org for bitbucket.org,
/// `/2.0` of any other address (a fake one in the tests).
pub fn api_base(url: &str) -> String {
    if host_of(url) == "bitbucket.org" {
        return std::env::var("ESCOUADE_BITBUCKET_API")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| "https://api.bitbucket.org/2.0".into())
            .trim_end_matches('/')
            .to_string();
    }
    format!("{}/2.0", url.trim_end_matches('/'))
}

impl Provider for BitbucketProvider {
    fn kind(&self) -> String {
        "bitbucket".into()
    }

    fn name(&self) -> &'static str {
        NAME
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            // Its client comes later (ESC-56 to ESC-58).
            pull_requests: false,
            rerun: false,
            draft: true,
            // No call merges once the checks pass (BCLOUD-22055, BCLOUD-22062).
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

    fn client(&self, http: reqwest::Client, account: &Credentials) -> Box<dyn Forge> {
        Box::new(Bitbucket {
            http,
            base: account
                .api
                .clone()
                .unwrap_or_else(|| api_base(&account.url)),
            email: account.email.trim().to_string(),
            token: account.token.trim().to_string(),
        })
    }
}

pub struct Bitbucket {
    http: reqwest::Client,
    base: String,
    email: String,
    token: String,
}

impl Forge for Bitbucket {
    fn me(&self) -> BoxFuture<'_, Result<Identity>> {
        Box::pin(async move {
            let rb = self
                .http
                .get(format!("{}/user", self.base))
                .basic_auth(&self.email, Some(&self.token));
            let (me, _) = call(NAME, rb).await?;
            let user = me["account_id"]
                .as_str()
                .or(me["uuid"].as_str())
                .unwrap_or("?")
                .to_string();
            let name = me["nickname"]
                .as_str()
                .or(me["display_name"].as_str())
                .unwrap_or(&self.email);
            Ok(Identity {
                label: format!("@{name}"),
                user,
                scopes: None,
            })
        })
    }
}
