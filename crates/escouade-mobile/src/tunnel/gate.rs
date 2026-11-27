//! The gate of the local proxy. On a phone, every app reaches 127.0.0.1: the proxy forwards a connection only if its
//! first request names the session's secret as the first segment of its path (`GET /<secret>/remote HTTP/1.1`), and
//! takes the secret out before it goes into the tunnel (`GET /remote HTTP/1.1`). The app's WebView knows the secret
//! (the plugin gives it the address); another app does not. Behind it, the API still asks for the device's token:
//! the proxy writes it itself (ESC-570, SR1-15), in the header [`DEVICE_HEADER`] after the first line, so that the
//! WebView's JavaScript never holds it (its handshake gives an empty one).

use subtle::ConstantTimeEq;

/// The header the proxy writes the device's token in (`src-tauri/src/remoteapi/proto.rs`, `DEVICE_HEADER`).
pub const DEVICE_HEADER: &str = "X-Escouade-Device";

/// The longest first line read before giving up (a WebSocket's is a few dozen bytes).
pub const MAX_LINE: usize = 8 * 1024;

/// What is sent back to a connection that does not name the secret, before closing it.
pub const REFUSAL: &[u8] =
    b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";

#[derive(Debug, PartialEq, Eq)]
pub enum Gate {
    /// The first line is not whole yet.
    NeedMore,
    /// The bytes to send into the tunnel: the first line without the secret, then what followed it.
    Pass(Vec<u8>),
    /// Not the secret, or not a request.
    Refuse,
}

/// Looks at the first bytes of a connection (all of them read so far); `token`: the device's, written in the request
/// (a token that is not a header's printable text is not written).
pub fn check(buf: &[u8], secret: &str, token: Option<&str>) -> Gate {
    let Some(end) = buf.windows(2).position(|w| w == b"\r\n") else {
        return if buf.len() >= MAX_LINE {
            Gate::Refuse
        } else {
            Gate::NeedMore
        };
    };
    let line = &buf[..end];
    let mut parts = line.splitn(3, |&b| b == b' ');
    let (Some(method), Some(target), Some(version)) = (parts.next(), parts.next(), parts.next())
    else {
        return Gate::Refuse;
    };
    if method.is_empty() || !version.starts_with(b"HTTP/") {
        return Gate::Refuse;
    }
    let Some(path) = target.strip_prefix(b"/") else {
        return Gate::Refuse;
    };
    let cut = path
        .iter()
        .position(|&b| b == b'/' || b == b'?')
        .unwrap_or(path.len());
    let (given, rest) = path.split_at(cut);
    if given.len() != secret.len() || !bool::from(given.ct_eq(secret.as_bytes())) {
        return Gate::Refuse;
    }
    let mut out = Vec::with_capacity(buf.len());
    out.extend_from_slice(method);
    out.push(b' ');
    if !rest.starts_with(b"/") {
        out.push(b'/');
    }
    out.extend_from_slice(rest);
    out.push(b' ');
    out.extend_from_slice(version);
    if let Some(t) = token.filter(|t| !t.is_empty() && t.bytes().all(|b| b.is_ascii_graphic())) {
        out.extend_from_slice(b"\r\n");
        out.extend_from_slice(DEVICE_HEADER.as_bytes());
        out.extend_from_slice(b": ");
        out.extend_from_slice(t.as_bytes());
    }
    out.extend_from_slice(&buf[end..]);
    Gate::Pass(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: &str = "Zm9vYmFyYmF6cXV4";

    fn pass(input: &str) -> String {
        match check(input.as_bytes(), S, None) {
            Gate::Pass(out) => String::from_utf8(out).unwrap(),
            other => panic!("{input:?}: {other:?}"),
        }
    }

    #[test]
    fn takes_the_secret_out_of_the_path() {
        assert_eq!(
            pass(&format!(
                "GET /{S}/remote HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"
            )),
            "GET /remote HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"
        );
        assert_eq!(
            pass(&format!("GET /{S} HTTP/1.1\r\n")),
            "GET / HTTP/1.1\r\n"
        );
        assert_eq!(
            pass(&format!("GET /{S}/ HTTP/1.1\r\n")),
            "GET / HTTP/1.1\r\n"
        );
        assert_eq!(
            pass(&format!("GET /{S}?a=1 HTTP/1.1\r\n")),
            "GET /?a=1 HTTP/1.1\r\n"
        );
        // What follows the first line goes on as it is, a WebSocket frame sent early included.
        assert_eq!(
            pass(&format!("POST /{S}/x HTTP/1.1\r\n\r\n\u{1}")),
            "POST /x HTTP/1.1\r\n\r\n\u{1}"
        );
    }

    #[test]
    fn waits_for_the_whole_first_line() {
        assert_eq!(check(b"", S, None), Gate::NeedMore);
        assert_eq!(
            check(format!("GET /{S}/remote HT").as_bytes(), S, None),
            Gate::NeedMore
        );
        assert_eq!(
            check(format!("GET /{S}/remote HTTP/1.1\r").as_bytes(), S, None),
            Gate::NeedMore
        );
        assert_eq!(check(&vec![b'a'; MAX_LINE], S, None), Gate::Refuse);
    }

    #[test]
    fn writes_the_device_s_token_after_the_first_line_and_never_a_broken_one() {
        let pass = |token: Option<&str>| match check(
            format!("GET /{S}/remote HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n").as_bytes(),
            S,
            token,
        ) {
            Gate::Pass(out) => String::from_utf8(out).unwrap(),
            other => panic!("{other:?}"),
        };
        assert_eq!(
            pass(Some("tok_EN-1")),
            "GET /remote HTTP/1.1\r\nX-Escouade-Device: tok_EN-1\r\nHost: 127.0.0.1\r\n\r\n"
        );
        // None, empty, or a text that would break the request: nothing written.
        for t in [None, Some(""), Some("a\r\nHost: x"), Some("a b")] {
            assert_eq!(
                pass(t),
                "GET /remote HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
                "{t:?}"
            );
        }
    }

    #[test]
    fn refuses_without_the_secret() {
        for bad in [
            "GET /remote HTTP/1.1\r\n".to_string(),
            "GET / HTTP/1.1\r\n".to_string(),
            format!("GET /{S}x/remote HTTP/1.1\r\n"),
            format!("GET /x{S}/remote HTTP/1.1\r\n"),
            format!("GET /{}/remote HTTP/1.1\r\n", &S[..S.len() - 1]),
            format!("GET //{S}/remote HTTP/1.1\r\n"),
            format!("GET {S}/remote HTTP/1.1\r\n"),
            format!("GET http://x/{S}/remote HTTP/1.1\r\n"),
            format!("GET /{S}/remote\r\n"),
            format!("GET /{S}/remote SSH-2.0\r\n"),
            format!(" /{S}/remote HTTP/1.1\r\n"),
            "SSH-2.0-OpenSSH_9.6\r\n".to_string(),
        ] {
            assert_eq!(check(bad.as_bytes(), S, Some("t")), Gate::Refuse, "{bad:?}");
        }
    }
}
