//! The live broadcast by WebSocket: a subscribed device receives the new entries of its team's streams (the team's
//! and every project's) as they are numbered. A device that falls behind receives `lagged`, and reads again from its
//! last number (`GET …/stream?since=n`). The socket closes when the session ends (4001) or the device is revoked
//! (4003). In memory, for one server process.

use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use axum::extract::ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use tokio::sync::broadcast;

use crate::auth::Authed;
use crate::db::Entry;
use crate::metrics::Metrics;
use crate::transport::{cbor, Problem};
use crate::util::Id;
use crate::Shared;

pub const CLOSE_SESSION_ENDED: u16 = 4001;
pub const CLOSE_REVOKED: u16 = 4003;

#[derive(Debug)]
pub enum Event {
    Team(Entry),
    Project(Id, Entry),
    Revoked(Id),
}

/// A frame to a device.
#[derive(Serialize)]
pub struct Frame<'a> {
    /// `team`, `project` or `lagged`.
    pub stream: &'static str,
    #[serde(skip_serializing_if = "Option::is_none", with = "opt_bytes")]
    pub project: Option<&'a [u8]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry: Option<&'a Entry>,
}

mod opt_bytes {
    pub fn serialize<S: serde::Serializer>(v: &Option<&[u8]>, s: S) -> Result<S::Ok, S::Error> {
        match v {
            Some(b) => serde_bytes::serialize(*b, s),
            None => s.serialize_none(),
        }
    }
}

#[derive(Default)]
pub struct Hub {
    teams: Mutex<HashMap<Id, broadcast::Sender<Arc<Event>>>>,
    per_device: Mutex<HashMap<Id, u32>>,
}

impl Hub {
    pub fn publish(&self, team: &Id, event: Event) {
        if let Some(tx) = self
            .teams
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(team)
        {
            let _ = tx.send(Arc::new(event));
        }
    }

    fn subscribe(&self, team: Id) -> broadcast::Receiver<Arc<Event>> {
        self.teams
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .entry(team)
            .or_insert_with(|| broadcast::channel(256).0)
            .subscribe()
    }

    /// Forgets the teams nobody listens to.
    pub fn sweep(&self) {
        self.teams
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|_, tx| tx.receiver_count() > 0);
    }

    fn enter(&self, device: Id, max: u32) -> bool {
        let mut map = self.per_device.lock().unwrap_or_else(|e| e.into_inner());
        let n = map.entry(device).or_default();
        if *n >= max {
            return false;
        }
        *n += 1;
        true
    }

    fn leave(&self, device: &Id) {
        let mut map = self.per_device.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(n) = map.get_mut(device) {
            *n -= 1;
            if *n == 0 {
                map.remove(device);
            }
        }
    }
}

pub async fn upgrade(State(app): State<Shared>, authed: Authed, ws: WebSocketUpgrade) -> Response {
    if !app
        .hub
        .enter(authed.device.id, app.limits.max_live_per_device)
    {
        return Problem::new(
            StatusCode::TOO_MANY_REQUESTS,
            "too_many_live",
            "too many live connections for this device",
        )
        .into_response();
    }
    let seat = Seat {
        app: app.clone(),
        device: authed.device.id,
    };
    // The client sends nothing but pings and a close: its frames stay small.
    ws.max_message_size(16 << 10)
        .max_frame_size(16 << 10)
        .on_upgrade(move |socket| async move {
            let _seat = seat;
            app.metrics.live_connections.fetch_add(1, Ordering::Relaxed);
            run(&app, socket, authed).await;
            app.metrics.live_connections.fetch_sub(1, Ordering::Relaxed);
        })
        .into_response()
}

/// A device's place among its live connections, given back when the connection ends or never opens.
struct Seat {
    app: Shared,
    device: Id,
}

impl Drop for Seat {
    fn drop(&mut self) {
        self.app.hub.leave(&self.device);
    }
}

async fn close(socket: &mut WebSocket, code: u16, reason: &'static str) {
    let _ = socket
        .send(Message::Close(Some(CloseFrame {
            code,
            reason: reason.into(),
        })))
        .await;
}

async fn run(app: &Shared, mut socket: WebSocket, authed: Authed) {
    let me = authed.device.id;
    let mut rx = app.hub.subscribe(authed.device.team);
    let deadline = tokio::time::Instant::from_std(authed.expires);
    loop {
        tokio::select! {
            event = rx.recv() => {
                let bytes = match event {
                    Ok(event) => match &*event {
                        Event::Revoked(d) if *d == me => {
                            close(&mut socket, CLOSE_REVOKED, "revoked").await;
                            return;
                        }
                        Event::Revoked(_) => continue,
                        Event::Team(entry) => cbor::encode(&Frame { stream: "team", project: None, entry: Some(entry) }),
                        Event::Project(p, entry) => {
                            cbor::encode(&Frame { stream: "project", project: Some(p), entry: Some(entry) })
                        }
                    },
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        cbor::encode(&Frame { stream: "lagged", project: None, entry: None })
                    }
                    Err(broadcast::error::RecvError::Closed) => return,
                };
                if socket.send(Message::Binary(bytes.into())).await.is_err() {
                    return;
                }
                Metrics::inc(&app.metrics.live_frames);
            }
            incoming = socket.recv() => match incoming {
                None | Some(Err(_)) | Some(Ok(Message::Close(_))) => return,
                Some(Ok(_)) => {}
            },
            _ = tokio::time::sleep_until(deadline) => {
                close(&mut socket, CLOSE_SESSION_ENDED, "session ended").await;
                return;
            }
        }
    }
}
