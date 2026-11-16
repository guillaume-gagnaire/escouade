//! The screen of the copies (ESC-487): the copies of each file of the data folder that is not made again, what each
//! holds, what taking one back would change, and taking one back.
//!
//! The copies themselves (`escouade_model::backups`) are made at the start and every 10 minutes; here the window lists
//! them, compares one to the file of today (nothing written), makes one now, and takes one back:
//! - a closed project's history at once (nothing holds it in memory), the file of now set aside, the history put back
//!   in the index of the closed projects (« Rouvrir » then gives the project back);
//! - any other file at the next start (the app holds it in memory and would write it over): refused while an agent
//!   works, a restart offered, and at that start the file of now set aside and the copy, read back again, put in place.
//!
//! Nothing that goes out of here holds a value of a field: counts and names (`backup_view`), never a token.

use crate::closed::ClosedEntry;
use crate::core::Core;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use escouade_model::backup_view::{self, Diff, Summary};
use escouade_model::backups::{self, Pending, Restored, SetAside};
use parking_lot::Mutex;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

/// A copy as the screen lists it.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CopyView {
    /// Its name in its folder: what « Comparer » and « Reprendre » name it by.
    pub name: String,
    pub at: DateTime<Utc>,
    pub size: u64,
    pub summary: Summary,
}

/// A file and its copies.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FileCopies {
    /// As named in the data folder (`state.json`, `closed-projects/<id>.json`).
    pub file: String,
    /// The file is there now.
    pub exists: bool,
    /// What it holds now (not readable when it is not there).
    pub today: Summary,
    /// Its folder of copies on the disk.
    pub folder: PathBuf,
    /// Newest first.
    pub copies: Vec<CopyView>,
}

/// A closed project's history the disk no longer has, its copies left: offered to be found again.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LostHistory {
    pub id: String,
    pub file: String,
    pub name: String,
    pub agents: usize,
    pub tickets: usize,
    /// Its newest copy that reads.
    pub copy: String,
    pub at: DateTime<Utc>,
}

/// What the screen shows.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BackupsView {
    /// The data folder, and the folder of the copies in it.
    pub data: PathBuf,
    pub folder: PathBuf,
    /// What all the copies take, and their bound.
    pub total: u64,
    pub max: u64,
    pub files: Vec<FileCopies>,
    /// The copy chosen for the next start.
    pub pending: Option<Pending>,
    /// The files set aside by a start or by a copy taken back.
    pub set_aside: Vec<SetAside>,
    /// The closed projects' histories to find again.
    pub lost: Vec<LostHistory>,
    /// The agents at work now: a copy of a file the app holds is not taken back while there are.
    pub at_work: usize,
}

/// What « Reprendre » did.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RestoreOutcome {
    /// Put in place now (a closed project's history), or at the next start.
    pub done: bool,
    /// A restart puts it in place.
    pub restart: bool,
    /// Where the file of now went (`done`).
    pub set_aside: Option<PathBuf>,
    /// The closed project found again (`done` for a history): its id, for « Rouvrir ».
    pub closed_id: Option<String>,
}

/// What each copy holds, by its path: a copy is never written again (its name is its time), so it is read once.
static SUMMARIES: LazyLock<Mutex<HashMap<PathBuf, (u64, Summary)>>> =
    LazyLock::new(Default::default);

fn summary_of_copy(file: &str, path: &Path, size: u64) -> Summary {
    if let Some((s, sum)) = SUMMARIES.lock().get(path) {
        if *s == size {
            return sum.clone();
        }
    }
    let sum = backups::read_copy(path)
        .map(|b| backup_view::summarize(file, &b))
        .unwrap_or_default();
    SUMMARIES
        .lock()
        .insert(path.to_path_buf(), (size, sum.clone()));
    sum
}

fn today_of(data: &Path, file: &str) -> Option<Vec<u8>> {
    std::fs::read(backups::file_path(data, file)).ok()
}

fn refused(lang: crate::i18n::Lang, e: backups::RestoreError) -> anyhow::Error {
    match e {
        backups::RestoreError::Unknown => anyhow!(tr_in!(
            lang,
            "Cette copie n'existe plus.",
            "This copy no longer exists."
        )),
        backups::RestoreError::Unreadable => anyhow!(tr_in!(
            lang,
            "Cette copie ne se lit pas : elle n'est pas reprise, le fichier d'aujourd'hui reste tel quel.",
            "This copy doesn't read: it isn't taken back, today's file stays as it is."
        )),
        backups::RestoreError::Io(e) => anyhow!(tr_in!(
            lang,
            "La copie n'a pas pu être reprise : {e}",
            "The copy couldn't be taken back: {e}"
        )),
    }
}

impl Core {
    /// The agents at work: in a turn, asking, having their worktree set up, a ticket being validated, work under way
    /// outside a turn, or something running in their background.
    pub(crate) fn agents_at_work(&self) -> usize {
        let held = self.held.lock().clone();
        let mut n = 0;
        for h in self.agents.read().values() {
            let rt = h.lock();
            if rt.meta.archived {
                continue;
            }
            let background =
                rt.proc.is_some() && rt.meta.plan.as_ref().is_some_and(|p| p.running_count() > 0);
            let busy = rt.meta.status.is_active()
                || held.contains(&rt.meta.id)
                || rt.setup.is_some()
                || background;
            n += usize::from(busy);
        }
        let validations = self
            .tickets
            .read()
            .iter()
            .filter(|t| t.step.is_some())
            .count();
        n + validations + self.works.load(std::sync::atomic::Ordering::Acquire)
    }

    /// The screen of the copies.
    pub fn backups_view(&self) -> BackupsView {
        let data = self.data.root();
        let files: Vec<FileCopies> = backups::known(data)
            .into_iter()
            .map(|file| {
                let today = today_of(data, &file);
                FileCopies {
                    exists: today.is_some(),
                    today: today
                        .map(|b| backup_view::summarize(&file, &b))
                        .unwrap_or_default(),
                    folder: backups::copies_dir(data, &file),
                    copies: backups::list(data, &file)
                        .into_iter()
                        .map(|(p, c)| CopyView {
                            name: p
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default(),
                            at: c.at,
                            size: c.size,
                            summary: summary_of_copy(&file, &p, c.size),
                        })
                        .collect(),
                    file,
                }
            })
            .collect();
        let open: Vec<String> = self.projects.read().iter().map(|p| p.id.clone()).collect();
        let indexed: Vec<String> = self.closed_index.lock().keys().cloned().collect();
        let lost = files
            .iter()
            .filter(|f| !f.exists)
            .filter_map(|f| {
                let id = f
                    .file
                    .strip_prefix("closed-projects/")?
                    .strip_suffix(".json")?;
                if open.iter().any(|o| o == id) || indexed.iter().any(|i| i == id) {
                    return None;
                }
                let c = f.copies.iter().find(|c| c.summary.readable)?;
                Some(LostHistory {
                    id: id.to_string(),
                    file: f.file.clone(),
                    name: c.summary.name.clone().unwrap_or_default(),
                    agents: c.summary.agents.unwrap_or(0),
                    tickets: c.summary.tickets.unwrap_or(0),
                    copy: c.name.clone(),
                    at: c.at,
                })
            })
            .collect();
        BackupsView {
            data: data.to_path_buf(),
            folder: data.join(backups::DIR),
            total: files
                .iter()
                .flat_map(|f| f.copies.iter().map(|c| c.size))
                .sum(),
            max: backups::TOTAL_MAX,
            files,
            pending: backups::pending(data),
            set_aside: backups::set_aside_files(data),
            lost,
            at_work: self.agents_at_work(),
        }
    }

    /// « Comparer »: what taking back the copy `name` of `file` gains and loses against the file of today. Nothing is
    /// written.
    pub fn compare_backup(&self, file: &str, name: &str) -> Result<Diff> {
        let data = self.data.root();
        let lang = self.lang().ui;
        let (path, _) = backups::find_copy(data, file, name)
            .ok_or_else(|| refused(lang, backups::RestoreError::Unknown))?;
        let copy = backups::read_copy(&path).unwrap_or_default();
        let today = today_of(data, file).unwrap_or_default();
        Ok(backup_view::diff(file, &copy, &today))
    }

    /// « Faire une copie maintenant »: a copy of each file that changed since its newest.
    pub fn back_up_now(&self) {
        backups::back_up_all(self.data.root(), Utc::now());
    }

    /// « Reprendre » the copy `name` of `file`. A closed project's history is put back at once (refused while its
    /// project is open); any other file is chosen for the next start, refused while an agent works.
    pub fn restore_backup(&self, file: &str, name: &str) -> Result<RestoreOutcome> {
        let data = self.data.root();
        let lang = self.lang().ui;
        if let Some(id) = file
            .strip_prefix("closed-projects/")
            .and_then(|f| f.strip_suffix(".json"))
        {
            if self.projects.read().iter().any(|p| p.id == id) || self.is_closing(id) {
                return Err(anyhow!(tr_in!(
                    lang,
                    "Ce projet est ouvert : son historique de projet fermé ne se reprend pas.",
                    "This project is open: its closed project's history can't be taken back."
                )));
            }
            let aside =
                backups::restore(data, file, name, Utc::now()).map_err(|e| refused(lang, e))?;
            match crate::closed::read_history(&self.data, id) {
                Some(closed) => {
                    let entry = ClosedEntry::of(&closed, &self.data);
                    self.closed_index.lock().insert(entry.id.clone(), entry);
                }
                None => log::warn!("{file}: taken back, but does not read as its project's"),
            }
            return Ok(RestoreOutcome {
                done: true,
                restart: false,
                set_aside: aside,
                closed_id: Some(id.to_string()),
            });
        }
        let busy = self.agents_at_work();
        if busy > 0 {
            return Err(anyhow!(at_work(lang, busy)));
        }
        backups::plan(data, file, name, Utc::now()).map_err(|e| refused(lang, e))?;
        log::warn!("{file}: its copy {name} chosen, put in place at the next start");
        Ok(RestoreOutcome {
            done: false,
            restart: true,
            set_aside: None,
            closed_id: None,
        })
    }

    /// The copy chosen for the next start called off.
    pub fn cancel_restore(&self) -> Result<()> {
        backups::cancel(self.data.root())?;
        log::info!("the copy chosen for the next start is called off");
        Ok(())
    }

    /// The copies taken back by hand at the last starts, until the window's notice closes.
    pub fn restored_backups(&self) -> Vec<Restored> {
        backups::restored(self.data.root())
    }
}

/// Refused: `n` agents work, a restart would cut them.
pub(crate) fn at_work(lang: crate::i18n::Lang, n: usize) -> String {
    tr_in!(
        lang,
        "{n} agent(s) au travail : une reprise redémarre Escouade et couperait leur tour. Attends la fin de leur travail ou arrête-les, puis reprends la copie.",
        "{n} agent(s) at work: taking a copy back restarts Escouade and would cut their turn. Wait for their work to end or stop them, then take the copy back."
    )
}

#[cfg(test)]
#[path = "copies_tests.rs"]
mod tests;
