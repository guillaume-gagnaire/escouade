//! Gitea and Forgejo (Codeberg, gitea.com and the self-hosted instances): an access token with
//! scopes. Their API is the same for what Escouade asks. The PR's calls come with their client
//! (ESC-58), written from their documentation: to try for real later.

use super::*;

pub struct GiteaProvider;

const NAME: &str = "Gitea";

impl Provider for GiteaProvider {
    fn kind(&self) -> String {
        "gitea".into()
    }

    fn name(&self) -> &'static str {
        "Gitea / Forgejo"
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            // Its client comes later (ESC-56 to ESC-58).
            pull_requests: false,
            rerun: false,
            draft: true,
            // `merge_when_checks_succeed`, on a protected branch only.
            auto_merge: true,
            checks: true,
            job_logs: false,
            reviews: true,
            review_threads: true,
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

    fn client(&self, http: reqwest::Client, account: &Credentials) -> Box<dyn Forge> {
        Box::new(Gitea {
            http,
            base: account
                .api
                .clone()
                .unwrap_or_else(|| format!("{}/api/v1", account.url.trim_end_matches('/'))),
            token: account.token.trim().to_string(),
        })
    }
}

pub struct Gitea {
    http: reqwest::Client,
    base: String,
    token: String,
}

impl Forge for Gitea {
    fn me(&self) -> BoxFuture<'_, Result<Identity>> {
        Box::pin(async move {
            let rb = self
                .http
                .get(format!("{}/user", self.base))
                .header("Authorization", format!("token {}", self.token));
            let (me, _) = call(NAME, rb).await?;
            let login = me["login"].as_str().unwrap_or("?").to_string();
            Ok(Identity {
                label: format!("@{login}"),
                user: login,
                scopes: None,
            })
        })
    }
}
