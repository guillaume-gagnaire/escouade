//! The password of the proxy's address (SEC-10, ESC-595): never written in the settings' file, nor sent to the window.
//! `Settings::proxy_url` holds the whole address in memory, password included, for the requests and the agents'
//! variables; whatever serializes the settings writes it without its password (`serialize_without_password`), and the
//! password goes to the system's keychain under `SECRET_ENTRY`, or stays in the file apart
//! (`Settings::proxy_password_in_file`) on a machine whose keychain refuses it. The core moves it (`proxysecret`).
//!
//! The address is read as the user wrote it, never rebuilt by `url`: what comes back is their text, byte for byte,
//! the password as they spelt it (percent-encoded or not).

use serde::Serializer;

/// The keychain's entry of the proxy's password.
pub const SECRET_ENTRY: &str = "proxy";

/// Where the user's part (`user:password`) of `url` is, as byte offsets in it: what follows the scheme's `://` (or the
/// start, for an address without one) up to the last `@` before the path. None without one.
fn userinfo(url: &str) -> Option<(usize, usize)> {
    let start = url.find("://").map_or(0, |i| i + 3);
    let rest = &url[start..];
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let at = rest[..end].rfind('@')?;
    Some((start, start + at))
}

/// The password of `url`, as written: None without one (no `user:` before an `@`, or an empty password).
pub fn password_of(url: &str) -> Option<String> {
    let (from, to) = userinfo(url)?;
    let info = &url[from..to];
    let (_, password) = info.split_once(':')?;
    (!password.is_empty()).then(|| password.to_string())
}

/// The user's name of `url`, as written: None without one.
pub fn user_of(url: &str) -> Option<String> {
    let (from, to) = userinfo(url)?;
    let user = url[from..to].split(':').next().unwrap_or_default();
    (!user.is_empty()).then(|| user.to_string())
}

/// `url` without its password: `http://ada:secret@proxy:3128` gives `http://ada@proxy:3128`; an address without one
/// as it is.
pub fn without_password(url: &str) -> String {
    let Some((from, to)) = userinfo(url) else {
        return url.to_string();
    };
    match url[from..to].split_once(':') {
        Some((user, _)) => format!("{}{user}{}", &url[..from], &url[to..]),
        None => url.to_string(),
    }
}

/// `url` with `password` after its user's name: an address with no user's name, or one with a password, as it is.
pub fn with_password(url: &str, password: &str) -> String {
    let Some((from, to)) = userinfo(url) else {
        return url.to_string();
    };
    let info = &url[from..to];
    if info.is_empty() || info.contains(':') || password.is_empty() {
        return url.to_string();
    }
    format!("{}{info}:{password}{}", &url[..from], &url[to..])
}

/// `Settings::proxy_url` as the settings' file and the window get it: without its password.
pub fn serialize_without_password<S: Serializer>(url: &str, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&without_password(url))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_password_is_read_and_taken_out_as_written() {
        let url = "http://ada:s%40cr:et@proxy.corp:3128/path?q=a@b";
        assert_eq!(password_of(url).as_deref(), Some("s%40cr:et"));
        assert_eq!(user_of(url).as_deref(), Some("ada"));
        assert_eq!(
            without_password(url),
            "http://ada@proxy.corp:3128/path?q=a@b"
        );
        assert_eq!(with_password(&without_password(url), "s%40cr:et"), url);
    }

    #[test]
    fn an_address_without_a_password_stays_as_it_is() {
        for url in [
            "",
            "http://proxy:3128",
            "proxy:3128",
            "http://ada@proxy:3128",
            "http://proxy:3128/a@b",
        ] {
            assert_eq!(password_of(url), None, "{url}");
            assert_eq!(without_password(url), url, "{url}");
        }
        // An empty password is none: the address keeps its user's name alone.
        assert_eq!(password_of("http://ada:@proxy:3128"), None);
        assert_eq!(
            without_password("http://ada:@proxy:3128"),
            "http://ada@proxy:3128"
        );
        assert_eq!(password_of("ada:pw@proxy:3128").as_deref(), Some("pw"));
        assert_eq!(without_password("ada:pw@proxy:3128"), "ada@proxy:3128");
    }

    #[test]
    fn a_password_goes_only_after_a_users_name() {
        assert_eq!(
            with_password("http://proxy:3128", "pw"),
            "http://proxy:3128"
        );
        assert_eq!(
            with_password("http://ada:x@proxy:3128", "pw"),
            "http://ada:x@proxy:3128"
        );
        assert_eq!(
            with_password("http://ada@proxy:3128", ""),
            "http://ada@proxy:3128"
        );
        assert_eq!(
            with_password("http://ada@proxy:3128", "pw"),
            "http://ada:pw@proxy:3128"
        );
    }

    #[test]
    fn serialized_the_address_has_no_password() {
        #[derive(serde::Serialize)]
        struct S {
            #[serde(serialize_with = "serialize_without_password")]
            url: String,
        }
        let s = S {
            url: "http://ada:secret@proxy:3128".into(),
        };
        assert_eq!(
            serde_json::to_string(&s).unwrap(),
            r#"{"url":"http://ada@proxy:3128"}"#
        );
    }
}
