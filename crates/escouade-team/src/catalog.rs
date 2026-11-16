//! The team's projects, clonable in one click (TS-8, `docs/TEAM-SERVER.md` §5): the list of the team's projects,
//! encrypted under the team's key, kept by its administrators, that the server does not read.
//!
//! - **Where it lives**: a stream of the server of its own ([`CATALOG_STREAM`], a project id no client draws), as the
//!   team's settings (TS-13). Each entry is an object sealed under the team's key of its epoch and signed by the leaf
//!   of its author: [`Entry::Put`] (a project added or changed, whole) or [`Entry::Remove`]. The server sees that an
//!   entry was written, its size and its hour: never a name, an address or a branch (§3.8).
//! - **Who writes**: an administrator. The client refuses a member's write before it leaves; the server refuses an
//!   entry of a member in this stream (defense in depth: it knows the roles); and every device refuses, at the
//!   reading, an entry sealed by a member who was not an administrator at its epoch (or, for an epoch this device did
//!   not live, is not one now). The administrators still there write again the projects of an administrator who left
//!   ([`serve`]), so that a device that comes later reads them.
//! - **What comes from the team is not trusted blindly**: an address of a repository is `https://` or ssh, nothing
//!   else ([`address`]: no `file://`, no option of git slipped in an address, no local path, no credential in it); a
//!   project whose address, folder, branch or logo does not pass is refused whole, at the writing as at the reading.
//! - **A clone already on the device** is known by the address of its repository ([`key`]): https and ssh, with or
//!   without `.git`, the host in any case, are the same address.
//! - **A lying server**: the device reads again the last entry it read before reading on. Gone, it is a rollback;
//!   changed, a rewritten entry: the reading stops and says it (`settings::Alarm`).

use std::collections::{BTreeMap, BTreeSet};

use escouade_team_crypto::content::SealedObject;
use escouade_team_crypto::{MemberId, ObjectId, ProjectId, Role};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub use crate::settings::Alarm;
use crate::team::Team;
use crate::{wire, CryptoError, Error};

/// The stream of the team's list of projects: a project id no client draws (16 bytes). The server knows it, to keep
/// its writes to the administrators.
pub const CATALOG_STREAM: ProjectId = ProjectId(*b"escouade/project");

/// Where this device keeps what it read of the stream, in the team's encrypted base.
const STATE: &str = "catalog";

/// The most repositories a project of the list holds.
pub const MAX_REPOS: usize = 16;
/// The longest address of a repository.
pub const MAX_ADDRESS: usize = 512;
/// The largest logo (its data URL).
pub const MAX_LOGO: usize = 256 * 1024;

// === The addresses ===

/// How git reaches a repository of the list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Scheme {
    Https,
    /// `ssh://` or `user@host:path`.
    Ssh,
}

/// The host of code an address is on, when it is one Escouade knows (the wizard names it: « sur GitHub »).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Forge {
    Github,
    Gitlab,
    Bitbucket,
    AzureDevops,
    Codeberg,
}

impl Forge {
    pub fn name(self) -> &'static str {
        match self {
            Forge::Github => "GitHub",
            Forge::Gitlab => "GitLab",
            Forge::Bitbucket => "Bitbucket",
            Forge::AzureDevops => "Azure DevOps",
            Forge::Codeberg => "Codeberg",
        }
    }

    fn of(host: &str) -> Option<Forge> {
        let h = host.strip_prefix("ssh.").unwrap_or(host);
        Some(match h {
            "github.com" => Forge::Github,
            "gitlab.com" => Forge::Gitlab,
            "bitbucket.org" => Forge::Bitbucket,
            "dev.azure.com" | "vs-ssh.visualstudio.com" => Forge::AzureDevops,
            "codeberg.org" => Forge::Codeberg,
            _ => return None,
        })
    }

    /// Its repositories' paths do not depend on the case (`Acme/API` is `acme/api`).
    fn caseless(self) -> bool {
        true
    }
}

/// An address of a repository that passed: what git is given, as written, and what it names.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Address {
    /// The address as written (trimmed): the one shown before the clone, the one git is given.
    pub url: String,
    pub scheme: Scheme,
    /// The host, in lower case, without its port nor its user.
    pub host: String,
    /// The path on the host, without the slashes around it nor `.git` (`acme/api`).
    pub path: String,
    pub forge: Option<Forge>,
}

impl Address {
    /// What names the repository whatever its form: the host and the path (in lower case on a host whose paths do
    /// not depend on it).
    pub fn key(&self) -> String {
        let path = if self.forge.is_some_and(Forge::caseless) {
            self.path.to_lowercase()
        } else {
            self.path.clone()
        };
        format!("{}/{}", self.host, path)
    }

    /// The last part of the path, without `.git`: the folder a clone goes to by default.
    pub fn name(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or(&self.path)
    }
}

/// Why an address is refused.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum BadAddress {
    #[error("no address")]
    Empty,
    #[error("an address of more than {MAX_ADDRESS} characters")]
    TooLong,
    #[error("a space or a control character in the address")]
    Blank,
    #[error("an address that starts with a dash: git would read an option")]
    Option,
    #[error("only https:// and ssh are cloned, not {scheme}")]
    Scheme { scheme: String },
    #[error("a local path is not an address of the team")]
    Local,
    #[error("a password or a token in the address: the team's list never holds one")]
    Credential,
    #[error("the address names no host, or a host that is not one")]
    Host,
    #[error("the address names no repository, or a part of its path git would read as an option")]
    Path,
}

fn host_ok(host: &str) -> bool {
    !host.is_empty()
        && host.len() <= 253
        // A single letter is a drive on Windows (`C:repo`): never a host.
        && host.len() > 1
        && host.split('.').all(|label| {
            !label.is_empty()
                && !label.starts_with('-')
                && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        })
}

fn user_ok(user: &str) -> bool {
    !user.is_empty()
        && !user.starts_with('-')
        && user
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
}

/// The path of an address, checked: none of its parts read as an option, no backslash, nothing but the characters of
/// a path; without the slashes around it nor `.git`.
fn path_of(raw: &str) -> Result<String, BadAddress> {
    let p = raw.trim_matches('/');
    let p = p.strip_suffix(".git").unwrap_or(p).trim_end_matches('/');
    let ok = !p.is_empty()
        && p.split('/').all(|part| {
            !part.is_empty()
                && !part.starts_with('-')
                && part != "."
                && part != ".."
                && part
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "._-~%+@:".contains(c))
        });
    if ok {
        Ok(p.to_owned())
    } else {
        Err(BadAddress::Path)
    }
}

/// An address of the team's list, checked (« Ce qui vient de l'équipe n'est pas de confiance aveugle »): `https://`
/// or ssh (`ssh://` or `user@host:path`) only; never `file://`, another transport (`ext::`, `git://`, `http://`), an
/// option of git (`--upload-pack=…`, a part of the path that starts with a dash), a local path, a password or a token.
pub fn address(raw: &str) -> Result<Address, BadAddress> {
    let url = raw.trim();
    if url.is_empty() {
        return Err(BadAddress::Empty);
    }
    if url.len() > MAX_ADDRESS {
        return Err(BadAddress::TooLong);
    }
    if url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(BadAddress::Blank);
    }
    if url.starts_with('-') {
        return Err(BadAddress::Option);
    }
    if url.contains('\\') || url.starts_with('/') || url.starts_with('.') || url.starts_with('~') {
        return Err(BadAddress::Local);
    }
    if let Some((scheme, rest)) = url.split_once("://") {
        let scheme_l = scheme.to_ascii_lowercase();
        let ssh = match scheme_l.as_str() {
            "https" => false,
            "ssh" | "git+ssh" | "ssh+git" => true,
            "file" => return Err(BadAddress::Local),
            _ => return Err(BadAddress::Scheme { scheme: scheme_l }),
        };
        let (authority, path) = rest.split_once('/').ok_or(BadAddress::Path)?;
        let host_port = match authority.rsplit_once('@') {
            Some((user, hp)) => {
                // A password, or a user on https (a token passes as one): a credential in the list.
                if !ssh || user.contains(':') {
                    return Err(BadAddress::Credential);
                }
                if !user_ok(user) {
                    return Err(BadAddress::Host);
                }
                hp
            }
            None => authority,
        };
        let host = match host_port.rsplit_once(':') {
            Some((h, port)) if !port.is_empty() && port.chars().all(|c| c.is_ascii_digit()) => h,
            Some(_) => return Err(BadAddress::Host),
            None => host_port,
        };
        let host = host.to_ascii_lowercase();
        if !host_ok(&host) {
            return Err(BadAddress::Host);
        }
        let path = path_of(path)?;
        return Ok(Address {
            url: url.to_owned(),
            scheme: if ssh { Scheme::Ssh } else { Scheme::Https },
            forge: Forge::of(&host),
            host,
            path,
        });
    }
    // `user@host:path`, git's short form of ssh. Another `scheme::address` is a remote helper (`ext::sh -c …`).
    if url.contains("::") {
        return Err(BadAddress::Scheme {
            scheme: url
                .split("::")
                .next()
                .unwrap_or_default()
                .to_ascii_lowercase(),
        });
    }
    let Some((before, path)) = url.split_once(':') else {
        return Err(BadAddress::Local);
    };
    if before.contains('/') {
        return Err(BadAddress::Local);
    }
    let Some((user, host)) = before.rsplit_once('@') else {
        // `host:path` without a user is a drive (`C:repo`) or a file named with a colon to git on Windows.
        return Err(BadAddress::Local);
    };
    if !user_ok(user) {
        return Err(BadAddress::Host);
    }
    let host = host.to_ascii_lowercase();
    if !host_ok(&host) {
        return Err(BadAddress::Host);
    }
    let path = path_of(path)?;
    Ok(Address {
        url: url.to_owned(),
        scheme: Scheme::Ssh,
        forge: Forge::of(&host),
        host,
        path,
    })
}

/// What names a repository whatever the form of its address (https or ssh, with or without `.git`, the host in any
/// case): None for an address that does not pass (a local path, `file://`…).
pub fn key(raw: &str) -> Option<String> {
    address(raw).ok().map(|a| a.key())
}

// === A project of the list ===

/// A repository of a project of the list.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Repo {
    /// Its address: https or ssh.
    pub url: String,
    /// Its folder in the project, for a project of several repositories (ESC-169); empty: the last part of its
    /// address. A project of one repository is its clone.
    pub folder: String,
}

/// Who has access to a project of the list.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Access {
    /// Every member of the team.
    #[default]
    Team,
    /// These members only (their ids, in hex).
    Members { members: Vec<String> },
}

/// A project of the team's list: what the wizard « Nouveau projet » is filled with.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Listed {
    /// Proposed, changeable on the device.
    pub name: String,
    /// `#rrggbb`, proposed; empty: the device chooses.
    pub color: String,
    /// A data URL (PNG, JPEG or WebP), proposed; none: the tile's letters.
    pub logo: Option<String>,
    pub repos: Vec<Repo>,
    /// Its default branch; empty: the repository's.
    pub branch: String,
    pub access: Access,
}

fn folder_ok(f: &str) -> bool {
    !f.is_empty()
        && f.len() <= 100
        && !f.starts_with('-')
        && !f.starts_with('.')
        && f.chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
}

fn branch_ok(b: &str) -> bool {
    b.len() <= 200
        && !b.starts_with('-')
        && !b.starts_with('/')
        && !b.ends_with('/')
        && !b.ends_with(".lock")
        && !b.contains("..")
        && !b.contains("@{")
        && !b.contains("//")
        && b.chars()
            .all(|c| !c.is_whitespace() && !c.is_control() && !"~^:?*[\\".contains(c))
}

fn logo_ok(logo: &str) -> bool {
    let Some(rest) = [
        "data:image/png;base64,",
        "data:image/jpeg;base64,",
        "data:image/webp;base64,",
    ]
    .iter()
    .find_map(|p| logo.strip_prefix(p)) else {
        return false;
    };
    use base64::Engine as _;
    logo.len() <= MAX_LOGO
        && base64::engine::general_purpose::STANDARD
            .decode(rest)
            .is_ok()
}

impl Listed {
    /// The folder of each repository in the project (its own, else the last part of its address).
    pub fn folders(&self) -> Vec<String> {
        self.repos
            .iter()
            .map(|r| {
                if r.folder.trim().is_empty() {
                    address(&r.url)
                        .map(|a| a.name().to_owned())
                        .unwrap_or_default()
                } else {
                    r.folder.trim().to_owned()
                }
            })
            .collect()
    }

    /// Whether `member` has access to it.
    pub fn open_to(&self, member: &MemberId) -> bool {
        match &self.access {
            Access::Team => true,
            Access::Members { members } => {
                let me = member.to_string();
                members.iter().any(|m| m.eq_ignore_ascii_case(&me))
            }
        }
    }
}

/// Why a project of the list is refused: at the writing, and at the reading of a device.
pub fn check(listed: &Listed) -> Result<(), String> {
    let name = listed.name.trim();
    if name.is_empty() || name.chars().count() > 80 || name.chars().any(char::is_control) {
        return Err("a project of the list has a name, of 80 characters at most".into());
    }
    if listed.repos.is_empty() || listed.repos.len() > MAX_REPOS {
        return Err(format!(
            "a project of the list has one repository at least, {MAX_REPOS} at most"
        ));
    }
    for r in &listed.repos {
        address(&r.url).map_err(|e| format!("{}: {e}", r.url.trim()))?;
    }
    let folders = listed.folders();
    if listed.repos.len() > 1 {
        let mut seen = BTreeSet::new();
        for f in &folders {
            if !folder_ok(f) {
                return Err(format!("{f}: not a folder of a project"));
            }
            if !seen.insert(f.to_lowercase()) {
                return Err(format!("{f}: two repositories in the same folder"));
            }
        }
    }
    let c = listed.color.trim();
    if !c.is_empty()
        && !(c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|x| x.is_ascii_hexdigit()))
    {
        return Err(format!("{c}: a colour is #rrggbb"));
    }
    if let Some(logo) = &listed.logo {
        if !logo_ok(logo) {
            return Err(format!(
                "a logo is a PNG, JPEG or WebP image of {} Kio at most",
                MAX_LOGO / 1024
            ));
        }
    }
    if !branch_ok(listed.branch.trim()) {
        return Err(format!("{}: not a branch", listed.branch.trim()));
    }
    if let Access::Members { members } = &listed.access {
        if members.is_empty()
            || members
                .iter()
                .any(|m| m.len() != 32 || !m.chars().all(|c| c.is_ascii_hexdigit()))
        {
            return Err("access kept to members names their ids".into());
        }
    }
    Ok(())
}

// === The stream ===

/// An entry of the stream, sealed and signed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Entry {
    /// A project added, or changed (whole).
    Put {
        id: [u8; 16],
        project: Listed,
        /// When the administrator wrote it (seconds).
        at: u64,
    },
    Remove {
        id: [u8; 16],
        at: u64,
    },
}

/// Why an entry is refused.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Refused {
    /// Signed by no leaf of the team (unsigned, or forged by the server).
    Unsigned,
    /// Signed by a member who was not an administrator.
    NotAdmin,
    /// A project that does not pass (an address, a folder, a branch, a logo): `why`.
    Bad { why: String },
    /// An entry that does not decode.
    Unreadable { why: String },
}

/// An entry refused at the reading.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Refusal {
    pub seq: u64,
    pub reason: Refused,
}

/// A project of the list, as this device read it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    /// Its id in the list (hex): opaque.
    pub id: String,
    pub project: Listed,
    /// The administrator who wrote it last (member id, hex).
    pub by: String,
    pub at: u64,
    /// The number of its entry in the stream.
    pub seq: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct State {
    cursor: u64,
    /// The last entry read: its number and the digest of its bytes.
    last: Option<(u64, [u8; 32])>,
    items: BTreeMap<String, Item>,
    refusals: Vec<Refusal>,
    alarm: Option<Alarm>,
}

/// What this device knows of the list.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    /// By name.
    pub projects: Vec<Item>,
    pub refusals: Vec<Refusal>,
    pub alarm: Option<Alarm>,
}

impl Catalog {
    pub fn get(&self, id: &str) -> Option<&Item> {
        self.projects.iter().find(|i| i.id == id)
    }

    /// The project of the list whose repositories hold this address (by [`key`]), with the repository's index.
    pub fn holding(&self, url: &str) -> Option<(&Item, usize)> {
        let k = key(url)?;
        self.projects.iter().find_map(|i| {
            i.project
                .repos
                .iter()
                .position(|r| key(&r.url).as_deref() == Some(k.as_str()))
                .map(|n| (i, n))
        })
    }
}

fn catalog_of(state: &State) -> Catalog {
    let mut projects: Vec<Item> = state.items.values().cloned().collect();
    projects.sort_by(|a, b| {
        a.project
            .name
            .to_lowercase()
            .cmp(&b.project.name.to_lowercase())
            .then(a.id.cmp(&b.id))
    });
    Catalog {
        projects,
        refusals: state.refusals.clone(),
        alarm: state.alarm,
    }
}

enum Opened {
    Refused(Refused),
    Fork,
    /// An epoch this device has not reached yet: read again later.
    Later,
}

fn entry_digest(e: &wire::Entry) -> [u8; 32] {
    Sha256::digest(wire::encode(e)).into()
}

fn admins_now(team: &Team) -> BTreeSet<MemberId> {
    team.roster()
        .map(|r| {
            r.members
                .iter()
                .filter(|(_, role)| **role == Role::Admin)
                .map(|(m, _)| *m)
                .collect()
        })
        .unwrap_or_default()
}

/// Opens an entry: its seal, its signer, an administrator at its epoch (or now, for an epoch not lived here), what it
/// says, and the project it writes checked.
fn open(team: &Team, e: &wire::Entry) -> Result<(MemberId, Entry), Opened> {
    let unreadable = |why: String| Opened::Refused(Refused::Unreadable { why });
    let sealed: SealedObject = wire::decode(&e.payload).map_err(unreadable)?;
    if sealed.header.project != CATALOG_STREAM {
        return Err(unreadable("an object of another stream".into()));
    }
    let plain = match team.device().open(&sealed) {
        Ok(p) => p,
        Err(CryptoError::Fork { .. }) => return Err(Opened::Fork),
        Err(CryptoError::Behind(_)) => return Err(Opened::Later),
        Err(_) => return Err(Opened::Refused(Refused::Unsigned)),
    };
    let who = team
        .signer_at(sealed.header.epoch, &sealed.header.author)
        .ok_or(Opened::Refused(Refused::Unsigned))?;
    let admin = match team.admins_at(sealed.header.epoch) {
        Some(then) => then.contains(&who.member),
        None => admins_now(team).contains(&who.member),
    };
    if !admin {
        return Err(Opened::Refused(Refused::NotAdmin));
    }
    let entry: Entry = wire::decode(&plain).map_err(unreadable)?;
    if let Entry::Put { project, .. } = &entry {
        check(project).map_err(|why| Opened::Refused(Refused::Bad { why }))?;
    }
    Ok((who.member, entry))
}

async fn read(team: &Team, state: &mut State) -> Result<(), Error> {
    let client = team.client().clone();
    if let Some((seq, d)) = state.last {
        let page = client
            .read_project(&CATALOG_STREAM.0, seq - 1, Some(1))
            .await?;
        match page.entries.first() {
            Some(e) if e.seq == seq => {
                if entry_digest(e) != d {
                    state.alarm = Some(Alarm::Rewritten);
                    return Ok(());
                }
            }
            _ => {
                state.alarm = Some(Alarm::Rollback);
                return Ok(());
            }
        }
    }
    loop {
        let page = client
            .read_project(&CATALOG_STREAM.0, state.cursor, None)
            .await?;
        for e in &page.entries {
            if e.seq <= state.cursor {
                state.alarm = Some(Alarm::Server);
                return Ok(());
            }
            match open(team, e) {
                Ok((by, Entry::Put { id, project, at })) => {
                    let id = hex::encode(id);
                    state.items.insert(
                        id.clone(),
                        Item {
                            id,
                            project,
                            by: by.to_string(),
                            at,
                            seq: e.seq,
                        },
                    );
                }
                Ok((_, Entry::Remove { id, .. })) => {
                    state.items.remove(&hex::encode(id));
                }
                Err(Opened::Refused(reason)) => state.refusals.push(Refusal { seq: e.seq, reason }),
                Err(Opened::Fork) => {
                    state.alarm = Some(Alarm::Fork);
                    return Ok(());
                }
                Err(Opened::Later) => return Ok(()),
            }
            state.cursor = e.seq;
            state.last = Some((e.seq, entry_digest(e)));
        }
        if !page.more || page.entries.is_empty() {
            return Ok(());
        }
    }
}

fn load(team: &Team) -> Result<State, Error> {
    Ok(team.load_extra(STATE)?.unwrap_or_default())
}

fn save(team: &Team, state: &State) -> Result<(), Error> {
    team.save_extra(STATE, state)
}

/// Reads what is new in the list. Under an alarm, nothing is read until [`retry`].
pub async fn sync(team: &mut Team) -> Result<Catalog, Error> {
    let mut state = load(team)?;
    if state.alarm.is_none() {
        read(team, &mut state).await?;
    }
    save(team, &state)?;
    Ok(catalog_of(&state))
}

/// The list as this device read it last, without reading the server.
pub fn view(team: &Team) -> Result<Catalog, Error> {
    Ok(catalog_of(&load(team)?))
}

/// « Réessayer »: the alarm lifted, the stream read again from where it was.
pub async fn retry(team: &mut Team) -> Result<Catalog, Error> {
    let mut state = load(team)?;
    state.alarm = None;
    save(team, &state)?;
    sync(team).await
}

fn admin(team: &Team) -> Result<(), Error> {
    if team.is_admin()? {
        Ok(())
    } else {
        Err(CryptoError::NotAdmin.into())
    }
}

/// The entry sealed by this device, as it goes to the server: a fresh object id each time (the server does not even
/// learn which entries are about the same project).
pub fn seal(team: &Team, entry: &Entry) -> Result<Vec<u8>, Error> {
    let mut object = [0u8; 16];
    getrandom::fill(&mut object).map_err(|e| Error::Encoding(e.to_string()))?;
    let sealed = team
        .device()
        .seal(CATALOG_STREAM, ObjectId(object), 0, &wire::encode(entry))?;
    Ok(wire::encode(&sealed))
}

async fn append(team: &mut Team, entry: &Entry) -> Result<Catalog, Error> {
    let view = sync(team).await?;
    if view.alarm.is_some() {
        return Err(Error::Refused(
            "the server shows what cannot be on the team's list of projects: nothing is sent"
                .into(),
        ));
    }
    let payload = seal(team, entry)?;
    team.client()
        .append_project(&CATALOG_STREAM.0, &payload)
        .await?;
    sync(team).await
}

/// An administrator adds a project to the list (`id` None: a new one, its id drawn here) or changes one, whole. The
/// project is checked first: an address that is not https or ssh, a folder or a branch that is not one, never leaves.
pub async fn put(
    team: &mut Team,
    id: Option<&str>,
    mut project: Listed,
    at: u64,
) -> Result<(String, Catalog), Error> {
    admin(team)?;
    project.name = project.name.trim().to_owned();
    project.branch = project.branch.trim().to_owned();
    project.color = project.color.trim().to_owned();
    for r in &mut project.repos {
        r.url = r.url.trim().to_owned();
        r.folder = r.folder.trim().to_owned();
    }
    check(&project).map_err(Error::Refused)?;
    let id = match id {
        Some(hex_id) => {
            let mut b = [0u8; 16];
            hex::decode_to_slice(hex_id, &mut b).map_err(|e| Error::Encoding(e.to_string()))?;
            b
        }
        None => {
            let mut b = [0u8; 16];
            getrandom::fill(&mut b).map_err(|e| Error::Encoding(e.to_string()))?;
            b
        }
    };
    let view = append(team, &Entry::Put { id, project, at }).await?;
    Ok((hex::encode(id), view))
}

/// An administrator takes a project out of the list (its clones stay where they are).
pub async fn remove(team: &mut Team, id: &str, at: u64) -> Result<Catalog, Error> {
    admin(team)?;
    let mut b = [0u8; 16];
    hex::decode_to_slice(id, &mut b).map_err(|e| Error::Encoding(e.to_string()))?;
    append(team, &Entry::Remove { id: b, at }).await
}

/// The device of an administrator writes again the projects of the list whose author is no longer an administrator
/// (they left, or lost the role): a device that comes later, which did not live their epoch, reads them. The number
/// written again.
pub async fn serve(team: &mut Team, at: u64) -> Result<usize, Error> {
    if !team.is_admin().unwrap_or(false) {
        return Ok(0);
    }
    let view = view(team)?;
    if view.alarm.is_some() {
        return Ok(0);
    }
    let admins: BTreeSet<String> = admins_now(team).iter().map(|m| m.to_string()).collect();
    let mut done = 0;
    for item in &view.projects {
        if !admins.contains(&item.by) {
            let mut id = [0u8; 16];
            hex::decode_to_slice(&item.id, &mut id).map_err(|e| Error::Encoding(e.to_string()))?;
            let payload = seal(
                team,
                &Entry::Put {
                    id,
                    project: item.project.clone(),
                    at,
                },
            )?;
            team.client()
                .append_project(&CATALOG_STREAM.0, &payload)
                .await?;
            done += 1;
        }
    }
    if done > 0 {
        sync(team).await?;
    }
    Ok(done)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_forms_of_one_address_are_the_same_address() {
        let forms = [
            "https://github.com/acme/api",
            "https://github.com/acme/api.git",
            "https://GitHub.com/acme/api/",
            "https://github.com:443/acme/api.git",
            "ssh://git@github.com/acme/api.git",
            "ssh://git@github.com:22/acme/api",
            "git@github.com:acme/api.git",
            "git@GITHUB.COM:acme/api",
            "git@github.com:/acme/api.git",
            "https://github.com/Acme/API.git",
        ];
        for f in forms {
            assert_eq!(
                key(f).as_deref(),
                Some("github.com/acme/api"),
                "{f} is github.com/acme/api"
            );
        }
        assert_ne!(
            key("https://github.com/acme/web"),
            key("https://github.com/acme/api")
        );
        assert_ne!(
            key("https://gitlab.com/acme/api"),
            key("https://github.com/acme/api")
        );
        // A host of its own keeps the case of its paths.
        assert_ne!(
            key("https://git.acme.fr/Acme/api"),
            key("https://git.acme.fr/acme/api")
        );
        assert_eq!(
            key("https://GIT.acme.fr/Acme/api.git"),
            key("git@git.acme.fr:Acme/api")
        );
    }

    /// An address, and what its refusal must be.
    type Case = (&'static str, fn(&BadAddress) -> bool);

    #[test]
    fn dangerous_addresses_are_refused() {
        let refused: &[Case] = &[
            ("", |e| *e == BadAddress::Empty),
            ("file:///c/src/api", |e| *e == BadAddress::Local),
            ("FILE:///etc", |e| *e == BadAddress::Local),
            ("http://github.com/acme/api", |e| {
                matches!(e, BadAddress::Scheme { .. })
            }),
            ("git://github.com/acme/api", |e| {
                matches!(e, BadAddress::Scheme { .. })
            }),
            ("ext::sh -c touch% /tmp/pwned", |e| *e == BadAddress::Blank),
            ("ext::sh%20-c%20id", |e| {
                matches!(e, BadAddress::Scheme { .. })
            }),
            ("fd::17", |e| matches!(e, BadAddress::Scheme { .. })),
            ("--upload-pack=touch /tmp/x", |e| *e == BadAddress::Blank),
            ("--upload-pack=calc.exe", |e| *e == BadAddress::Option),
            ("-oProxyCommand=calc", |e| *e == BadAddress::Option),
            ("ssh://-oProxyCommand=calc/x", |e| *e == BadAddress::Host),
            ("git@-oProxyCommand=calc:x", |e| *e == BadAddress::Host),
            ("https://github.com/-acme/api", |e| *e == BadAddress::Path),
            ("https://github.com/acme/--upload-pack=x", |e| {
                *e == BadAddress::Path
            }),
            ("https://github.com/acme/../../etc", |e| {
                *e == BadAddress::Path
            }),
            ("C:\\src\\api", |e| *e == BadAddress::Local),
            ("C:/src/api", |e| *e == BadAddress::Local),
            ("c:src", |e| *e == BadAddress::Local),
            ("/home/alice/api", |e| *e == BadAddress::Local),
            ("./api", |e| *e == BadAddress::Local),
            ("../api", |e| *e == BadAddress::Local),
            ("~/api", |e| *e == BadAddress::Local),
            ("\\\\server\\share\\api", |e| *e == BadAddress::Local),
            ("api", |e| *e == BadAddress::Local),
            ("https://user:secret@github.com/acme/api", |e| {
                *e == BadAddress::Credential
            }),
            ("https://ghp_token@github.com/acme/api", |e| {
                *e == BadAddress::Credential
            }),
            ("ssh://git:pass@github.com/acme/api", |e| {
                *e == BadAddress::Credential
            }),
            ("https://github.com", |e| *e == BadAddress::Path),
            ("https://github.com/", |e| *e == BadAddress::Path),
            ("https:///acme/api", |e| *e == BadAddress::Host),
            ("https://git hub.com/acme/api", |e| *e == BadAddress::Blank),
            ("https://github.com/acme/api\n--upload-pack=x", |e| {
                *e == BadAddress::Blank
            }),
        ];
        for (url, is) in refused {
            match address(url) {
                Ok(a) => panic!("{url} passed: {a:?}"),
                Err(e) => assert!(is(&e), "{url}: {e:?}"),
            }
        }
        let a = address("  https://github.com/acme/api.git ").unwrap();
        assert_eq!(a.url, "https://github.com/acme/api.git");
        assert_eq!(a.scheme, Scheme::Https);
        assert_eq!(a.forge, Some(Forge::Github));
        assert_eq!(a.name(), "api");
        let a = address("git@ssh.dev.azure.com:v3/acme/demo/api").unwrap();
        assert_eq!(a.scheme, Scheme::Ssh);
        assert_eq!(a.forge, Some(Forge::AzureDevops));
    }

    fn listed() -> Listed {
        Listed {
            name: "API".into(),
            repos: vec![Repo {
                url: "https://github.com/acme/api.git".into(),
                folder: String::new(),
            }],
            ..Listed::default()
        }
    }

    #[test]
    fn a_project_of_the_list_is_checked_whole() {
        assert_eq!(check(&listed()), Ok(()));
        let mut p = listed();
        p.repos[0].url = "file:///c/api".into();
        assert!(check(&p).is_err());
        let mut p = listed();
        p.name = " ".into();
        assert!(check(&p).is_err());
        let mut p = listed();
        p.branch = "--orphan".into();
        assert!(check(&p).is_err());
        let mut p = listed();
        p.branch = "release/2.3".into();
        assert!(check(&p).is_ok());
        let mut p = listed();
        p.color = "red".into();
        assert!(check(&p).is_err());
        let mut p = listed();
        p.logo = Some("data:image/svg+xml;base64,PHN2Zz48L3N2Zz4=".into());
        assert!(check(&p).is_err(), "no SVG: it can carry a script");
        let mut p = listed();
        p.logo = Some("data:image/png;base64,iVBORw0KGgo=".into());
        assert!(check(&p).is_ok());
        // Several repositories: a folder each, never twice the same, never out of the project.
        let mut p = listed();
        p.repos.push(Repo {
            url: "git@github.com:acme/front.git".into(),
            folder: String::new(),
        });
        assert_eq!(p.folders(), vec!["api", "front"]);
        assert!(check(&p).is_ok());
        p.repos[1].folder = "api".into();
        assert!(check(&p).is_err());
        p.repos[1].folder = "../front".into();
        assert!(check(&p).is_err());
        p.repos[1].folder = ".git".into();
        assert!(check(&p).is_err());
        let mut p = listed();
        p.access = Access::Members { members: vec![] };
        assert!(check(&p).is_err());
        p.access = Access::Members {
            members: vec!["01".repeat(16)],
        };
        assert!(check(&p).is_ok());
        assert!(p.open_to(&MemberId([1; 16])));
        assert!(!p.open_to(&MemberId([2; 16])));
    }

    #[test]
    fn the_catalog_finds_the_project_of_an_address() {
        let c = Catalog {
            projects: vec![Item {
                id: "aa".into(),
                project: Listed {
                    repos: vec![
                        Repo {
                            url: "https://github.com/acme/front".into(),
                            folder: String::new(),
                        },
                        Repo {
                            url: "https://github.com/acme/api".into(),
                            folder: String::new(),
                        },
                    ],
                    ..listed()
                },
                by: String::new(),
                at: 0,
                seq: 1,
            }],
            ..Catalog::default()
        };
        let (item, n) = c.holding("git@github.com:Acme/api.git").unwrap();
        assert_eq!((item.id.as_str(), n), ("aa", 1));
        assert!(c.holding("https://github.com/acme/other").is_none());
        assert!(c.holding("C:/src/api").is_none());
    }
}
