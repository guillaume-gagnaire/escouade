//! Azure DevOps: dev.azure.com (and the former `<org>.visualstudio.com`) and Azure DevOps Server
//! (`https://server/tfs/<collection>`). An account is an organization's (a Server's collection):
//! a personal access token (sent in Basic, `:token`), or the `az` CLI's (an Entra ID token, in
//! Bearer). Its repositories are an organization, a project and a repository: `RepoRef` keeps
//! `org/project` as its namespace. The pull requests' calls are in `azure_api`.

use super::azure_api::Azure;
use super::*;

pub struct AzureProvider;

pub(crate) const NAME: &str = "Azure DevOps";

/// Its public instance.
pub const CLOUD: &str = "dev.azure.com";

/// Azure DevOps' resource, for an Entra ID token of the `az` CLI.
pub const RESOURCE: &str = "499b84ac-1321-427f-aa17-267ca6975798";

/// Where dev.azure.com's API is, when `ESCOUADE_AZURE_DEVOPS_API` says (a fake one): in place of
/// `https://dev.azure.com` (and of its profiles' and identities' hosts).
pub fn api_override(url: &str) -> Option<String> {
    if host_of(url) != CLOUD {
        return None;
    }
    std::env::var("ESCOUADE_AZURE_DEVOPS_API")
        .ok()
        .map(|v| v.trim().trim_end_matches('/').to_string())
        .filter(|v| !v.is_empty())
}

/// A remote's path segment, its percent-encoding (`My%20Project`) undone.
pub(crate) fn decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if let Some(b) = s
                .get(i + 1..i + 3)
                .and_then(|h| u8::from_str_radix(h, 16).ok())
            {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

/// The organization of a former `<org>.visualstudio.com` host.
fn vs_org(host: &str) -> Option<&str> {
    host.strip_suffix(".visualstudio.com")
        .filter(|o| !o.is_empty() && !o.contains('.') && *o != "vs-ssh")
}

/// The organization (or collection) and the project of a repository.
pub fn parts(repo: &RepoRef) -> (&str, &str) {
    repo.namespace
        .split_once('/')
        .unwrap_or((repo.namespace.as_str(), repo.name.as_str()))
}

fn repo(org: &str, project: &str, name: &str) -> Option<RepoRef> {
    (!org.is_empty() && !project.is_empty() && !name.is_empty()).then(|| RepoRef {
        namespace: format!("{org}/{project}"),
        name: name.to_string(),
    })
}

impl Provider for AzureProvider {
    fn kind(&self) -> String {
        "azure".into()
    }

    fn name(&self) -> &'static str {
        NAME
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            pull_requests: true,
            // `isDraft`.
            draft: true,
            // Azure's auto-complete.
            auto_merge: true,
            // The build and status policies' evaluations, the statuses and Azure Pipelines' builds.
            checks: true,
            job_logs: true,
            // The reviewers' votes.
            reviews: true,
            review_threads: true,
            // Azure Pipelines' environments say nothing simple of a branch: none.
            deployments: false,
            rerun: true,
            merge_strategies: vec![
                MergeStrategy::Merge,
                MergeStrategy::Squash,
                MergeStrategy::Rebase,
                MergeStrategy::SemiLinear,
            ],
        }
    }

    fn default_host(&self) -> Option<&'static str> {
        Some(CLOUD)
    }

    fn address_hint(&self) -> Option<&'static str> {
        Some("dev.azure.com/acme")
    }

    /// dev.azure.com, its ssh (`ssh.dev.azure.com`), and the former `<org>.visualstudio.com` with
    /// theirs (`vs-ssh.visualstudio.com`): all one instance, dev.azure.com.
    fn own_host(&self, host: &str) -> Option<String> {
        match host {
            CLOUD | "ssh.dev.azure.com" | "vs-ssh.visualstudio.com" => Some(CLOUD.into()),
            h if vs_org(h).is_some() => Some(CLOUD.into()),
            _ => None,
        }
    }

    fn repo_of(&self, segments: &[String]) -> Option<RepoRef> {
        self.repo_in("", segments)
    }

    /// `<org>/<project>/_git/<repo>` (`<org>/_git/<repo>` when the project bears the repository's
    /// name), `[DefaultCollection/]<project>/_git/<repo>` on `<org>.visualstudio.com`,
    /// `[tfs/]<collection>/<project>/_git/<repo>` on a Server, and ssh's `v3/<org>/<project>/<repo>`.
    fn repo_in(&self, host: &str, segments: &[String]) -> Option<RepoRef> {
        let segs: Vec<String> = segments.iter().map(|s| decode(s)).collect();
        if segs.first().map(String::as_str) == Some("v3") {
            return match &segs[1..] {
                [org, project, name] => repo(org, project, name),
                _ => None,
            };
        }
        let git = segs.iter().position(|s| s.eq_ignore_ascii_case("_git"))?;
        let name = segs.get(git + 1).filter(|_| segs.len() == git + 2)?;
        let before = &segs[..git];
        if let Some(org) = vs_org(host) {
            let rest: Vec<&String> = before
                .iter()
                .filter(|s| !s.eq_ignore_ascii_case("DefaultCollection"))
                .collect();
            return match rest.as_slice() {
                [] => repo(org, name, name),
                [project] => repo(org, project, name),
                _ => None,
            };
        }
        match before {
            [org] => repo(org, name, name),
            [.., org, project] => repo(org, project, name),
            [] => None,
        }
    }

    /// An organization of dev.azure.com is an instance of its own: `https://dev.azure.com/<org>`.
    fn instance_of(&self, url: String, repo: &RepoRef) -> String {
        if host_of(&url) == CLOUD {
            format!("https://{CLOUD}/{}", parts(repo).0)
        } else {
            url
        }
    }

    fn instance(&self, url: String) -> String {
        match vs_org(bare_host(&host_of(&url))) {
            Some(org) => format!("https://{CLOUD}/{org}"),
            None => url,
        }
    }

    /// Code (read and write), Build (read and execute), and Work Items (read and write) for
    /// Azure Boards. Azure does not tell a token's rights: none is checked.
    fn scopes(&self) -> Vec<String> {
        ["vso.code_write", "vso.build_execute", "vso.work_write"]
            .map(String::from)
            .to_vec()
    }

    fn token_page(&self, url: &str) -> String {
        format!("{}/_usersSettings/tokens", url.trim_end_matches('/'))
    }

    /// `az account get-access-token`, for dev.azure.com only (a Server knows no Entra ID token).
    fn cli(&self, host: &str) -> Option<CliToken> {
        if !host.is_empty() && host != CLOUD {
            return None;
        }
        Some(CliToken {
            program: "az".into(),
            args: [
                "account",
                "get-access-token",
                "--resource",
                RESOURCE,
                "--query",
                "accessToken",
                "--output",
                "tsv",
            ]
            .map(str::to_string)
            .to_vec(),
            login: "az login".into(),
            read: CliRead::StdoutWord,
        })
    }

    fn client(&self, http: reqwest::Client, account: &Credentials) -> Box<dyn Forge> {
        Box::new(Azure::new(
            http,
            account.url.clone(),
            account.api.clone().or_else(|| api_override(&account.url)),
            account.token.clone(),
        ))
    }
}
