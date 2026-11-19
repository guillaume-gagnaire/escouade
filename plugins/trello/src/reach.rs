//! A service out of reach (its name unknown to the DNS, no connection, its delay passed), typed as the protocol says it:
//! the error 503 (`-32503`), which the core asks again later and the bench counts as typed (ESC-536). The kit says it by
//! an error with no type (-32000).

use anyhow::{anyhow, Error, Result};
use escouade_plugin_kit::errors::http_error_in;
use escouade_plugin_kit::errors::HttpError;
use escouade_plugin_kit::http::reqwest;
use escouade_plugin_kit::i18n::{self, Lang};
use escouade_plugin_kit::tr;
use escouade_plugin_kit::tr_in;
use serde_json::Value;

/// The status of a service out of reach: a 503, as a gateway answers for a server it cannot reach.
pub const UNREACHABLE: u16 = 503;

/// The call to `name` that did not reach it, for `e`: a typed error, in the interface's language.
pub fn unreached(name: &str, e: reqwest::Error) -> Error {
    unreached_in(i18n::ui(), name, e)
}

/// `unreached`, in `lang`.
pub fn unreached_in(lang: Lang, name: &str, e: reqwest::Error) -> Error {
    let e = Error::from(e.without_url());
    HttpError {
        status: UNREACHABLE,
        message: tr_in!(
            lang,
            "{name} injoignable : {e:#}. Vérifie la connexion ou le proxy (Réglages › Avancé › Réseau).",
            "{name} can’t be reached: {e:#}. Check the connection or the proxy (Settings › Advanced › Network)."
        ),
    }
    .into()
}

/// `http::call`: the JSON (null when empty) of a call to `name`, its redirections followed. Out of reach, a typed
/// error; a status other than a success, an `HttpError` that says what the service said.
pub async fn call(name: &str, rb: reqwest::RequestBuilder) -> Result<Value> {
    let resp = rb.send().await.map_err(|e| unreached(name, e))?;
    let status = resp.status().as_u16();
    let text = resp.text().await.unwrap_or_default();
    if !(200..300).contains(&status) {
        return Err(HttpError {
            status,
            message: http_error_in(i18n::ui(), name, status, &text),
        }
        .into());
    }
    if text.trim().is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(&text).map_err(|_| {
        anyhow!(tr!(
            "{name} : réponse illisible",
            "{name}: unreadable answer"
        ))
    })
}

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use super::*;
    use escouade_plugin_kit::protocol::code;
    use escouade_plugin_kit::serve::serve_on;
    use serde_json::{json, Value};
    use std::sync::Arc;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    /// An address where nothing answers, on this machine: neither DNS nor network.
    const NOWHERE: &str = "http://127.0.0.1:1";

    /// The points the plugin serves, as `account/me` names them.
    fn points() -> Vec<Value> {
        vec![json!({ "point": "ticketSource", "source": "trello" })]
    }

    /// What `account/me` of `point` answers, on an account at `NOWHERE` whose secret Escouade gives as `secret`: the
    /// code and the message of its error.
    async fn me(point: &Value, secret: Value) -> (i64, String) {
        let (mut to, input) = tokio::io::duplex(1 << 16);
        let (output, from) = tokio::io::duplex(1 << 16);
        tokio::spawn(serve_on(Arc::new(crate::hosts()), input, output));
        let mut from = BufReader::new(from).lines();
        let mut params = point.clone();
        params["account"] = json!({ "url": NOWHERE, "api": NOWHERE });
        let call =
            json!({ "jsonrpc": "2.0", "id": "me", "method": "account/me", "params": params });
        to.write_all(format!("{call}\n").as_bytes()).await.unwrap();
        loop {
            let line = from.next_line().await.unwrap().expect("an answer");
            let msg: Value = serde_json::from_str(&line).unwrap();
            if msg["method"] == "secrets/get" {
                let answer = json!({ "jsonrpc": "2.0", "id": msg["id"], "result": secret });
                to.write_all(format!("{answer}\n").as_bytes())
                    .await
                    .unwrap();
            } else if msg["id"] == "me" {
                let e = &msg["error"];
                let message = e["message"].as_str().unwrap_or_default().to_string();
                return (e["code"].as_i64().unwrap_or_default(), message);
            }
        }
    }

    #[tokio::test]
    async fn a_host_out_of_reach_is_a_typed_error() {
        for point in points() {
            let (code, message) = me(&point, json!("{\"key\":\"k\",\"token\":\"t\"}")).await;
            assert_eq!(code, code::http(UNREACHABLE), "{point}: {message}");
            assert!(message.contains(" injoignable : "), "{point}: {message}");
        }
    }

    #[tokio::test]
    async fn without_an_account_nothing_is_sent() {
        // No token: refused at once by a 401, which the core reads « connect the account again ». Sent, the call would
        // have found nothing at NOWHERE (a 503).
        for point in points() {
            let (code, message) = me(&point, Value::Null).await;
            assert_eq!(code, code::http(401), "{point}: {message}");
            assert!(message.contains("aucun jeton"), "{point}: {message}");
        }
    }

    #[tokio::test]
    async fn out_of_reach_is_said_in_both_languages() {
        let failed = || async {
            reqwest::Client::new()
                .get(NOWHERE)
                .send()
                .await
                .unwrap_err()
        };
        let fr = unreached_in(Lang::Fr, "Trello", failed().await).to_string();
        assert!(fr.starts_with("Trello injoignable : "), "{fr}");
        assert!(fr.ends_with("(Réglages › Avancé › Réseau)."), "{fr}");
        let en = unreached_in(Lang::En, "Trello", failed().await);
        assert_eq!(en.downcast_ref::<HttpError>().map(|h| h.status), Some(503));
        let en = en.to_string();
        assert!(en.starts_with("Trello can’t be reached: "), "{en}");
        assert!(en.ends_with("(Settings › Advanced › Network)."), "{en}");
    }
}
