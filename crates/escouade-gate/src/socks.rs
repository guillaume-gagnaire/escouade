//! The gate in SOCKS5 (RFC 1928), by name (`socks5h`): the token as the user's name of RFC 1929 (what a client makes of
//! `socks5h://<token>@127.0.0.1:P`), `CONNECT` only. An address the agent resolved itself is taken as an address: it is
//! never taken for a name of a private network, and goes where an address goes (§ 4.4: a literal address is refused by
//! a connector unless its ceiling writes it so).

use crate::lists::Target;
use crate::pipe::pipe;
use crate::reason::Reason;
use crate::{Asked, Inner};
use std::net::{Ipv4Addr, Ipv6Addr};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// The replies of RFC 1928.
const OK: u8 = 0x00;
const FAILURE: u8 = 0x01;
const NOT_ALLOWED: u8 = 0x02;
const HOST_UNREACHABLE: u8 = 0x04;
const COMMAND_NOT_SUPPORTED: u8 = 0x07;
const ADDRESS_NOT_SUPPORTED: u8 = 0x08;

fn reply(code: u8) -> [u8; 10] {
    [5, code, 0, 1, 0, 0, 0, 0, 0, 0]
}

/// The code of a refusal: « connection not allowed by ruleset » for what the gate or a connector refused.
fn code_of(why: &Reason) -> u8 {
    match why {
        Reason::Unreachable { .. } | Reason::UpstreamRefused { .. } => HOST_UNREACHABLE,
        Reason::Malformed(_) => FAILURE,
        _ => NOT_ALLOWED,
    }
}

async fn token(s: &mut TcpStream) -> std::io::Result<Option<Option<String>>> {
    let mut hello = [0u8; 2];
    s.read_exact(&mut hello).await?;
    let mut methods = vec![0u8; hello[1] as usize];
    s.read_exact(&mut methods).await?;
    if !methods.contains(&2) {
        // No user's name offered: no token.
        s.write_all(&[5, 0xff]).await?;
        return Ok(None);
    }
    s.write_all(&[5, 2]).await?;
    let mut v = [0u8; 2];
    s.read_exact(&mut v).await?;
    let mut user = vec![0u8; v[1] as usize];
    s.read_exact(&mut user).await?;
    let mut plen = [0u8; 1];
    s.read_exact(&mut plen).await?;
    let mut pass = vec![0u8; plen[0] as usize];
    s.read_exact(&mut pass).await?;
    Ok(Some(String::from_utf8(user).ok()))
}

async fn target(s: &mut TcpStream) -> std::io::Result<Result<Target, u8>> {
    let mut head = [0u8; 4];
    s.read_exact(&mut head).await?;
    let host = match head[3] {
        1 => {
            let mut a = [0u8; 4];
            s.read_exact(&mut a).await?;
            Ipv4Addr::from(a).to_string()
        }
        3 => {
            let mut n = [0u8; 1];
            s.read_exact(&mut n).await?;
            let mut name = vec![0u8; n[0] as usize];
            s.read_exact(&mut name).await?;
            String::from_utf8_lossy(&name).into_owned()
        }
        4 => {
            let mut a = [0u8; 16];
            s.read_exact(&mut a).await?;
            Ipv6Addr::from(a).to_string()
        }
        _ => return Ok(Err(ADDRESS_NOT_SUPPORTED)),
    };
    let mut port = [0u8; 2];
    s.read_exact(&mut port).await?;
    if head[1] != 1 {
        return Ok(Err(COMMAND_NOT_SUPPORTED));
    }
    Ok(Target::new(&host, u16::from_be_bytes(port)).ok_or(ADDRESS_NOT_SUPPORTED))
}

pub(crate) async fn serve(inner: &Inner, mut s: TcpStream) {
    let mut asked = Asked::new("socks");
    let given = match tokio::time::timeout(crate::pipe::HEAD_WITHIN, token(&mut s)).await {
        Ok(Ok(g)) => g,
        _ => return,
    };
    let holder = match given {
        None => Err(Reason::NoToken),
        Some(t) => inner.admit(t.as_deref()),
    };
    let holder = match holder {
        Ok(h) => {
            if s.write_all(&[1, 0]).await.is_err() {
                return;
            }
            h
        }
        Err(why) => {
            // The method refused, or the user's name: no request follows.
            if why == Reason::UnknownToken {
                let _ = s.write_all(&[1, 1]).await;
            }
            inner.refused(&asked, &why);
            return;
        }
    };
    asked.holder = Some(holder.clone());
    let t = match tokio::time::timeout(crate::pipe::HEAD_WITHIN, target(&mut s)).await {
        Ok(Ok(Ok(t))) => t,
        Ok(Ok(Err(code))) => {
            let _ = s.write_all(&reply(code)).await;
            inner.refused(
                &asked,
                &Reason::Malformed(format!("SOCKS5 request refused ({code})")),
            );
            return;
        }
        _ => return,
    };
    asked.target = Some(t.clone());
    match inner.reach(&holder, &t, false).await {
        Ok(reached) => {
            if s.write_all(&reply(OK)).await.is_err() {
                return;
            }
            let bytes = pipe(s, reached.flow).await;
            inner.ended(&asked, (reached.route, reached.connector), bytes);
        }
        Err(why) => {
            let _ = s.write_all(&reply(code_of(&why))).await;
            inner.refused(&asked, &why);
        }
    }
}
