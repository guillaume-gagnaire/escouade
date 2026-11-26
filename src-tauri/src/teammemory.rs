//! The shared memory of a team's project on the team server (TS-10, `docs/TEAM-SERVER.md` §8 and C14-B).
//!
//! A memory is an instruction that lasts, read by every agent of the team: an agent fooled by a third party's data, or a
//! member who means harm, must not leave one there that the others follow. So:
//!
//! - **Where**: the journal of the project's docs (TS-9, `teamdocs`: the one of its Kanban), sealed and signed as they
//!   are, by the same choice (C4-B), the same pass and the same outbox. Each write is an object of its own (kind
//!   `memory`, an opaque id drawn here): its name, description, kind and text, and its **provenance**. Nothing is ever
//!   taken out of the journal: a memory forgotten, set aside or put in quarantine stays there, for the investigation.
//! - **The provenance**, signed by the device that writes (the operation's signature, checked when it is read): the
//!   member and their device, the date, by hand or by an agent (its ticket's key, harness, model), and whether that
//!   agent's session had received a third party's data. A memory whose content fields were not all written by the
//!   device its provenance names, or whose device is not of the member it names, is refused: never shown.
//! - **Proposed, then accepted** (C14-B, `memory.review`, on by default, imposable by the team): a memory an agent
//!   writes is born proposed. Only the agents of its author read it (on the machines of that member), marked as such.
//!   It becomes the team's once a human accepts it (« Garder »: its author, or an administrator), at its ticket's
//!   validation or in Réglages › Projets › Équipe › Mémoire partagée; « Écarter » sets it aside. An acceptance holds the digest
//!   of what was accepted: a content written over afterwards is no longer accepted. A memory written by hand by a member
//!   is accepted at once. No tool of the MCP server accepts one: an agent never validates a memory.
//! - **The removal**: forgotten in the window, a memory leaves a tombstone that goes to every machine; `forget_memory`
//!   (a tool: an agent, or Claude outside Escouade) of a memory a human accepted only **proposes** its removal (ESC-647),
//!   shown at the review as a write is, that a member settles in the window (« Retirer » or « Garder »); a tool still
//!   forgets its own proposals, and, the review off, what an agent wrote. An administrator puts in
//!   **quarantine** by a rule (the agent of a ticket, a member or one of their devices, since a date), kept in the
//!   journal: it holds for what is there, and for what comes later (a device back from offline).
//! - **On this machine**, the copy of what this member may read (`team-docs/.escouade/memory/`, beside the copy of the
//!   docs) as a project's folder holds it: what reads the memory (the tools, the index, the vectors) reads it as before.
//!   `team-memory.json` keeps what the journal says of each one there (its provenance, whether it is accepted).
//! - A project outside a team, or whose team keeps its docs in the repository, keeps `.escouade/memory/` as before.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Result};
use escouade_team::crypto::hlc::Hlc;
use escouade_team::crypto::ops::{OpBody, Value};
use escouade_team::crypto::{ObjectId, ProjectId, Role};
use escouade_team::Team;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::core::Core;
use crate::memory::{self, Draft, Kind, Memory};
use crate::model::{Project, UiEvent};

/// The kind of a memory's objects in the journal.
pub const KIND: &str = "memory";
/// The kind of a quarantine's rules.
pub const RULE: &str = "memory-rule";
/// The fields a memory's author writes, and no one else.
const CONTENT: [&str; 5] = ["name", "description", "type", "body", "provenance"];

fn hex_id(o: &ObjectId) -> String {
    hex::encode(o.0)
}

fn id_of(text: &str) -> Option<ObjectId> {
    let mut out = [0u8; 16];
    hex::decode_to_slice(text, &mut out).ok()?;
    Some(ObjectId(out))
}

fn say(e: escouade_team::Error) -> anyhow::Error {
    anyhow!(crate::teams::say(&e))
}

// === The provenance ===

/// The agent that wrote a memory.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AgentOf {
    pub name: String,
    /// The key of its ticket, when it works on one.
    pub ticket: Option<String>,
    pub harness: String,
    pub model: String,
}

/// Who wrote a memory, and how: signed with it by the device that wrote it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Provenance {
    /// The member (hex), checked against the device that signed.
    pub member: String,
    /// The device (hex) that wrote it.
    pub device: String,
    /// The member's name, as their device says it.
    pub name: String,
    /// When (milliseconds).
    pub at: i64,
    /// `hand` (a member, in the window) or `agent`.
    pub by: String,
    pub agent: Option<AgentOf>,
    /// The agent's session had received a third party's data before it wrote.
    pub third_party: bool,
    /// Imported from the repository's `.escouade/memory/` by « Verser à l'équipe ».
    pub imported: bool,
}

/// Who writes now.
#[derive(Debug, Clone, Default)]
pub struct Writer {
    /// None: a member, by hand.
    pub agent: Option<AgentOf>,
    pub third_party: bool,
    pub imported: bool,
}

impl Writer {
    pub fn hand() -> Writer {
        Writer::default()
    }
}

/// Where a memory stands for the team.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Standing {
    #[default]
    Accepted,
    Proposed,
    Rejected,
}

/// A memory of the journal, as this device reads it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Entry {
    /// Its object (hex).
    pub id: String,
    pub name: String,
    pub description: String,
    pub kind: Kind,
    pub body: String,
    pub provenance: Provenance,
    pub standing: Standing,
    /// The member (hex) who accepted it, or set it aside.
    pub reviewed_by: Option<String>,
    /// Forgotten (a tombstone), and by whom (member, hex).
    pub removed: bool,
    pub removed_by: Option<String>,
    /// Under a quarantine's rule (its id).
    pub quarantined: Option<String>,
    /// Its removal proposed by a tool (ESC-647): it stays read until a member settles it in the window.
    pub removal: Option<RemovalAsk>,
    /// Written by this member (by hand or by one of their agents).
    pub mine: bool,
    /// Given as data, never as an instruction: proposed, written by an agent that had read a third party's data, or
    /// another member's that no other human kept (SR3-04).
    pub untrusted: bool,
    /// The hosts of the addresses it names that the project does not know: shown at its validation.
    pub unknown: Vec<String>,
    /// When it last counted (its writing, its acceptance or its removal: milliseconds).
    pub at: u64,
    /// The same, as the journal orders it.
    #[serde(skip)]
    order: Vec<u8>,
}

/// A removal a tool proposed (ESC-647), signed by the device that asked: the member (hex), their name, the agent.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RemovalAsk {
    pub member: String,
    pub name: String,
    pub agent: Option<AgentOf>,
    pub at: i64,
}

/// What `forget_memory` did (ESC-647).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Forgot {
    /// Nothing of that name.
    Nothing,
    /// Tombstones left.
    Gone,
    /// Its removal proposed: a human accepted it.
    Proposed,
}

/// A quarantine: what it takes out, written by an administrator.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Rule {
    /// The agent of this ticket (its key).
    pub ticket: Option<String>,
    /// This member (hex).
    pub member: Option<String>,
    /// This device (hex).
    pub device: Option<String>,
    /// Written since (milliseconds).
    pub since: Option<i64>,
}

impl Rule {
    fn is_empty(&self) -> bool {
        self.ticket.is_none()
            && self.member.is_none()
            && self.device.is_none()
            && self.since.is_none()
    }

    fn cleaned(mut self) -> Rule {
        let clean = |s: Option<String>| s.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        self.ticket = clean(self.ticket).map(|t| t.to_uppercase());
        self.member = clean(self.member).map(|t| t.to_lowercase());
        self.device = clean(self.device).map(|t| t.to_lowercase());
        self
    }

    fn takes(&self, p: &Provenance) -> bool {
        !self.is_empty()
            && self.ticket.as_ref().is_none_or(|t| {
                p.agent
                    .as_ref()
                    .and_then(|a| a.ticket.as_deref())
                    .is_some_and(|k| k.eq_ignore_ascii_case(t))
            })
            && self.member.as_ref().is_none_or(|m| *m == p.member)
            && self.device.as_ref().is_none_or(|d| *d == p.device)
            && self.since.is_none_or(|s| p.at >= s)
    }
}

/// A quarantine as the window shows it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RuleView {
    pub id: String,
    #[serde(flatten)]
    pub rule: Rule,
    /// The administrator (member, hex, and name as their device says it), when.
    pub by: String,
    pub by_name: String,
    pub at: i64,
    /// The memories it takes out now.
    pub count: usize,
}

/// Who wrote memories of the journal: the choices of a quarantine.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AuthorView {
    pub member: String,
    pub device: String,
    pub name: String,
}

/// What the journal holds of the memory, as this device reads it.
#[derive(Debug, Clone, Default)]
pub struct Resolved {
    pub entries: Vec<Entry>,
    pub rules: Vec<RuleView>,
    /// The memories refused: a provenance that the signature does not bear out.
    pub refused: usize,
    /// This member (hex), and whether they are an administrator.
    pub me: String,
    pub admin: bool,
}

impl Resolved {
    /// What this member's agents read: for each name, the latest of the memories accepted and of this member's own
    /// proposals, unless it was forgotten; never one set aside, in quarantine or refused.
    pub fn visible(&self) -> Vec<&Entry> {
        let mut best: BTreeMap<&str, &Entry> = BTreeMap::new();
        for e in &self.entries {
            if e.quarantined.is_some() || e.standing == Standing::Rejected {
                continue;
            }
            if e.standing == Standing::Proposed && !e.mine {
                continue;
            }
            match best.get(e.name.as_str()) {
                Some(b) if b.order >= e.order => {}
                _ => {
                    best.insert(&e.name, e);
                }
            }
        }
        best.into_values().filter(|e| !e.removed).collect()
    }

    /// The removals a tool proposed (ESC-647), of what this member's agents read: any member settles them.
    pub fn removals(&self) -> Vec<&Entry> {
        self.visible()
            .into_iter()
            .filter(|e| e.removal.is_some())
            .collect()
    }

    /// The proposals this member may accept or set aside: their own, or every one for an administrator.
    pub fn reviewable(&self) -> Vec<&Entry> {
        self.entries
            .iter()
            .filter(|e| {
                e.standing == Standing::Proposed
                    && !e.removed
                    && e.quarantined.is_none()
                    && (e.mine || self.admin)
            })
            .collect()
    }
}

/// The digest of what a memory says and of its provenance: what an acceptance holds.
fn digest(fields: &BTreeMap<String, Value>) -> String {
    let parts: Vec<Value> = CONTENT
        .iter()
        .map(|k| fields.get(*k).cloned().unwrap_or(Value::Null))
        .collect();
    hex::encode(Sha256::digest(
        serde_json::to_vec(&parts).unwrap_or_default(),
    ))
}

/// The host of a remote's address (`https://host/…`, `git@host:…`, `ssh://git@host/…`).
fn remote_host(url: &str) -> Option<String> {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    let rest = rest.rsplit_once('@').map_or(rest, |(_, r)| r);
    let host: String = rest
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        .collect();
    (!host.is_empty()).then(|| host.to_lowercase())
}

/// The hosts a project knows: its remotes', this machine's.
pub(crate) fn known_hosts(remotes: &[String]) -> BTreeSet<String> {
    let mut out: BTreeSet<String> = ["localhost", "127.0.0.1", "[::1]", "escouade.dev"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    out.extend(remotes.iter().filter_map(|u| remote_host(u)));
    out
}

/// The memories and the quarantines of the journal of `pid`, as this device reads them (the module's doc). `devices`
/// keeps the member of each device met, for those the journal no longer names (after a snapshot, a device revoked).
pub fn resolve(
    t: &mut Team,
    pid: ProjectId,
    devices: &mut BTreeMap<String, String>,
    known: &BTreeSet<String>,
) -> Result<Resolved> {
    let me = hex::encode(t.identity().member.0);
    let mut roles: BTreeMap<String, Role> = BTreeMap::new();
    for (m, role, ds) in t.members().map_err(say)? {
        let m = hex::encode(m.0);
        for d in ds {
            devices.insert(hex::encode(d.0), m.clone());
        }
        roles.insert(m, role);
    }
    let admin = roles.get(&me) == Some(&Role::Admin);
    let r = t.replica(&pid).map_err(say)?;
    let mut member_of = |device: &[u8; 16]| -> Option<String> {
        let d = hex::encode(device);
        if let Some(m) = devices.get(&d) {
            return Some(m.clone());
        }
        let m = r
            .author_member(&escouade_team::crypto::DeviceId(*device))
            .ok()
            .flatten()
            .map(|m| hex::encode(m.0))?;
        devices.insert(d, m.clone());
        Some(m)
    };
    let stamp =
        |id: &ObjectId, field: &str| -> Option<Hlc> { r.field_stamp(id, field).ok().flatten() };
    let mut out = Resolved {
        me: me.clone(),
        admin,
        ..Resolved::default()
    };
    // The quarantines, an administrator's alone.
    let mut rules: Vec<(RuleView, Rule)> = Vec::new();
    for view in r.objects(Some(RULE)).map_err(say)? {
        let Some(s) = stamp(&view.id, "rule") else {
            continue;
        };
        let Some(by) = member_of(&s.device.0) else {
            continue;
        };
        if roles.get(&by) != Some(&Role::Admin) {
            continue;
        }
        let lifted = view.fields.get("lifted") == Some(&Value::Bool(true))
            && stamp(&view.id, "lifted")
                .and_then(|l| member_of(&l.device.0))
                .is_some_and(|m| roles.get(&m) == Some(&Role::Admin));
        if lifted {
            continue;
        }
        let Some(rule) = view
            .fields
            .get("rule")
            .and_then(|v| serde_json::from_value::<Rule>(v.clone()).ok())
            .filter(|r| !r.is_empty())
        else {
            continue;
        };
        let by_name = view
            .fields
            .get("byName")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        rules.push((
            RuleView {
                id: hex_id(&view.id),
                rule: rule.clone(),
                by,
                by_name,
                at: s.wall as i64,
                count: 0,
            },
            rule,
        ));
    }
    for view in r.objects(Some(KIND)).map_err(say)? {
        let f = &view.fields;
        let Some(origin) = stamp(&view.id, "provenance") else {
            out.refused += 1;
            continue;
        };
        // Every field of its content written by the device of its provenance.
        if CONTENT
            .iter()
            .any(|k| stamp(&view.id, k).is_none_or(|s| s.device != origin.device))
        {
            out.refused += 1;
            continue;
        }
        let Some(prov) = f
            .get("provenance")
            .and_then(|v| serde_json::from_value::<Provenance>(v.clone()).ok())
        else {
            out.refused += 1;
            continue;
        };
        let author = member_of(&origin.device.0);
        if prov.device != hex::encode(origin.device.0) || author.as_deref() != Some(&prov.member) {
            out.refused += 1;
            continue;
        }
        let name = f.get("name").and_then(Value::as_str).unwrap_or_default();
        if memory::name_of(name).ok().as_deref() != Some(name) {
            out.refused += 1;
            continue;
        }
        let text = |k: &str| {
            f.get(k)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string()
        };
        let mut e = Entry {
            id: hex_id(&view.id),
            name: name.to_string(),
            description: text("description"),
            kind: Kind::parse(&text("type")).unwrap_or_default(),
            body: text("body"),
            mine: prov.member == me,
            provenance: prov.clone(),
            standing: Standing::Proposed,
            ..Entry::default()
        };
        let mut order = origin;
        // Accepted: what it says now, by its author or an administrator.
        let hash = digest(f);
        let reviewer = |field: &str, member_of: &mut dyn FnMut(&[u8; 16]) -> Option<String>| {
            let s = stamp(&view.id, field)?;
            let m = member_of(&s.device.0)?;
            (m == prov.member || roles.get(&m) == Some(&Role::Admin)).then_some((m, s))
        };
        if f.get("accepted").and_then(Value::as_str) == Some(hash.as_str()) {
            if let Some((m, s)) = reviewer("accepted", &mut member_of) {
                e.standing = Standing::Accepted;
                e.reviewed_by = Some(m);
                order = order.max(s);
            }
        }
        if f.get("rejected") == Some(&Value::Bool(true)) {
            if let Some((m, s)) = reviewer("rejected", &mut member_of) {
                if e.standing != Standing::Accepted || s > order {
                    e.standing = Standing::Rejected;
                    e.reviewed_by = Some(m);
                }
            }
        }
        if f.get("removed") == Some(&Value::Bool(true)) {
            if let Some(s) = stamp(&view.id, "removed") {
                if let Some(m) = member_of(&s.device.0) {
                    e.removed = true;
                    e.removed_by = Some(m);
                    order = order.max(s);
                }
            }
        }
        if !e.removed {
            if let Some(ask) = f
                .get("removalAsked")
                .and_then(|v| serde_json::from_value::<RemovalAsk>(v.clone()).ok())
            {
                // Its member is the one of the device that signed it, whatever it says.
                if let Some(m) =
                    stamp(&view.id, "removalAsked").and_then(|s| member_of(&s.device.0))
                {
                    e.removal = Some(RemovalAsk { member: m, ..ask });
                }
            }
        }
        e.order = order.to_bytes().to_vec();
        e.at = order.wall;
        // Another member's word is a third party's here (SR3-04): what its author alone kept (written by hand, or by
        // one of their agents and kept by them) is data, never an instruction, until another human keeps it (an
        // administrator, or this member).
        let theirs_alone =
            prov.member != me && e.reviewed_by.as_deref().is_none_or(|r| r == prov.member);
        e.untrusted = e.standing != Standing::Accepted || prov.third_party || theirs_alone;
        if let Some((rv, _)) = rules.iter().find(|(_, rule)| rule.takes(&prov)) {
            e.quarantined = Some(rv.id.clone());
        }
        e.unknown = memory::addresses(&format!("{}\n{}", e.description, e.body))
            .into_iter()
            .filter(|h| !known.contains(h))
            .collect();
        out.entries.push(e);
    }
    // The hosts the team's accepted memories name are known too.
    let accepted: BTreeSet<String> = out
        .entries
        .iter()
        .filter(|e| e.standing == Standing::Accepted && e.quarantined.is_none())
        .flat_map(|e| memory::addresses(&format!("{}\n{}", e.description, e.body)))
        .collect();
    for e in out.entries.iter_mut() {
        if e.standing != Standing::Accepted {
            e.unknown.retain(|h| !accepted.contains(h));
        }
    }
    for (mut rv, rule) in rules {
        rv.count = out
            .entries
            .iter()
            .filter(|e| rule.takes(&e.provenance))
            .count();
        out.rules.push(rv);
    }
    out.entries.sort_by(|a, b| a.order.cmp(&b.order));
    Ok(out)
}

// === What this machine keeps ===

/// What this machine keeps of the journal's memory, beside its copy: what each memory of the folder is.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Manifest {
    /// The memories of the folder, by name.
    pub memories: BTreeMap<String, Entry>,
    /// The member of each device met (hex).
    pub devices: BTreeMap<String, String>,
    pub refused: usize,
}

impl Manifest {
    pub fn load(path: &Path) -> Manifest {
        std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        crate::paths::write_atomic(path, &serde_json::to_vec_pretty(self)?)?;
        Ok(())
    }
}

/// The folder `dir` brought to what this member reads (`visible`): each memory's file written when it differs, those
/// of no memory removed, the index written again. Files written or removed.
pub fn project_into(dir: &Path, visible: &[&Entry]) -> Result<usize> {
    std::fs::create_dir_all(dir)?;
    let mut changed = 0;
    let keep: BTreeSet<&str> = visible.iter().map(|e| e.name.as_str()).collect();
    for m in memory::list(dir)? {
        if !keep.contains(m.name.as_str()) {
            std::fs::remove_file(dir.join(format!("{}.md", m.name)))?;
            changed += 1;
        }
    }
    for e in visible {
        let text = memory::render(&Memory {
            name: e.name.clone(),
            description: e.description.clone(),
            kind: e.kind,
            body: e.body.clone(),
        });
        let path = dir.join(format!("{}.md", e.name));
        if std::fs::read_to_string(&path).ok().as_deref() != Some(text.as_str()) {
            crate::paths::write_atomic(&path, text.as_bytes())?;
            changed += 1;
        }
    }
    if changed > 0 || !dir.join(memory::INDEX).exists() {
        memory::write_index(dir)?;
    }
    Ok(changed)
}

/// Writes a memory in the journal: a new object, its provenance signed with it; accepted at once when written by hand
/// or when `review` is off, proposed otherwise. Offline, it waits in the outbox. Its object.
pub fn write_entry(
    t: &mut Team,
    pid: ProjectId,
    m: &Memory,
    writer: &Writer,
    name_of_me: &str,
    review: bool,
) -> Result<ObjectId> {
    let ident = t.identity();
    let prov = Provenance {
        member: hex::encode(ident.member.0),
        device: hex::encode(ident.device.0),
        name: name_of_me.to_string(),
        at: crate::model::now_ms(),
        by: if writer.agent.is_some() {
            "agent"
        } else {
            "hand"
        }
        .into(),
        agent: writer.agent.clone(),
        third_party: writer.third_party,
        imported: writer.imported,
    };
    let mut fields: BTreeMap<String, Value> = [
        ("name", json!(m.name)),
        ("description", json!(m.description)),
        ("type", json!(m.kind.as_str())),
        ("body", json!(m.body)),
        ("provenance", serde_json::to_value(&prov)?),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect();
    if writer.agent.is_none() || !review {
        let hash = digest(&fields);
        fields.insert("accepted".into(), json!(hash));
    }
    let object = ObjectId(*uuid::Uuid::new_v4().as_bytes());
    t.write(
        pid,
        OpBody::Create {
            object,
            kind: KIND.into(),
            fields,
        },
    )
    .map_err(say)?;
    Ok(object)
}

/// The memories of the repository's `.escouade/memory/` written in the journal by « Verser à l'équipe », by hand (the
/// member's gesture), said imported; one the journal holds already the same is not written again. How many.
pub fn import(
    t: &mut Team,
    pid: ProjectId,
    repo_dir: &Path,
    name_of_me: &str,
    have: &Resolved,
) -> Result<usize> {
    let mut n = 0;
    for m in memory::list(repo_dir).unwrap_or_default() {
        if memory::check(&m.description, &m.body).is_err() || m.description.trim().is_empty() {
            log::warn!(
                "team memory: « {} » not imported (a secret, or no description)",
                m.name
            );
            continue;
        }
        let same = have.visible().iter().any(|e| {
            e.name == m.name
                && e.description == m.description
                && e.body == m.body
                && e.kind == m.kind
        });
        if same {
            continue;
        }
        let writer = Writer {
            imported: true,
            ..Writer::hand()
        };
        write_entry(t, pid, &m, &writer, name_of_me, true)?;
        n += 1;
    }
    Ok(n)
}

// === What the window shows ===

/// The shared memory of a project of a team, as Réglages › Projets › Équipe › Mémoire partagée shows it.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamMemoryView {
    /// Its memory is on the team server (else in the project's folder, as before).
    pub on: bool,
    /// « Valider les souvenirs des agents » (C14-B), and who imposes it.
    pub review: bool,
    pub review_locked_by: Option<String>,
    pub admin: bool,
    pub me: String,
    /// What this member's agents read.
    pub memories: Vec<Entry>,
    /// The proposals this member may accept or set aside.
    pub proposals: Vec<Entry>,
    /// The removals a tool proposed (ESC-647): « Retirer » or « Garder ».
    pub removals: Vec<Entry>,
    /// The memories forgotten, set aside or in quarantine: the journal keeps them (an administrator's view holds all).
    pub withdrawn: Vec<Entry>,
    pub rules: Vec<RuleView>,
    /// Who wrote memories, and the tickets whose agents did: the choices of a quarantine.
    pub authors: Vec<AuthorView>,
    pub tickets: Vec<String>,
    pub refused: usize,
}

/// What a quarantine would take out, before it does anything.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuarantinePreview {
    pub rule: Rule,
    pub entries: Vec<Entry>,
    /// Written now (`apply`): the rule's id.
    pub applied: Option<String>,
}

fn view_of(r: &Resolved) -> TeamMemoryView {
    let mut v = TeamMemoryView {
        on: true,
        admin: r.admin,
        me: r.me.clone(),
        refused: r.refused,
        rules: r.rules.clone(),
        ..TeamMemoryView::default()
    };
    v.memories = r.visible().into_iter().cloned().collect();
    v.memories.sort_by(|a, b| a.name.cmp(&b.name));
    v.proposals = r.reviewable().into_iter().cloned().collect();
    v.removals = r.removals().into_iter().cloned().collect();
    let shown: BTreeSet<&str> = v.memories.iter().map(|e| e.id.as_str()).collect();
    v.withdrawn = r
        .entries
        .iter()
        .filter(|e| e.removed || e.quarantined.is_some() || e.standing == Standing::Rejected)
        .filter(|e| !shown.contains(e.id.as_str()))
        .filter(|e| r.admin || e.mine || e.standing == Standing::Accepted)
        .cloned()
        .collect();
    let mut authors: Vec<AuthorView> = Vec::new();
    let mut tickets: BTreeSet<String> = BTreeSet::new();
    for e in &r.entries {
        let p = &e.provenance;
        if !authors.iter().any(|a| a.device == p.device) {
            authors.push(AuthorView {
                member: p.member.clone(),
                device: p.device.clone(),
                name: p.name.clone(),
            });
        }
        if let Some(k) = p.agent.as_ref().and_then(|a| a.ticket.clone()) {
            tickets.insert(k);
        }
    }
    v.authors = authors;
    v.tickets = tickets.into_iter().collect();
    v
}

/// Whether a tool of a harness reads the web, as its hook or its stream names it (TS-10): Claude Code's `WebFetch` and
/// `WebSearch`, Codex's `web_search`. What it read is a third party's data.
pub fn web_tool(name: &str) -> bool {
    matches!(name, "WebFetch" | "WebSearch" | "web_search" | "web_fetch")
}

// === The core ===

impl Core {
    /// The agent `id` received a third party's data in its session (TS-10): the memories it writes say so.
    pub(crate) fn agent_read_third_party(&self, id: &str) {
        let Ok(h) = self.agent(id) else {
            return;
        };
        if h.lock().meta.third_party {
            return;
        }
        h.lock().meta.third_party = true;
        self.request_save();
    }

    /// Where a project's memory is read and written: this machine's copy of its team's when its docs are on the team
    /// server (TS-10), else `.escouade/memory/` of its folder.
    pub fn memory_dir(&self, p: &Project) -> PathBuf {
        if self.team_docs_on(p) {
            self.team_memory_dir(&p.id)
        } else {
            memory::dir_of(p)
        }
    }

    pub(crate) fn team_memory_dir(&self, project_id: &str) -> PathBuf {
        self.team_docs_root(project_id).join(memory::DIR)
    }

    fn team_memory_manifest_file(&self, project_id: &str) -> PathBuf {
        self.data.project_files(project_id).join("team-memory.json")
    }

    pub(crate) fn team_memory_manifest(&self, project_id: &str) -> Manifest {
        Manifest::load(&self.team_memory_manifest_file(project_id))
    }

    /// What the journal says of the memory `name` this machine's agents read: its provenance, whether it is accepted.
    /// None for a project whose memory is not on the team server.
    pub(crate) fn team_memory_entry(&self, project: &Project, name: &str) -> Option<Entry> {
        if !self.team_docs_on(project) {
            return None;
        }
        self.team_memory_manifest(&project.id)
            .memories
            .get(name)
            .cloned()
    }

    /// « Valider les souvenirs des agents » for the project (C14-B): on unless the project turned it off, and on
    /// whatever it says when its team imposes it.
    pub(crate) fn team_memory_review(&self, project: &Project) -> (bool, Option<String>) {
        match self.team_layers_for(&project.id) {
            Some((team, layers)) => match layers.enforced.get(crate::projectfile::MEMORY_REVIEW) {
                Some(v) => (v.as_bool() != Some(false), Some(team)),
                None => (!project.memory_review_off, None),
            },
            None => (!project.memory_review_off, None),
        }
    }

    async fn team_memory_known(&self, project: &Project) -> BTreeSet<String> {
        let mut urls = Vec::new();
        let path = crate::docsbranch::base_of(project);
        let path = path.to_string_lossy().to_string();
        for remote in crate::git::remotes(&path).await {
            if let Some(u) = crate::git::remote_url(&path, &remote).await {
                urls.push(u);
            }
        }
        known_hosts(&urls)
    }

    /// The memory of the journal brought to this machine's copy (in a pass of the project's docs, `t` held): the
    /// manifest saved; the window told when the folder changed.
    pub(crate) async fn team_memory_settle(
        &self,
        project: &Project,
        t: &mut Team,
        pid: ProjectId,
    ) -> Result<Resolved> {
        // No remote of git read here (the pass holds the team): the unknown addresses are the window's view's.
        let known = BTreeSet::new();
        let file = self.team_memory_manifest_file(&project.id);
        let mut man = Manifest::load(&file);
        let resolved = resolve(t, pid, &mut man.devices, &known)?;
        let visible = resolved.visible();
        let changed = project_into(&self.team_memory_dir(&project.id), &visible)?;
        let before = man.memories.clone();
        // As the manifest keeps them (the journal's order is not saved): compared to what it held, the window is told
        // of a change only, never at every pass.
        man.memories = visible
            .iter()
            .map(|e| {
                let mut e = (*e).clone();
                e.order.clear();
                (e.name.clone(), e)
            })
            .collect();
        let refused_changed = man.refused != resolved.refused;
        man.refused = resolved.refused;
        man.save(&file)?;
        if changed > 0 || before != man.memories || refused_changed {
            self.memory_changed(&project.id);
        }
        Ok(resolved)
    }

    /// The repository's `.escouade/memory/` written in the journal by a gesture (« Verser à l'équipe », the docs moved
    /// to the server), `t` held: once for the team, unless `again` (back from the repository). How many memories went.
    pub(crate) async fn team_memory_import(
        &self,
        project: &Project,
        t: &mut Team,
        pid: ProjectId,
        again: bool,
    ) -> Result<usize> {
        if !again && crate::teamdocs::memory_imported(t, pid) {
            return Ok(0);
        }
        let known = BTreeSet::new();
        let mut devices = self.team_memory_manifest(&project.id).devices;
        let have = resolve(t, pid, &mut devices, &known)?;
        let me = self.me_in(project).name;
        let n = import(t, pid, &memory::dir_of(project), &me, &have)?;
        crate::teamdocs::set_memory_imported(t, pid)?;
        Ok(n + 1)
    }

    /// Whether the repository's memory waits for « Verser à l'équipe »: the team's docs are on its server, and no
    /// device poured its memory yet.
    pub(crate) async fn team_memory_pending(&self, project: &Project) -> bool {
        let Ok((shared, pid)) = self.team_memory_link(project) else {
            return false;
        };
        let mut t = shared.lock().await;
        !crate::teamdocs::memory_imported(&mut t, pid)
    }

    /// « Verser à l'équipe » of the memory alone (the team's docs on its server already): the repository's
    /// `.escouade/memory/` written in the journal, nothing removed from the repository; this machine's copy read again.
    pub(crate) async fn team_memory_pour(&self, project_id: &str) -> Result<usize> {
        let project = self.project(project_id)?;
        let (shared, pid) = self.team_memory_link(&project)?;
        let one = crate::teamdocs::pass_lock(project_id);
        let _one = one.lock().await;
        let mut t = shared.lock().await;
        let n = self
            .team_memory_import(&project, &mut t, pid, false)
            .await?;
        let _ = t.sync_project(pid).await;
        self.team_memory_settle(&project, &mut t, pid).await?;
        // The files left in the repository, for « Retirer du dépôt ».
        let file = self.data.project_files(project_id).join("team-docs.json");
        let mut man = crate::teamdocs::Manifest::load(&file);
        for f in crate::teamdocs::repo_memory_files(&crate::docsbranch::base_of(&project)) {
            if !man.repo_files.contains(&f) {
                man.repo_files.push(f);
            }
        }
        man.save(&file)?;
        Ok(n)
    }

    /// The team, the journal and the pass's lock of a project whose memory is on the team server.
    fn team_memory_link(&self, project: &Project) -> Result<(crate::teams::Shared, ProjectId)> {
        if !self.team_docs_on(project) {
            bail!(tr!(
                "La mémoire de ce projet n’est pas sur le serveur d’équipe.",
                "This project’s memory is not on the team server."
            ));
        }
        let team_id = project.team.clone().ok_or_else(|| anyhow!("no team"))?;
        let teams = self.team_docs_teams().ok_or_else(|| anyhow!("no teams"))?;
        let shared = teams.team(&team_id).map_err(|e| anyhow!(e))?;
        let pid = project
            .team_project
            .as_deref()
            .and_then(id_of)
            .map(|o| ProjectId(o.0))
            .ok_or_else(|| anyhow!("no journal"))?;
        Ok((shared, pid))
    }

    /// After a write in the journal: the copy brought up to date at once, and the team read and sent soon, off the
    /// caller.
    async fn team_memory_after(
        &self,
        project: &Project,
        t: &mut Team,
        pid: ProjectId,
    ) -> Result<Resolved> {
        let resolved = self.team_memory_settle(project, t, pid).await?;
        if let (Some(team_id), Some(teams)) = (project.team.clone(), self.team_docs_teams()) {
            tokio::spawn(async move {
                let _ = teams.sync(&team_id).await;
            });
        }
        Ok(resolved)
    }

    /// A memory written in a project whose memory is on the team server: checked (no secret), then a new object of the
    /// journal with its provenance; proposed when an agent writes it and the review is on. Answers it as written,
    /// whether its name is new to this member's agents, and how it stands.
    pub(crate) async fn team_memory_write(
        &self,
        project_id: &str,
        draft: Draft,
        writer: Writer,
    ) -> Result<(Memory, bool, Entry)> {
        let project = self.project(project_id)?;
        let (shared, pid) = self.team_memory_link(&project)?;
        // Checked as a folder's memory is: its name, its description, its length, no secret.
        let m = memory::validated(draft)?;
        let (review, _) = self.team_memory_review(&project);
        let me = self.me_in(&project).name;
        let one = crate::teamdocs::pass_lock(project_id);
        let _one = one.lock().await;
        let mut t = shared.lock().await;
        let new = !self
            .team_memory_manifest(project_id)
            .memories
            .contains_key(&m.name);
        let object = write_entry(&mut t, pid, &m, &writer, &me, review)?;
        let resolved = self.team_memory_after(&project, &mut t, pid).await?;
        let entry = resolved
            .entries
            .iter()
            .find(|e| e.id == hex_id(&object))
            .cloned()
            .ok_or_else(|| anyhow!("the memory written is not read back"))?;
        Ok((m, new, entry))
    }

    /// A memory forgotten in a project whose memory is on the team server: a tombstone on what this member's agents
    /// read of that name (and on their own proposals of it), which goes to every machine. By a tool (`by_tool`, with
    /// the agent if one calls: ESC-647), a memory a human accepted is not forgotten: its removal is proposed.
    pub(crate) async fn team_memory_forget(
        &self,
        project_id: &str,
        name: &str,
        by_tool: Option<Option<AgentOf>>,
    ) -> Result<Forgot> {
        let project = self.project(project_id)?;
        let (shared, pid) = self.team_memory_link(&project)?;
        let name = memory::name_of(name)?;
        let one = crate::teamdocs::pass_lock(project_id);
        let _one = one.lock().await;
        let mut t = shared.lock().await;
        let known = BTreeSet::new();
        let mut devices = self.team_memory_manifest(project_id).devices;
        let resolved = resolve(&mut t, pid, &mut devices, &known)?;
        let visible: BTreeSet<String> = resolved.visible().iter().map(|e| e.id.clone()).collect();
        let targets: Vec<String> = resolved
            .entries
            .iter()
            .filter(|e| e.name == name && !e.removed)
            .filter(|e| visible.contains(&e.id) || (e.mine && e.standing == Standing::Proposed))
            .map(|e| e.id.clone())
            .collect();
        if targets.is_empty() {
            return Ok(Forgot::Nothing);
        }
        let (review, _) = self.team_memory_review(&project);
        let mut proposed = false;
        for id in targets {
            let o = id_of(&id).ok_or_else(|| anyhow!("an object"))?;
            // Accepted by a human: by hand, or kept at the review (the review off, what an agent wrote is accepted
            // without one, and a tool forgets it as it wrote it).
            let human = resolved.entries.iter().any(|e| {
                e.id == id
                    && e.standing == Standing::Accepted
                    && (review || e.provenance.by != "agent")
            });
            match &by_tool {
                Some(agent) if human => {
                    let ask = RemovalAsk {
                        member: resolved.me.clone(),
                        name: self.me_in(&project).name,
                        agent: agent.clone(),
                        at: crate::model::now_ms(),
                    };
                    t.set_field(pid, o, "removalAsked", serde_json::to_value(&ask)?)
                        .map_err(say)?;
                    proposed = true;
                }
                _ => {
                    t.set_field(pid, o, "removed", Value::Bool(true))
                        .map_err(say)?;
                }
            }
        }
        self.team_memory_after(&project, &mut t, pid).await?;
        Ok(if proposed {
            Forgot::Proposed
        } else {
            Forgot::Gone
        })
    }

    /// « Retirer » (`remove`) or « Garder » a memory whose removal a tool proposed (ESC-647): a member's gesture, in
    /// the window alone (no tool of the MCP server comes here). Removed, a tombstone for every machine; kept, the
    /// proposal goes.
    pub(crate) async fn team_memory_decide_removal(
        &self,
        project_id: &str,
        id: &str,
        remove: bool,
    ) -> Result<TeamMemoryView> {
        let project = self.project(project_id)?;
        let (shared, pid) = self.team_memory_link(&project)?;
        let object = id_of(id).ok_or_else(|| anyhow!("an id of 32 hexadecimal digits"))?;
        {
            let one = crate::teamdocs::pass_lock(project_id);
            let _one = one.lock().await;
            let mut t = shared.lock().await;
            let mut devices = self.team_memory_manifest(project_id).devices;
            let resolved = resolve(&mut t, pid, &mut devices, &BTreeSet::new())?;
            if !resolved
                .entries
                .iter()
                .any(|e| e.id == id && e.removal.is_some())
            {
                bail!(tr!(
                    "Ce retrait n’est plus proposé : la mémoire se met à jour.",
                    "This removal is no longer proposed: the memory is updating."
                ));
            }
            if remove {
                t.set_field(pid, object, "removed", Value::Bool(true))
                    .map_err(say)?;
            } else {
                t.set_field(pid, object, "removalAsked", Value::Null)
                    .map_err(say)?;
            }
            self.team_memory_after(&project, &mut t, pid).await?;
        }
        self.memory_changed(project_id);
        self.team_memory_view(project_id).await
    }

    /// The shared memory of a project of a team, as its settings show it.
    pub(crate) async fn team_memory_view(&self, project_id: &str) -> Result<TeamMemoryView> {
        let project = self.project(project_id)?;
        let (review, review_locked_by) = self.team_memory_review(&project);
        if !self.team_docs_on(&project) {
            return Ok(TeamMemoryView {
                review,
                review_locked_by,
                ..TeamMemoryView::default()
            });
        }
        let (shared, pid) = self.team_memory_link(&project)?;
        let known = self.team_memory_known(&project).await;
        let mut t = shared.lock().await;
        let mut devices = self.team_memory_manifest(project_id).devices;
        let resolved = resolve(&mut t, pid, &mut devices, &known)?;
        drop(t);
        let mut v = view_of(&resolved);
        v.review = review;
        v.review_locked_by = review_locked_by;
        Ok(v)
    }

    /// « Garder » (`keep`) or « Écarter » a proposed memory (its object): by its author or an administrator, in the
    /// window alone (no tool of the MCP server comes here). Kept, it is the team's; set aside, no one reads it.
    pub(crate) async fn team_memory_decide(
        &self,
        project_id: &str,
        id: &str,
        keep: bool,
    ) -> Result<TeamMemoryView> {
        let project = self.project(project_id)?;
        let (shared, pid) = self.team_memory_link(&project)?;
        let object = id_of(id).ok_or_else(|| anyhow!("an id of 32 hexadecimal digits"))?;
        {
            let one = crate::teamdocs::pass_lock(project_id);
            let _one = one.lock().await;
            let mut t = shared.lock().await;
            let mut devices = self.team_memory_manifest(project_id).devices;
            let resolved = resolve(&mut t, pid, &mut devices, &BTreeSet::new())?;
            let Some(e) = resolved.entries.iter().find(|e| e.id == id) else {
                bail!(tr!(
                    "Ce souvenir n’est plus dans la mémoire de l’équipe.",
                    "This memory is no longer in the team’s memory."
                ));
            };
            if !e.mine && !resolved.admin {
                bail!(tr!(
                    "Seul son auteur ou un administrateur de l’équipe valide un souvenir.",
                    "Only its author or an administrator of the team validates a memory."
                ));
            }
            if keep {
                let fields = t
                    .replica(&pid)
                    .map_err(say)?
                    .object(&object)
                    .map_err(say)?
                    .map(|v| v.fields)
                    .unwrap_or_default();
                t.set_field(pid, object, "accepted", json!(digest(&fields)))
                    .map_err(say)?;
                if fields.get("rejected") == Some(&Value::Bool(true)) {
                    t.set_field(pid, object, "rejected", Value::Bool(false))
                        .map_err(say)?;
                }
            } else {
                t.set_field(pid, object, "rejected", Value::Bool(true))
                    .map_err(say)?;
            }
            self.team_memory_after(&project, &mut t, pid).await?;
        }
        self.team_memory_view(project_id).await
    }

    /// A quarantine (an administrator's): what it would take out (`apply` false), or the rule written in the journal,
    /// which holds for every machine, now and for what comes later.
    pub(crate) async fn team_memory_quarantine(
        &self,
        project_id: &str,
        rule: Rule,
        apply: bool,
    ) -> Result<QuarantinePreview> {
        let rule = rule.cleaned();
        if rule.is_empty() {
            bail!(tr!(
                "Choisis ce que la quarantaine retire : l’agent d’un ticket, un membre ou un appareil, depuis une date.",
                "Choose what the quarantine takes out: a ticket’s agent, a member or a device, since a date."
            ));
        }
        let project = self.project(project_id)?;
        let (shared, pid) = self.team_memory_link(&project)?;
        let one = crate::teamdocs::pass_lock(project_id);
        let _one = one.lock().await;
        let mut t = shared.lock().await;
        if !t.is_admin().map_err(say)? {
            bail!(tr!(
                "Seul un administrateur de l’équipe met des souvenirs en quarantaine.",
                "Only an administrator of the team puts memories in quarantine."
            ));
        }
        let mut devices = self.team_memory_manifest(project_id).devices;
        let resolved = resolve(&mut t, pid, &mut devices, &BTreeSet::new())?;
        let entries: Vec<Entry> = resolved
            .entries
            .iter()
            .filter(|e| e.quarantined.is_none() && rule.takes(&e.provenance))
            .cloned()
            .collect();
        let mut out = QuarantinePreview {
            rule: rule.clone(),
            entries,
            applied: None,
        };
        if apply {
            let object = ObjectId(*uuid::Uuid::new_v4().as_bytes());
            let fields: BTreeMap<String, Value> = [
                ("rule".to_string(), serde_json::to_value(&rule)?),
                ("byName".to_string(), json!(self.me_in(&project).name)),
            ]
            .into_iter()
            .collect();
            t.write(
                pid,
                OpBody::Create {
                    object,
                    kind: RULE.into(),
                    fields,
                },
            )
            .map_err(say)?;
            self.team_memory_after(&project, &mut t, pid).await?;
            out.applied = Some(hex_id(&object));
        }
        Ok(out)
    }

    /// « Lever la quarantaine »: the rule lifted (an administrator's), what it took out read again.
    pub(crate) async fn team_memory_lift(
        &self,
        project_id: &str,
        rule_id: &str,
    ) -> Result<TeamMemoryView> {
        let project = self.project(project_id)?;
        let (shared, pid) = self.team_memory_link(&project)?;
        let object = id_of(rule_id).ok_or_else(|| anyhow!("an id of 32 hexadecimal digits"))?;
        {
            let one = crate::teamdocs::pass_lock(project_id);
            let _one = one.lock().await;
            let mut t = shared.lock().await;
            if !t.is_admin().map_err(say)? {
                bail!(tr!(
                    "Seul un administrateur de l’équipe lève une quarantaine.",
                    "Only an administrator of the team lifts a quarantine."
                ));
            }
            t.set_field(pid, object, "lifted", Value::Bool(true))
                .map_err(say)?;
            self.team_memory_after(&project, &mut t, pid).await?;
        }
        self.team_memory_view(project_id).await
    }

    /// « Valider les souvenirs des agents » turned on or off for the project (C14-B): refused when its team imposes it.
    pub(crate) fn team_memory_set_review(&self, project_id: &str, on: bool) -> Result<Project> {
        let project = self.project(project_id)?;
        if let (_, Some(team)) = self.team_memory_review(&project) {
            bail!(tr!(
                "Imposé par l’équipe {team} : la validation des souvenirs ne se change pas ici.",
                "Imposed by the team {team}: the review of memories is not changed here."
            ));
        }
        let p = {
            let mut projects = self.projects.write();
            let p = projects
                .iter_mut()
                .find(|p| p.id == project_id)
                .ok_or_else(crate::core::project_not_found)?;
            p.memory_review_off = !on;
            p.clone()
        };
        self.hub.emit(UiEvent::Project { project: p.clone() });
        self.request_save();
        Ok(p)
    }

    /// The memories a ticket's agent proposed (its key), that this member may keep or set aside: « Ce ticket a laissé
    /// 2 souvenirs » at its validation.
    pub(crate) async fn team_memory_of_ticket(
        &self,
        project_id: &str,
        key: &str,
    ) -> Result<Vec<Entry>> {
        let v = self.team_memory_view(project_id).await?;
        Ok(v.proposals
            .into_iter()
            .filter(|e| {
                e.provenance
                    .agent
                    .as_ref()
                    .and_then(|a| a.ticket.as_deref())
                    .is_some_and(|k| k.eq_ignore_ascii_case(key))
            })
            .collect())
    }
}

#[cfg(test)]
#[path = "teammemory_tests.rs"]
mod tests;
