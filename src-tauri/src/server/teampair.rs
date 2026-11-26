//! `escouade-server team pair <serveur> <CODE> [--name <nom>]` (OE-1, `docs/TEAM-SERVER.md` §3.4 and §13): the
//! machine enters the team as a **machine of the team** (the role of service). An administrator shows a short code
//! (Réglages › Équipe › « Inscrire une machine de l'équipe »); it is typed on the machine, which plays its one try
//! of the PAKE on the team server's relay, gets the invitation (under the role of service) and asks to enter; the
//! command prints the six words of its request, which the administrator compares before « Admettre ».
//!
//! The code goes from the administrator to the machine, as for « Ajouter un appareil »: the relay lets only a device
//! of the team open a slot (SR2-05), and the machine is none yet.
//!
//! The server holds the data folder: the command, another process, leaves its request in `teams/team-pair.json`
//! (readable by the service's user alone), which the server takes at once (it removes it before it plays it) and
//! answers in `teams/team-pair.answer.json` (the six words, or why not). Neither the code nor the words go to the log.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

use crate::teams::Teams;

pub const REQUEST: &str = "team-pair.json";
pub const ANSWER: &str = "team-pair.answer.json";

/// How long the command waits for the server's answer: the PAKE waits for the administrator's device.
pub const WAIT: Duration = Duration::from_secs(150);

/// What the command asks the server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    pub server: String,
    pub code: String,
    pub name: String,
    /// Its id: the answer names it, so that an answer of before is never taken for this one.
    pub id: String,
}

/// What the server answers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Answer {
    pub id: String,
    /// The team the machine asked to enter (its name, as the server gives it).
    pub team: Option<String>,
    /// The six words of its request, for the administrator to compare.
    pub words: Vec<String>,
    pub error: Option<String>,
}

fn folder(root: &Path) -> PathBuf {
    root.join("teams")
}

/// Writes `bytes` at `path` through a file beside it, readable by this user alone.
fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    {
        use std::io::Write;
        let mut file = options.open(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// The command's side: leaves `request` for the server of the folder `root`.
pub fn ask(root: &Path, request: &Request) -> Result<()> {
    let _ = std::fs::remove_file(folder(root).join(ANSWER));
    write_private(&folder(root).join(REQUEST), &serde_json::to_vec(request)?)
}

/// The command's side: waits up to `wait` for the answer to the request `id`, and removes it.
pub fn wait(root: &Path, id: &str, wait: Duration) -> Result<Answer> {
    let path = folder(root).join(ANSWER);
    let until = Instant::now() + wait;
    loop {
        if let Ok(bytes) = std::fs::read(&path) {
            if let Ok(answer) = serde_json::from_slice::<Answer>(&bytes) {
                if answer.id == id {
                    let _ = std::fs::remove_file(&path);
                    return Ok(answer);
                }
            }
        }
        if Instant::now() >= until {
            let _ = std::fs::remove_file(folder(root).join(REQUEST));
            return Err(anyhow!(tr!(
                "le serveur n’a pas répondu à temps : tourne-t-il sur ce dossier ? Le code est-il encore affiché chez l’administrateur ?",
                "the server did not answer in time: does it run on this folder? Is the code still shown on the administrator's device?"
            )));
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

/// The server's side: takes the request left in `root`, if any (removed before it is played: one try), joins the team
/// by its code, and leaves the answer. Whether there was one.
pub async fn serve_once(teams: &Teams, root: &Path) -> bool {
    let path = folder(root).join(REQUEST);
    let Ok(bytes) = std::fs::read(&path) else {
        return false;
    };
    let _ = std::fs::remove_file(&path);
    let Ok(request) = serde_json::from_slice::<Request>(&bytes) else {
        log::warn!("escouade-server: a request of « team pair » that does not read, dropped");
        return true;
    };
    let answer = match teams
        .pair_join(&request.server, &request.code, &request.name)
        .await
    {
        Ok(view) => {
            log::info!(
                "escouade-server: asked to enter the team « {} » as a machine of the team",
                view.name
            );
            Answer {
                id: request.id.clone(),
                team: Some(view.name),
                words: view.join_words,
                error: None,
            }
        }
        Err(e) => Answer {
            id: request.id.clone(),
            team: None,
            words: Vec::new(),
            error: Some(e),
        },
    };
    if let Err(e) = write_private(
        &folder(root).join(ANSWER),
        &serde_json::to_vec(&answer).unwrap_or_default(),
    ) {
        log::warn!("escouade-server: the answer of « team pair » not written: {e:#}");
    }
    true
}

/// The server's side: looks for a request every second, for as long as the server runs.
pub fn watch(teams: Arc<Teams>, root: PathBuf) {
    tauri::async_runtime::spawn(async move {
        loop {
            serve_once(&teams, &root).await;
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });
}
