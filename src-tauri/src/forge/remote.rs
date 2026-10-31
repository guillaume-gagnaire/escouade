//! A git remote's address, whatever its form (https, ssh, scp-like, with a port, a user, `.git`
//! or not), and the repository each code host reads in it.

/// What an address names, before any host reads its path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Address {
    /// `https`, `http`, `ssh` (the scp-like form too) or `git`.
    pub scheme: String,
    /// In lower case, without the user or the port.
    pub host: String,
    /// The port given, if any (an ssh port, or an https one of a self-hosted instance).
    pub port: Option<u16>,
    /// The path's segments, without `.git` nor empty ones.
    pub segments: Vec<String>,
}

/// The parts of a remote's address: `https://[user@]host[:port]/path`, `ssh://[user@]host[:port]/path`,
/// `git://…`, `http://…`, or the scp-like `[user@]host:path`. None for a local path (`C:/remotes/x.git`,
/// `/srv/x.git`, `file://…`) or an address without a path.
pub fn parse(url: &str) -> Option<Address> {
    let url = url.trim();
    let (scheme, authority, path) = match url.split_once("://") {
        Some((scheme, rest)) => {
            let scheme = match scheme.to_ascii_lowercase().as_str() {
                s @ ("https" | "http" | "ssh" | "git") => s.to_string(),
                "git+ssh" | "ssh+git" => "ssh".to_string(),
                _ => return None,
            };
            let (authority, path) = rest.split_once('/')?;
            (scheme, authority, path)
        }
        None => {
            // scp-like: a colon before any slash, and not a Windows drive (`C:/…`, `C:\…`).
            let (authority, path) = url.split_once(':')?;
            if authority.is_empty() || authority.contains('/') || authority.contains('\\') {
                return None;
            }
            if authority.len() == 1 && authority.chars().all(|c| c.is_ascii_alphabetic()) {
                return None;
            }
            ("ssh".to_string(), authority, path)
        }
    };
    // The user (and a password or token) before the last `@`.
    let host_port = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
    let (host, port) = match host_port.rsplit_once(':') {
        // `[::1]` has colons of its own, and no port.
        Some((h, p)) if !p.contains(']') => match p.parse::<u16>() {
            Ok(n) => (h, Some(n)),
            Err(_) if p.is_empty() => (h, None),
            Err(_) => return None,
        },
        _ => (host_port, None),
    };
    let host = host.trim().to_ascii_lowercase();
    if host.is_empty() || host.contains(char::is_whitespace) {
        return None;
    }
    let path = path.split(['?', '#']).next().unwrap_or("");
    let mut segments: Vec<String> = path
        .split('/')
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    if let Some(last) = segments.last_mut() {
        if let Some(stem) = last.strip_suffix(".git") {
            *last = stem.to_string();
        }
    }
    segments.retain(|s| !s.is_empty());
    (!segments.is_empty()).then_some(Address {
        scheme,
        host,
        port,
        segments,
    })
}

/// A repository on a code host: its namespace (owner, organization, GitLab group and subgroups,
/// Bitbucket workspace) and its name.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(host: &str, port: Option<u16>, path: &str) -> Option<Address> {
        Some(Address {
            scheme: String::new(),
            host: host.into(),
            port,
            segments: path.split('/').map(str::to_string).collect(),
        })
    }

    #[test]
    fn every_form_of_address_is_read() {
        for (url, want) in [
            (
                "https://github.com/acme/demo.git",
                addr("github.com", None, "acme/demo"),
            ),
            (
                "https://github.com/acme/demo/",
                addr("github.com", None, "acme/demo"),
            ),
            (
                "http://127.0.0.1:3000/acme/demo",
                addr("127.0.0.1", Some(3000), "acme/demo"),
            ),
            (
                "https://x-token:secret@GitHub.com/acme/demo",
                addr("github.com", None, "acme/demo"),
            ),
            (
                "git@github.com:acme/demo.git",
                addr("github.com", None, "acme/demo"),
            ),
            (
                "github.com:acme/demo",
                addr("github.com", None, "acme/demo"),
            ),
            (
                "ssh://git@github.com/acme/demo.git",
                addr("github.com", None, "acme/demo"),
            ),
            (
                "ssh://git@ssh.github.com:443/acme/demo.git",
                addr("ssh.github.com", Some(443), "acme/demo"),
            ),
            (
                "git+ssh://git@gitea.example.com:2222/acme/demo.git",
                addr("gitea.example.com", Some(2222), "acme/demo"),
            ),
            (
                "git://example.com/acme/demo.git",
                addr("example.com", None, "acme/demo"),
            ),
            (
                "  https://gitlab.com/a/b/c/d.git\n",
                addr("gitlab.com", None, "a/b/c/d"),
            ),
        ] {
            let got = parse(url).map(|a| Address {
                scheme: String::new(),
                ..a
            });
            assert_eq!(got, want, "{url}");
        }
    }

    #[test]
    fn local_paths_and_odd_addresses_are_not_remotes() {
        for url in [
            "C:/remotes/demo.git",
            "C:\\remotes\\demo.git",
            "/srv/git/demo.git",
            "../demo",
            "file:///srv/git/demo.git",
            "https://github.com",
            "https://github.com/",
            "https://host:port/acme/demo",
            "",
        ] {
            assert_eq!(parse(url), None, "{url}");
        }
    }

    #[test]
    fn a_repository_is_two_segments_or_groups_and_a_project() {
        let s = |p: &str| p.split('/').map(str::to_string).collect::<Vec<_>>();
        assert_eq!(RepoRef::two(&s("acme/demo")).unwrap().full(), "acme/demo");
        assert_eq!(RepoRef::two(&s("acme")), None);
        assert_eq!(RepoRef::two(&s("a/b/c")), None);
        let r = RepoRef::nested(&s("acme/web/front/app")).unwrap();
        assert_eq!(
            (r.namespace.as_str(), r.name.as_str()),
            ("acme/web/front", "app")
        );
        assert_eq!(RepoRef::nested(&s("acme")), None);
    }
}
