//! The first words with a server through its tunnel (ESC-537): the handshake that says whether Escouade answers there,
//! its version and whether this computer's token holds; the pairing by a code; the forgetting (`Unpair`). Each on a
//! WebSocket of its own, closed after its answer: the window then opens its own (`src/lib/transport.ts`).
//!
//! What the server answers is data: shown as text, never run.

// A refusal carries what the window says of it (the versions, a revocation): built rarely, once per step of the dialog,
// never on a path where its size matters.
#![allow(clippy::result_large_err)]

use crate::remoteapi::proto::{self, Machine, Revoked, ToClient, ToServer};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use std::time::Duration;
use tokio_tungstenite::tungstenite::Message;

/// How long a server has to answer.
pub const ANSWER_WAIT: Duration = Duration::from_secs(20);

/// What the handshake said, passed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Welcome {
    pub version: String,
    pub device: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub machine: Option<Machine>,
    pub secrets_in_file: bool,
}

/// What the server refused, or why it did not answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Refusal {
    /// Nothing answered as Escouade at the end of the tunnel (the WebSocket did not open, or closed without a word).
    NoEscouade { detail: String },
    /// The server refused (`remoteapi::codes`).
    Refused {
        code: String,
        error: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        server_version: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        update: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        revoked: Option<Revoked>,
        /// For the server to update: how (ESC-515).
        #[serde(skip_serializing_if = "Option::is_none")]
        server_update: Option<proto::ServerUpdate>,
    },
}

type Ws = tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>;

async fn open(stream: tokio::net::TcpStream, port: u16) -> Result<Ws, Refusal> {
    let url = format!("ws://127.0.0.1:{port}{}", proto::PATH);
    match tokio::time::timeout(ANSWER_WAIT, tokio_tungstenite::client_async(url, stream)).await {
        Ok(Ok((ws, _))) => Ok(ws),
        Ok(Err(e)) => Err(Refusal::NoEscouade {
            detail: e.to_string(),
        }),
        Err(_) => Err(Refusal::NoEscouade {
            detail: tr!("pas de réponse à temps", "no answer in time"),
        }),
    }
}

/// `message` sent first, the first answer of the protocol read; the socket then closed.
async fn ask(
    stream: tokio::net::TcpStream,
    port: u16,
    message: &ToServer,
) -> Result<ToClient, Refusal> {
    let mut ws = open(stream, port).await?;
    let text = serde_json::to_string(message).map_err(|e| Refusal::NoEscouade {
        detail: e.to_string(),
    })?;
    ws.send(Message::text(text))
        .await
        .map_err(|e| Refusal::NoEscouade {
            detail: e.to_string(),
        })?;
    let answer = loop {
        match tokio::time::timeout(ANSWER_WAIT, ws.next()).await {
            Ok(Some(Ok(Message::Text(t)))) => match serde_json::from_str::<ToClient>(&t) {
                Ok(m) => break m,
                Err(_) => continue,
            },
            Ok(Some(Ok(Message::Close(_)))) | Ok(None) => {
                return Err(Refusal::NoEscouade {
                    detail: tr!("fermé sans réponse", "closed without an answer"),
                })
            }
            Ok(Some(Ok(_))) => continue,
            Ok(Some(Err(e))) => {
                return Err(Refusal::NoEscouade {
                    detail: e.to_string(),
                })
            }
            Err(_) => {
                return Err(Refusal::NoEscouade {
                    detail: tr!("pas de réponse à temps", "no answer in time"),
                })
            }
        }
    };
    let _ = tokio::time::timeout(Duration::from_secs(2), ws.close(None)).await;
    match answer {
        ToClient::Refused {
            code,
            error,
            server_version,
            update,
            reason,
            revoked,
            server_update,
        } => Err(Refusal::Refused {
            code,
            error,
            server_version,
            update,
            reason,
            revoked,
            server_update,
        }),
        other => Ok(other),
    }
}

/// The handshake with `token` (empty: none yet, to learn whether Escouade answers and its version).
pub async fn hello(
    stream: tokio::net::TcpStream,
    port: u16,
    version: &str,
    token: &str,
) -> Result<Welcome, Refusal> {
    let answer = ask(
        stream,
        port,
        &ToServer::Hello {
            version: version.to_string(),
            token: token.to_string(),
            session: None,
        },
    )
    .await?;
    match answer {
        ToClient::Welcome {
            version,
            device,
            secrets_in_file,
            machine,
            ..
        } => Ok(Welcome {
            version,
            device,
            machine,
            secrets_in_file,
        }),
        _ => Err(Refusal::NoEscouade {
            detail: tr!("réponse inattendue", "unexpected answer"),
        }),
    }
}

/// The pairing by `code`, this computer named `name` there: its device's token.
pub async fn pair(
    stream: tokio::net::TcpStream,
    port: u16,
    code: &str,
    name: &str,
) -> Result<String, Refusal> {
    let answer = ask(
        stream,
        port,
        &ToServer::Pair {
            code: code.to_string(),
            name: name.to_string(),
            key: None,
        },
    )
    .await?;
    match answer {
        ToClient::Paired { token, .. } if !token.is_empty() => Ok(token),
        _ => Err(Refusal::NoEscouade {
            detail: tr!("réponse inattendue", "unexpected answer"),
        }),
    }
}

/// The server asked to put itself at `version` (ESC-702, « Mettre à jour le serveur »), by this computer's token: the
/// version it aims at, or why not (`token`, `scope`, `update`: in the image, without systemd, an older version).
pub async fn update_server(
    stream: tokio::net::TcpStream,
    port: u16,
    token: &str,
    version: &str,
) -> Result<String, Refusal> {
    match ask(
        stream,
        port,
        &ToServer::UpdateServer {
            token: token.to_string(),
            version: version.to_string(),
        },
    )
    .await?
    {
        ToClient::UpdateAsked { to } => Ok(to),
        _ => Err(Refusal::NoEscouade {
            detail: tr!("réponse inattendue", "unexpected answer"),
        }),
    }
}

/// This computer forgotten by the server: its token revoked there.
pub async fn unpair(stream: tokio::net::TcpStream, port: u16, token: &str) -> Result<(), Refusal> {
    match ask(
        stream,
        port,
        &ToServer::Unpair {
            token: token.to_string(),
        },
    )
    .await?
    {
        ToClient::Unpaired {} => Ok(()),
        _ => Err(Refusal::NoEscouade {
            detail: tr!("réponse inattendue", "unexpected answer"),
        }),
    }
}
