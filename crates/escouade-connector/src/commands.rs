//! The commands a person runs (§ 4.5): `pair`, `unpair` and `check`. They say what they did in plain text; the service's
//! journal says the pairing's change when it sees it.

use std::time::{Duration, Instant};

use escouade_connector_proto::pairing::{self, PairError};
use escouade_connector_proto::transport::{self, ClientConfig, Proxy};
use escouade_connector_proto::{Keypair, Version};

use crate::ceiling::{show, Rule};
use crate::config::Config;
use crate::guard::{self, Guards, Resolve};
use crate::keys::{Pairing, State};
use crate::run::Dial;

/// The whole pairing within this.
pub const PAIR_TIMEOUT: Duration = Duration::from_secs(30);

/// The entry's address in shape: `wss://host[:port]/connector` (the path `/connector` when none is given).
pub fn entry_address(address: &str) -> Result<String, String> {
    let mut url = url::Url::parse(address.trim())
        .map_err(|_| format!("« {address} » is not an address (wss://host:port/connector)"))?;
    if url.scheme() != "wss" {
        return Err(format!(
            "« {address} »: a wss:// address alone (the one Space settings › Private network gives)"
        ));
    }
    if url.host_str().is_none() {
        return Err(format!("« {address} »: no host"));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(format!(
            "« {address} »: no user nor password in the address"
        ));
    }
    if url.path() == "/" || url.path().is_empty() {
        url.set_path(escouade_connector_proto::PATH);
    }
    let path = url.path().trim_end_matches('/').to_string();
    url.set_path(&path);
    url.set_query(None);
    url.set_fragment(None);
    Ok(url.to_string())
}

/// `escouade-connector pair <address> <code>`: a new pair of keys, the code proved to the server, both public keys
/// kept. The code is never written anywhere, nor said back.
pub async fn pair<D: Dial>(
    address: &str,
    code: &str,
    config: Option<&Config>,
    state: &State,
    version: Version,
    system_roots: bool,
    dial: &D,
) -> Result<String, String> {
    if let Some((p, _)) = state.load().map_err(|e| e.to_string())? {
        return Err(format!(
            "this connector is already paired with {}: escouade-connector unpair first",
            p.server
        ));
    }
    let entry = entry_address(address)?;
    if let Some(configured) = config.and_then(|c| c.server.as_deref()) {
        if configured.trim_end_matches('/') != entry {
            return Err(format!(
                "connector.toml names the server {configured}, not {entry}: change it, or pair with it"
            ));
        }
    }
    if pairing::normalize_code(code).len() < 8 {
        return Err("the code has 8 letters (as ABCD-EFGH)".into());
    }
    let url = pairing::pairing_url(&entry).ok_or("not a wss:// address")?;
    let mut client = ClientConfig::new(url);
    client.system_roots = system_roots;
    client.timeout = PAIR_TIMEOUT;
    if let Some(c) = config {
        client.authorities = c.authorities.clone();
        client.proxy = c.proxy.clone();
    }
    if client.proxy.is_none() {
        let host = transport::parse_target(&entry)
            .map_err(|e| e.to_string())?
            .host;
        client.proxy = Proxy::from_env(&host).and_then(Result::ok);
    }
    let ws = dial.dial(&client).await.map_err(|e| e.to_string())?;
    let keys = Keypair::generate();
    let paired = pairing::pair_server(ws, &keys, code, version, PAIR_TIMEOUT)
        .await
        .map_err(|e| match e {
            PairError::Version { .. } => e.to_string(),
            other => other.to_string(),
        })?;
    state
        .save(
            &Pairing {
                server: entry.clone(),
                server_key: paired.server,
                connector_key: keys.public(),
            },
            &keys,
        )
        .map_err(|e| format!("paired, but the keys are not kept: {e}"))?;
    let mut said = format!(
        "paired with {entry}\n  the server's key: {}\n  this connector's key: {}\n  kept in {}",
        paired.server,
        keys.public(),
        state.dir.display()
    );
    if let Some(advice) = paired.welcome.advice(version) {
        said.push_str(&format!("\n  {advice}"));
    }
    Ok(said)
}

/// `escouade-connector unpair`: the keys forgotten (zeros over the private one). The server keeps the connector until
/// it is removed from the space; without its key, nothing passes.
pub fn unpair(state: &State) -> Result<String, String> {
    let paired = state.load().ok().flatten().map(|(p, _)| p.server);
    match state.remove().map_err(|e| e.to_string())? {
        true => Ok(match paired {
            Some(server) => format!(
                "unpaired from {server}: the keys are forgotten (remove the connector from the space too)"
            ),
            None => "unpaired: the keys are forgotten".into(),
        }),
        false => Ok("this connector was not paired".into()),
    }
}

/// A line of `check`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CheckLine {
    pub rule: String,
    pub ok: bool,
    pub text: String,
}

/// `escouade-connector check`: each host of the ceiling resolved, checked and joined as an `open` would be (a pattern
/// is said, not tried: its names are known only when asked).
pub async fn check(
    config: &Config,
    resolver: &dyn Resolve,
    guards: &Guards,
    timeout: Duration,
) -> Vec<CheckLine> {
    let mut out = Vec::new();
    for rule in &config.ceiling.rules {
        let (host, port) = match rule {
            Rule::Under { .. } => {
                out.push(CheckLine {
                    rule: rule.to_string(),
                    ok: true,
                    text: "a pattern: each name under it is checked when an agent asks for it"
                        .into(),
                });
                continue;
            }
            Rule::Exact { host, port } => (host.clone(), *port),
            Rule::Address { ip, port } => (ip.to_string(), *port),
        };
        let started = Instant::now();
        let line = match guard::admit(&host, port, &config.ceiling, resolver, guards).await {
            Err(refusal) => CheckLine {
                rule: rule.to_string(),
                ok: false,
                text: refusal.reason,
            },
            Ok(admitted) => {
                let resolved = admitted
                    .addrs
                    .iter()
                    .map(|a| a.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                match guard::join(&admitted, port, timeout).await {
                    Ok((_tcp, addr)) => {
                        let mut text = format!(
                            "{} answers at {} ({} ms)",
                            show(&admitted.host, port),
                            addr,
                            started.elapsed().as_millis()
                        );
                        if !admitted.dropped.is_empty() {
                            let dropped = admitted
                                .dropped
                                .iter()
                                .map(|(ip, why)| format!("{ip}: {why}"))
                                .collect::<Vec<_>>()
                                .join("; ");
                            text.push_str(&format!(", never joined: {dropped}"));
                        }
                        CheckLine {
                            rule: rule.to_string(),
                            ok: true,
                            text,
                        }
                    }
                    Err(refusal) => CheckLine {
                        rule: rule.to_string(),
                        ok: false,
                        text: format!("resolved to {resolved}; {}", refusal.reason),
                    },
                }
            }
        };
        out.push(line);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_entry_is_put_in_shape() {
        assert_eq!(
            entry_address("wss://escouade.example.com:8443/connector").unwrap(),
            "wss://escouade.example.com:8443/connector"
        );
        assert_eq!(
            entry_address("wss://escouade.example.com").unwrap(),
            "wss://escouade.example.com/connector"
        );
        assert_eq!(
            entry_address("wss://escouade.example.com/connector/?x=1").unwrap(),
            "wss://escouade.example.com/connector"
        );
        assert!(entry_address("https://escouade.example.com/connector").is_err());
        assert!(entry_address("wss://u:p@escouade.example.com/connector").is_err());
        assert!(entry_address("escouade.example.com").is_err());
    }
}
