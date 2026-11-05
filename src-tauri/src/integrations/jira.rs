//! Jira, out of the core: its plugin serves it (`plugins/jira`, INT-7.3). What stays is the
//! address of a site and the rule that decides where a token may be sent, which the forms of the accounts apply.

/// A site's address as typed ("atlas.atlassian.net/"), as the API is called on.
pub fn site(typed: &str) -> String {
    let s = typed.trim().trim_end_matches('/');
    if s.is_empty() || s.contains("://") {
        s.to_string()
    } else {
        format!("https://{s}")
    }
}

/// A site its token may be sent to: over https, or over http to this machine only (a fake one).
pub fn secure(site: &str) -> bool {
    if site.starts_with("https://") {
        return true;
    }
    let host = site
        .strip_prefix("http://")
        .unwrap_or(site)
        .split('/')
        .next()
        .unwrap_or("");
    let name = host.rsplit_once(':').map_or(host, |(h, _)| h);
    matches!(name, "localhost" | "127.0.0.1" | "[::1]")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_typed_site_is_an_address() {
        assert_eq!(
            site(" atlas.atlassian.net/ "),
            "https://atlas.atlassian.net"
        );
        assert_eq!(site("http://localhost:4101"), "http://localhost:4101");
    }

    #[test]
    fn a_token_goes_over_https_or_to_this_machine_only() {
        assert!(secure("https://atlas.atlassian.net"));
        assert!(secure("http://localhost:4101") && secure("http://127.0.0.1:1"));
        assert!(!secure("http://atlas.atlassian.net"));
    }
}
