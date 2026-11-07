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
    let Ok(url) = reqwest::Url::parse(site) else {
        return false;
    };
    match url.scheme() {
        "https" => true,
        "http" => matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]")),
        _ => false,
    }
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

    #[test]
    fn the_host_of_a_site_is_the_one_its_address_really_names() {
        // What stands before an @ is a user, however local it looks.
        assert!(!secure("http://localhost:1@exemple.dev"));
        assert!(!secure("http://127.0.0.1@exemple.dev/"));
        assert!(secure("http://user@localhost:4101"));
        assert!(!secure("http://localhost.exemple.dev"));
        assert!(secure("http://[::1]:4101") && secure("http://[::1]"));
        assert!(!secure("http://[::2]:4101"));
        assert!(secure("http://LocalHost:4101") && secure("HTTP://127.0.0.1"));
        assert!(secure("HTTPS://atlas.atlassian.net"));
        assert!(!secure("HTTP://Atlas.Atlassian.Net"));
    }

    #[test]
    fn what_is_no_web_address_takes_no_token() {
        assert!(!secure("https://") && !secure("http://"));
        assert!(!secure("ftp://localhost") && !secure("localhost:4101"));
    }
}
