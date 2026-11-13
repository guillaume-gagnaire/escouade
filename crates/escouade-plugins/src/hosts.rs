//! The host of an address (the app's `forge` re-exports them): what a plugin may reach, and whose account it is.

/// The host of an instance's address (`https://git.acme.dev:8443` → `git.acme.dev:8443`), in
/// lower case: what an account is known by, one per instance.
pub fn host_of(url: &str) -> String {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    rest.split('/').next().unwrap_or("").to_ascii_lowercase()
}

/// The host without its port (what a remote's address gives).
pub fn bare_host(host: &str) -> &str {
    match host.rsplit_once(':') {
        Some((h, p)) if p.chars().all(|c| c.is_ascii_digit()) => h,
        _ => host,
    }
}
