//! `escouade-server via <host> <port>`: `ssh`'s `ProxyCommand` (C1, § 4.3). It opens `host:port` the way its
//! environment says, and passes the bytes between it and its standard input and output: through the gate when its
//! `HTTPS_PROXY` names it (a `CONNECT` with the agent's token: a host of the project's list goes through its connector,
//! another one straight out, GitHub stays reachable), straight otherwise (a git run without the agent's variables, a
//! server without the gate). A refusal is one line on its standard error, which `ssh` shows.

use crate::outside::{bypassed, no_proxy_of};
use base64::Engine as _;
use std::io::{self, Read, Write};
use std::net::{Shutdown, TcpStream};
use std::time::Duration;

/// The proxy `var` names for `host`: its host, port and `Proxy-Authorization`. `None` to go straight.
pub fn proxy_for(
    var: &dyn Fn(&str) -> Option<String>,
    host: &str,
) -> Option<(String, u16, Option<String>)> {
    let pick = |names: &[&str]| {
        names
            .iter()
            .find_map(|n| var(n).filter(|v| !v.trim().is_empty()))
    };
    let no = pick(&["NO_PROXY", "no_proxy"]).unwrap_or_default();
    if bypassed(&no_proxy_of(&no), host) {
        return None;
    }
    let url = pick(&["HTTPS_PROXY", "https_proxy", "HTTP_PROXY", "http_proxy"])?;
    let u = url::Url::parse(url.trim()).ok()?;
    if u.scheme() != "http" {
        return None;
    }
    let h = u.host_str()?;
    let h = h
        .strip_prefix('[')
        .and_then(|x| x.strip_suffix(']'))
        .unwrap_or(h)
        .to_string();
    let auth = (!u.username().is_empty()).then(|| {
        let pair = format!("{}:{}", u.username(), u.password().unwrap_or(""));
        format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(pair)
        )
    });
    Some((h, u.port_or_known_default().unwrap_or(80), auth))
}

/// `host:port` opened, through the proxy of `var` or straight; or the sentence of why not.
pub fn connect(
    host: &str,
    port: u16,
    var: &dyn Fn(&str) -> Option<String>,
) -> Result<TcpStream, String> {
    let shown = if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    };
    let Some((ph, pp, auth)) = proxy_for(var, host) else {
        return TcpStream::connect((host, port)).map_err(|e| format!("{shown}: {e}"));
    };
    let mut s = TcpStream::connect((ph.as_str(), pp)).map_err(|e| format!("{ph}:{pp}: {e}"))?;
    let _ = s.set_nodelay(true);
    let mut ask = format!("CONNECT {shown} HTTP/1.1\r\nHost: {shown}\r\n");
    if let Some(a) = auth {
        ask.push_str(&format!("Proxy-Authorization: {a}\r\n"));
    }
    ask.push_str("\r\n");
    s.write_all(ask.as_bytes())
        .map_err(|e| format!("{ph}:{pp}: {e}"))?;
    let _ = s.set_read_timeout(Some(Duration::from_secs(60)));
    let head = read_head(&mut s).map_err(|e| format!("{ph}:{pp}: {e}"))?;
    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut answer = httparse::Response::new(&mut headers);
    let parsed = answer.parse(&head);
    let status = answer.code.unwrap_or(0);
    if matches!(parsed, Ok(httparse::Status::Complete(_))) && (200..300).contains(&status) {
        let _ = s.set_read_timeout(None);
        return Ok(s);
    }
    let length = answer
        .headers
        .iter()
        .find(|h| h.name.eq_ignore_ascii_case("content-length"))
        .and_then(|h| std::str::from_utf8(h.value).ok())
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(0)
        .min(4096);
    let mut body = vec![0u8; length];
    let _ = s.read_exact(&mut body);
    let said = String::from_utf8_lossy(&body).trim().to_string();
    Err(if said.is_empty() {
        format!("{shown}: the proxy answered {status}")
    } else {
        said
    })
}

/// The head of an answer, byte by byte (nothing of the tunnel read past it).
fn read_head(s: &mut TcpStream) -> io::Result<Vec<u8>> {
    let mut head = Vec::new();
    let mut b = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        if s.read(&mut b)? == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "closed before its answer",
            ));
        }
        head.push(b[0]);
        if head.len() > crate::pipe::HEAD_MAX {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "answer too long",
            ));
        }
    }
    Ok(head)
}

/// The bytes between `s` and `input`/`output` until `s` ends.
pub fn pump(
    s: TcpStream,
    mut input: impl Read + Send + 'static,
    mut output: impl Write,
) -> io::Result<()> {
    let mut sending = s.try_clone()?;
    std::thread::spawn(move || {
        let _ = io::copy(&mut input, &mut sending);
        let _ = sending.shutdown(Shutdown::Write);
    });
    let mut s = s;
    let mut buf = [0u8; 16 * 1024];
    loop {
        let n = match s.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        };
        output.write_all(&buf[..n])?;
        output.flush()?;
    }
    Ok(())
}

/// `escouade-server via` with `args` (`<host> <port>`): its exit code.
pub fn main(args: &[String]) -> i32 {
    let (host, port) = match args {
        [h, p] => match p.parse::<u16>() {
            Ok(p) if !h.is_empty() && !h.starts_with('-') => (h.as_str(), p),
            _ => {
                eprintln!("escouade-server via: {p}: not a port");
                return 2;
            }
        },
        _ => {
            eprintln!(
                "escouade-server via <host> <port> (ssh's ProxyCommand: escouade-server via %h %p)"
            );
            return 2;
        }
    };
    match connect(host, port, &|k| std::env::var(k).ok()) {
        Ok(s) => match pump(s, io::stdin(), io::stdout()) {
            Ok(()) => 0,
            Err(_) => 1,
        },
        Err(why) => {
            eprintln!("escouade-server via: {why}");
            1
        }
    }
}
