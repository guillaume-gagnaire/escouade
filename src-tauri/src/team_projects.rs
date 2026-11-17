//! « Projets de l'équipe » (TS-8, `docs/TEAM-SERVER.md` §5): the team's list of projects next to the projects of this
//! device, and « Cloner ».
//!
//! - **A project already here** is known by the address of a remote of one of its repositories
//!   (`escouade_team::catalog::key`: https and ssh, with or without `.git`, the host in any case). It is attached to the
//!   team (its `team`) by the member's gesture, never by itself, and its name and logo stay its own: when the list's
//!   differ, the window says so.
//! - **« Cloner »** goes through the member's own accounts on the host of code (git's credentials, its ssh keys): the
//!   team server gives no access to git and keeps no credential of it; nothing of the clone goes to it. Each address
//!   is checked again here, from the list (never from the window), before git sees it, and given after `--`.
//! - **Without access**, git's refusal is said as « Demande l'accès à acme/api sur GitHub »; a project of several
//!   repositories clones them all and says which one is missing.

use std::path::{Path, PathBuf};
use std::time::Duration;

#[cfg(test)]
use escouade_team::catalog::Catalog;
use escouade_team::catalog::{self, Access, Address, Item, Listed};
use serde::Serialize;

use crate::model::Project;

/// How long a clone may take.
const CLONE_TIMEOUT: Duration = Duration::from_secs(30 * 60);

/// A repository of a project of the list, as the window shows it before the clone: its whole address.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RepoView {
    pub url: String,
    pub folder: String,
    pub host: String,
    pub path: String,
    /// GitHub, GitLab… when the host is one Escouade knows.
    pub forge: Option<String>,
    /// `https` or `ssh`.
    pub scheme: String,
}

/// The project of this device that is a project of the list.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Here {
    pub project: String,
    pub name: String,
    /// It is a project of this team already (its `team`).
    pub attached: bool,
    /// It is a project of another team.
    pub other_team: bool,
    /// The repositories of the list found in it, by index.
    pub repos: Vec<usize>,
    /// The list names it otherwise: its own name stays, said.
    pub name_differs: bool,
    /// Its Kanban is the one the list names (TS-7).
    pub board_joined: bool,
    /// Its Kanban is a team's, another than the one the list names.
    pub board_other: bool,
}

/// A Kanban a member proposed for a project of the list (TS-7), shown to an administrator.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProposedView {
    pub board: String,
    /// The member who proposed it (hex).
    pub by: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TeamProjectView {
    pub id: String,
    pub name: String,
    pub color: String,
    pub logo: Option<String>,
    pub branch: String,
    pub access: Access,
    pub repos: Vec<RepoView>,
    /// The administrator who wrote it last.
    pub by: String,
    /// The member of this device is among those who have access.
    pub open_to_me: bool,
    pub here: Option<Here>,
    /// The journal of its Kanban (TS-7), empty when the list names none.
    pub board: String,
    /// A Kanban a member proposed for it, other than the list's: shown to an administrator.
    pub proposed: Option<ProposedView>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TeamProjectsView {
    pub team: String,
    pub name: String,
    pub admin: bool,
    pub me: String,
    pub members: Vec<crate::teams::MemberView>,
    pub projects: Vec<TeamProjectView>,
    /// The projects of this team on this device whose repositories are not in the list: their box stays.
    pub outside: Vec<String>,
    /// The server showed what cannot be: the list is no longer read.
    pub alarm: bool,
    /// Entries refused at the reading (not signed by an administrator, an address that does not pass).
    pub refused: usize,
}

/// A project of this device, with the keys of the addresses of its repositories' remotes.
#[derive(Clone, Debug, Default)]
pub struct Local {
    pub id: String,
    pub name: String,
    pub team: Option<String>,
    pub keys: Vec<String>,
    /// The journal of its Kanban, when it is a team's (TS-6).
    pub board: Option<String>,
}

pub fn repo_view(url: &str, folder: &str) -> RepoView {
    match catalog::address(url) {
        Ok(a) => RepoView {
            url: a.url.clone(),
            folder: folder.to_owned(),
            host: a.host.clone(),
            path: a.path.clone(),
            forge: a.forge.map(|f| f.name().to_owned()),
            scheme: match a.scheme {
                catalog::Scheme::Https => "https",
                catalog::Scheme::Ssh => "ssh",
            }
            .into(),
        },
        Err(_) => RepoView {
            url: url.to_owned(),
            folder: folder.to_owned(),
            host: String::new(),
            path: String::new(),
            forge: None,
            scheme: String::new(),
        },
    }
}

fn view_of(
    item: &Item,
    me: &str,
    team: &str,
    locals: &[Local],
    proposals: &escouade_team::proposals::Proposals,
    admin: bool,
) -> TeamProjectView {
    let p = &item.project;
    let keys: Vec<Option<String>> = p.repos.iter().map(|r| catalog::key(&r.url)).collect();
    let here = locals.iter().find_map(|l| {
        let repos: Vec<usize> = keys
            .iter()
            .enumerate()
            .filter(|(_, k)| k.as_ref().is_some_and(|k| l.keys.contains(k)))
            .map(|(i, _)| i)
            .collect();
        (!repos.is_empty()).then(|| Here {
            project: l.id.clone(),
            name: l.name.clone(),
            attached: l.team.as_deref() == Some(team),
            other_team: l.team.as_deref().is_some_and(|t| t != team),
            repos,
            name_differs: l.name.trim() != p.name.trim(),
            board_joined: !p.board.is_empty() && l.board.as_deref() == Some(p.board.as_str()),
            board_other: l
                .board
                .as_ref()
                .is_some_and(|b| !p.board.is_empty() && *b != p.board),
        })
    });
    let proposed = proposals
        .of(&item.id)
        .filter(|x| admin && x.board != p.board)
        .map(|x| ProposedView {
            board: x.board.clone(),
            by: x.by.clone(),
        });
    let folders = p.folders();
    let me_id = me
        .parse::<HexMember>()
        .map(|m| m.0)
        .unwrap_or(escouade_team::crypto::MemberId([0; 16]));
    TeamProjectView {
        id: item.id.clone(),
        name: p.name.clone(),
        color: p.color.clone(),
        logo: p.logo.clone(),
        branch: p.branch.clone(),
        access: p.access.clone(),
        repos: p
            .repos
            .iter()
            .zip(folders)
            .map(|(r, f)| repo_view(&r.url, &f))
            .collect(),
        by: item.by.clone(),
        open_to_me: p.open_to(&me_id),
        here,
        board: p.board.clone(),
        proposed,
    }
}

struct HexMember(escouade_team::crypto::MemberId);

impl std::str::FromStr for HexMember {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, ()> {
        let mut b = [0u8; 16];
        hex::decode_to_slice(s, &mut b).map_err(|_| ())?;
        Ok(HexMember(escouade_team::crypto::MemberId(b)))
    }
}

/// The list next to the projects of this device: each project of the list with the one here that holds one of its
/// repositories; the projects of the team here that the list does not hold.
pub fn combine(of: crate::teams::CatalogOf, team: &str, locals: &[Local]) -> TeamProjectsView {
    let projects: Vec<TeamProjectView> = of
        .catalog
        .projects
        .iter()
        .map(|i| view_of(i, &of.me, team, locals, &of.proposals, of.admin))
        .collect();
    let outside = locals
        .iter()
        .filter(|l| l.team.as_deref() == Some(team))
        .filter(|l| {
            !projects
                .iter()
                .any(|p| p.here.as_ref().is_some_and(|h| h.project == l.id))
        })
        .map(|l| l.id.clone())
        .collect();
    TeamProjectsView {
        team: team.to_owned(),
        name: of.name,
        admin: of.admin,
        me: of.me,
        members: of.members,
        projects,
        outside,
        alarm: of.catalog.alarm.is_some(),
        refused: of.catalog.refusals.len(),
    }
}

/// The folders of a project's repositories: its own, or each of them (ESC-169).
fn repo_dirs(p: &Project) -> Vec<PathBuf> {
    let root = PathBuf::from(&p.path);
    if p.repos.is_empty() {
        vec![root]
    } else {
        p.repos
            .iter()
            .map(|r| {
                if r.is_empty() {
                    root.clone()
                } else {
                    root.join(r)
                }
            })
            .collect()
    }
}

/// The keys of the addresses of the remotes of a folder's repository (as configured, before any `insteadOf`).
async fn keys_of(dir: &Path) -> Vec<String> {
    let dir = dir.to_string_lossy().to_string();
    let mut out = Vec::new();
    for remote in crate::git::remotes(&dir).await {
        if let Some(url) = crate::git::remote_url(&dir, &remote).await {
            if let Some(k) = catalog::key(&url) {
                out.push(k);
            }
        }
    }
    out
}

/// The projects of this device, with the keys of their repositories' remotes.
pub async fn locals(projects: Vec<Project>) -> Vec<Local> {
    let reads = projects.into_iter().map(|p| async move {
        let mut keys = Vec::new();
        for d in repo_dirs(&p) {
            keys.extend(keys_of(&d).await);
        }
        Local {
            id: p.id,
            name: p.name,
            team: p.team,
            keys,
            board: p.team_project,
        }
    });
    futures_util::future::join_all(reads).await
}

// === « Cloner » ===

/// A repository of the clone, done or not.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RepoClone {
    pub url: String,
    /// Its folder (the project's own, for a project of one repository).
    pub path: String,
    pub done: bool,
    /// It was there already (a clone tried again).
    pub already: bool,
    /// git says the member has no access: « Demande l'accès à acme/api sur GitHub ».
    pub no_access: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CloneView {
    /// The project's folder: what the wizard goes on with.
    pub path: String,
    pub repos: Vec<RepoClone>,
    /// Every repository is there.
    pub complete: bool,
}

/// git's refusal of a repository the member has no access to (or that does not exist for them: a private repository
/// of GitHub answers « not found »).
pub fn no_access_in(stderr: &str) -> bool {
    let s = stderr.to_ascii_lowercase();
    // A branch of the list that the repository does not have is no refusal of access.
    if s.contains("remote branch") {
        return false;
    }
    [
        "repository not found",
        "not found",
        "authentication failed",
        "permission denied",
        "could not read username",
        "could not read password",
        "terminal prompts disabled",
        "access denied",
        "403",
        "401",
        "could not read from remote repository",
        "you do not have permission",
        "unauthorized",
        "host key verification failed",
    ]
    .iter()
    .any(|w| s.contains(w))
}

/// What the wizard says of a repository the member cannot reach.
pub fn ask_access(a: &Address) -> String {
    let host = a
        .forge
        .map(|f| f.name().to_owned())
        .unwrap_or_else(|| a.host.clone());
    let path = &a.path;
    tr!(
        "Demande l’accès à {path} sur {host}",
        "Ask for access to {path} on {host}"
    )
}

fn folder_ok(f: &str) -> bool {
    !f.is_empty()
        && f.len() <= 120
        && f != "."
        && f != ".."
        && !f.starts_with('-')
        && !f
            .chars()
            .any(|c| "/\\:*?\"<>|".contains(c) || c.is_control())
        && f.trim() == f
}

fn empty_dir(p: &Path) -> bool {
    std::fs::read_dir(p).is_ok_and(|mut d| d.next().is_none())
}

/// Clones one repository, checked again, into `dest`: the member's credentials, a time limit, the address after
/// `--`. `config` is given to git with `-c` (the tests' `insteadOf`).
async fn clone_one(
    a: &Address,
    branch: &str,
    dest: &Path,
    config: &[String],
) -> Result<(), (bool, String)> {
    let parent = dest.parent().unwrap_or(dest).to_string_lossy().to_string();
    let dest_s = dest.to_string_lossy().to_string();
    let mut args: Vec<&str> = Vec::new();
    for c in config {
        args.push("-c");
        args.push(c);
    }
    args.extend(["clone", "--origin", "origin"]);
    if !branch.is_empty() {
        args.extend(["--branch", branch]);
    }
    args.extend(["--", a.url.as_str(), dest_s.as_str()]);
    let mut cmd = crate::git::in_english(crate::git::command(&parent, &args));
    cmd.kill_on_drop(true);
    let out = match escouade_model::proc::output_within(cmd, CLONE_TIMEOUT).await {
        Ok(o) => o,
        Err(e) => return Err((false, e.to_string())),
    };
    if out.status.success() {
        return Ok(());
    }
    let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
    // A clone stopped halfway leaves its folder: it goes, so that « Réessayer » finds the place free.
    if dest.exists() && !dest.join(".git").join("HEAD").exists() {
        let _ = std::fs::remove_dir_all(dest);
    }
    Err((no_access_in(&err), err))
}

/// The keys of the remotes of a folder that is a clone already.
async fn same_clone(dest: &Path, a: &Address) -> bool {
    dest.join(".git").exists() && keys_of(dest).await.contains(&a.key())
}

/// « Cloner »: each repository of the project of the list into `parent/folder` (a project of one repository is its
/// clone; one of several holds each in its folder), by the member's own accounts. A repository there already (the
/// same remote) is kept: « Réessayer » clones only what is missing.
pub async fn clone_listed(
    listed: &Listed,
    parent: &Path,
    folder: &str,
    config: &[String],
) -> Result<CloneView, String> {
    catalog::check(listed).map_err(|why| {
        tr!(
            "Ce projet de la liste ne passe pas : {why}",
            "This project of the list does not pass: {why}"
        )
    })?;
    let folder = folder.trim();
    if !folder_ok(folder) {
        return Err(tr!(
            "« {folder} » n’est pas un nom de dossier.",
            "“{folder}” is not a folder name."
        ));
    }
    if !parent.is_dir() {
        let p = parent.display();
        return Err(tr!("{p} n’est pas un dossier.", "{p} is not a folder."));
    }
    let root = parent.join(folder);
    let several = listed.repos.len() > 1;
    let addresses: Vec<Address> = listed
        .repos
        .iter()
        .map(|r| catalog::address(&r.url).expect("checked"))
        .collect();
    if root.exists() {
        let ours = if several {
            root.is_dir()
        } else {
            empty_dir(&root) || same_clone(&root, &addresses[0]).await
        };
        if !ours {
            let p = root.display();
            return Err(tr!(
                "{p} existe déjà : choisis un autre dossier.",
                "{p} exists already: choose another folder."
            ));
        }
    }
    if several {
        std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    }
    let mut repos = Vec::new();
    for (a, f) in addresses.iter().zip(listed.folders()) {
        let dest = if several { root.join(&f) } else { root.clone() };
        let mut r = RepoClone {
            url: a.url.clone(),
            path: dest.to_string_lossy().to_string(),
            done: false,
            already: false,
            no_access: None,
            error: None,
        };
        if same_clone(&dest, a).await {
            r.done = true;
            r.already = true;
        } else if dest.exists() && !empty_dir(&dest) {
            let p = dest.display();
            r.error = Some(tr!(
                "{p} existe déjà et n’est pas ce dépôt.",
                "{p} exists already and is not this repository."
            ));
        } else {
            if dest.exists() {
                let _ = std::fs::remove_dir(&dest);
            }
            match clone_one(a, &listed.branch, &dest, config).await {
                Ok(()) => r.done = true,
                Err((true, e)) => {
                    log::info!("team projects: no access to {}: {e}", a.url);
                    r.no_access = Some(ask_access(a));
                }
                Err((false, e)) => r.error = Some(e),
            }
        }
        repos.push(r);
    }
    Ok(CloneView {
        path: root.to_string_lossy().to_string(),
        complete: repos.iter().all(|r| r.done),
        repos,
    })
}

/// The logo of a project of the list, written where the wizard takes it from (a file, as « Choisir une image… »).
pub fn logo_file(dir: &Path, id: &str, logo: &str) -> Option<PathBuf> {
    use base64::Engine as _;
    let (ext, data) = [
        ("png", "data:image/png;base64,"),
        ("jpg", "data:image/jpeg;base64,"),
        ("webp", "data:image/webp;base64,"),
    ]
    .iter()
    .find_map(|(e, p)| logo.strip_prefix(p).map(|d| (*e, d)))?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
        .ok()?;
    std::fs::create_dir_all(dir).ok()?;
    let safe: String = id.chars().filter(char::is_ascii_hexdigit).collect();
    let file = dir.join(format!("team-logo-{safe}.{ext}"));
    std::fs::write(&file, bytes).ok()?;
    Some(file)
}

#[cfg(test)]
#[path = "team_projects_tests.rs"]
mod tests;
