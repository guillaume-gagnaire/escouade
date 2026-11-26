//! The gate in HTTP: `CONNECT host:port` (a tunnel, for HTTPS and `escouade-server via`), and a request in plain HTTP
//! in its absolute form (`GET http://host/path`, what `HTTP_PROXY` makes of an `http://` address), sent on with
//! `Connection: close`. The token comes in `Proxy-Authorization` (Basic, the token as the user's name), as clients
//! send it from `http://<token>@127.0.0.1:P`.

use crate::lists::Target;
use crate::pipe::{pipe, read_head};
use crate::reason::{ascii, Reason};
use crate::{Asked, Inner};
use base64::Engine as _;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

/// The answer of a refusal: its status, its sentence as the reason phrase (in ASCII) and as its body.
fn refusal(why: &Reason, said: &str) -> Vec<u8> {
    let status = why.status();
    let phrase = ascii(said);
    let body = format!("{said}\n");
    let mut out = format!("HTTP/1.1 {status} {phrase}\r\n");
    if status == 407 {
        out.push_str("Proxy-Authenticate: Basic realm=\"escouade\"\r\n");
    }
    out.push_str(&format!(
        "Content-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    ));
    out.into_bytes()
}

/// The token of a `Proxy-Authorization: Basic …`: the user's name (the password, if any, is not read).
pub(crate) fn token_of(value: &str) -> Option<String> {
    let (scheme, rest) = value.trim().split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("basic") {
        return None;
    }
    let pair = base64::engine::general_purpose::STANDARD
        .decode(rest.trim())
        .ok()?;
    let pair = String::from_utf8(pair).ok()?;
    let user = pair.split_once(':').map(|(u, _)| u).unwrap_or(&pair);
    Some(user.to_string())
}

/// What a request asks.
struct Request {
    method: String,
    /// `host:port` of a `CONNECT`, or the absolute address of a plain request.
    target: String,
    token: Option<String>,
    /// The other headers, as they came (those of the proxy left out).
    headers: Vec<(String, Vec<u8>)>,
}

fn parse(head: &[u8]) -> Result<Request, String> {
    let mut headers = [httparse::EMPTY_HEADER; 96];
    let mut req = httparse::Request::new(&mut headers);
    match req.parse(head) {
        Ok(httparse::Status::Complete(_)) => {}
        Ok(httparse::Status::Partial) => return Err("incomplete head".into()),
        Err(e) => return Err(e.to_string()),
    }
    let mut token = None;
    let mut kept = Vec::new();
    for h in req.headers.iter() {
        let name = h.name.to_ascii_lowercase();
        match name.as_str() {
            "proxy-authorization" => {
                token = std::str::from_utf8(h.value).ok().and_then(token_of);
            }
            "proxy-connection" | "connection" | "keep-alive" => {}
            _ => kept.push((h.name.to_string(), h.value.to_vec())),
        }
    }
    Ok(Request {
        method: req.method.unwrap_or_default().to_string(),
        target: req.path.unwrap_or_default().to_string(),
        token,
        headers: kept,
    })
}

/// The host and port, and the path, of a plain request's absolute address.
fn absolute(target: &str) -> Result<(Target, String), String> {
    let u = url::Url::parse(target).map_err(|e| format!("{target}: {e}"))?;
    if u.scheme() != "http" {
        return Err(format!(
            "{target}: only http:// goes in plain (https:// takes CONNECT)"
        ));
    }
    let host = u.host_str().ok_or_else(|| format!("{target}: no host"))?;
    let t = Target::new(host, u.port_or_known_default().unwrap_or(80))
        .ok_or_else(|| format!("{target}: no host"))?;
    let mut path = u.path().to_string();
    if let Some(q) = u.query() {
        path.push('?');
        path.push_str(q);
    }
    Ok((t, path))
}

/// The head sent on for a plain request: in origin form to the host, in absolute form to the space's proxy.
fn head_on(req: &Request, line_target: &str, t: &Target, upstream_auth: Option<&str>) -> Vec<u8> {
    let mut out = format!("{} {} HTTP/1.1\r\n", req.method, line_target).into_bytes();
    let mut host = false;
    for (name, value) in &req.headers {
        host |= name.eq_ignore_ascii_case("host");
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(b": ");
        out.extend_from_slice(value);
        out.extend_from_slice(b"\r\n");
    }
    if !host {
        out.extend_from_slice(format!("Host: {t}\r\n").as_bytes());
    }
    if let Some(a) = upstream_auth {
        out.extend_from_slice(format!("Proxy-Authorization: {a}\r\n").as_bytes());
    }
    out.extend_from_slice(b"Connection: close\r\n\r\n");
    out
}

pub(crate) async fn serve(inner: &Inner, mut s: TcpStream) {
    let mut asked = Asked::new("http");
    let (head, rest) = match read_head(&mut s).await {
        Ok(h) => h,
        Err(_) => return,
    };
    let refuse = |asked: &Asked, why: Reason| {
        inner.refused(asked, &why);
        refusal(&why, &why.say(inner.lang))
    };
    let req = match parse(&head) {
        Ok(r) => r,
        Err(e) => {
            let answer = refuse(&asked, Reason::Malformed(e));
            let _ = s.write_all(&answer).await;
            return;
        }
    };
    let connect = req.method.eq_ignore_ascii_case("CONNECT");
    let aimed = if connect {
        Target::parse(&req.target)
            .map(|t| (t, String::new()))
            .ok_or_else(|| format!("{}: no host:port", req.target))
    } else {
        absolute(&req.target)
    };
    let (t, path) = match aimed {
        Ok(a) => a,
        Err(e) => {
            let answer = refuse(&asked, Reason::Malformed(e));
            let _ = s.write_all(&answer).await;
            return;
        }
    };
    asked.target = Some(t.clone());
    let holder = match inner.admit(req.token.as_deref()) {
        Ok(h) => h,
        Err(why) => {
            let answer = refuse(&asked, why);
            let _ = s.write_all(&answer).await;
            return;
        }
    };
    asked.holder = Some(holder.clone());
    let mut reached = match inner.reach(&holder, &t, !connect).await {
        Ok(r) => r,
        Err(why) => {
            let answer = refuse(&asked, why);
            let _ = s.write_all(&answer).await;
            return;
        }
    };
    if connect {
        if s.write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
            .await
            .is_err()
        {
            return;
        }
    } else {
        let head = match &reached.upstream_auth {
            Some(auth) => head_on(&req, &req.target, &t, auth.as_deref()),
            None => head_on(&req, &path, &t, None),
        };
        if reached.flow.write_all(&head).await.is_err() {
            return;
        }
    }
    if !rest.is_empty() && reached.flow.write_all(&rest).await.is_err() {
        return;
    }
    let bytes = pipe(s, reached.flow).await;
    let up = bytes.0 + rest.len() as u64;
    inner.ended(&asked, (reached.route, reached.connector), (up, bytes.1));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_token_is_the_user_s_name_of_basic() {
        let v = format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode("abc123:")
        );
        assert_eq!(token_of(&v).as_deref(), Some("abc123"));
        let v = format!(
            "basic {}",
            base64::engine::general_purpose::STANDARD.encode("abc")
        );
        assert_eq!(token_of(&v).as_deref(), Some("abc"));
        assert_eq!(token_of("Bearer x"), None);
    }

    #[test]
    fn a_plain_request_goes_on_in_origin_form_without_the_proxy_s_headers() {
        let head = b"GET http://Example.org:8080/a?b=1 HTTP/1.1\r\nHost: example.org:8080\r\nProxy-Authorization: Basic eDo=\r\nProxy-Connection: keep-alive\r\nAccept: */*\r\n\r\n";
        let req = parse(head).unwrap();
        assert_eq!(req.token.as_deref(), Some("x"));
        let (t, path) = absolute(&req.target).unwrap();
        assert_eq!(t.to_string(), "example.org:8080");
        assert_eq!(path, "/a?b=1");
        let on = String::from_utf8(head_on(&req, &path, &t, None)).unwrap();
        assert!(on.starts_with("GET /a?b=1 HTTP/1.1\r\n"), "{on}");
        assert!(!on.to_ascii_lowercase().contains("proxy-"), "{on}");
        assert!(on.ends_with("Connection: close\r\n\r\n"));
        assert!(absolute("https://x/").is_err());
    }
}
