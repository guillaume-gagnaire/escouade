//! The layers of the settings and the lock (docs/TEAM-SERVER.md §6.1, §6.2, §6.4 and §6.5, TS-11).
//!
//! Six ranks, from the weakest to the strongest:
//! 1. Escouade's defaults;
//! 2. the team, recommended (the `defaults` of the team's `.escouade.yml`);
//! 3. the repository (`.escouade.yml`);
//! 4. the device (Réglages, « Réglage local »);
//! 5. the team, imposed (the `enforced` of the team's file, signed);
//! 6. the managed device (the policies `escouade-policy` reads: ADMX, MDM profile, `/etc`).
//!
//! Ranks 2 to 4 keep today's order; ranks 5 and 6 go before everything. Between them, for one key: a **bound** (the
//! modes allowed, the harnesses allowed) crosses, what both allow is left; a **value** of the managed device wins. Each
//! key has its sort ([`Sort`]): an executable one is never imposed by the team, a sensitive one waits for each member's
//! consent (TS-13) and is not imposed by the team either until then.
//!
//! [`KEYS`] is the one table of the keys known so far: each with its sort, its form and the rows of the settings it
//! locks (`SETTINGS_ROWS`, src/lib/settings-pages.ts). TS-12 completes it and generates the ADMX from it.
//!
//! Rank 6 is read once at the start (`Managed::read`); rank 5 has no source yet (the team server comes later): it is
//! injectable (`Core::set_team_policies`) and empty in the app. The core refuses a write of what is locked, by the
//! window, the MCP and the `.escouade.yml` (`Locks::check_settings`, `Locks::check_project`, `Locks::file_refusal`).

use crate::model::{AgentMeta, Project, Settings};
use anyhow::{bail, Result};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

/// What a key may do, when an administrator imposes it (§6.5).
// Sensitive and Executable: no key of the app's table has them yet (TS-12 and TS-13 add them); the engine knows them.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Sort {
    /// It turns a function off or narrows a list: applied at once. The worst a pirate gets of it is a denial of service.
    Restrict,
    /// A value (a target branch, a prefix): applied and shown.
    Value,
    /// A proxy's address, a file of authorities: waits for each member's consent when it is the team's (TS-13).
    Sensitive,
    /// A command, a hook, a plugin to install: never imposed by the team; the managed device alone may.
    Executable,
}

/// The shape of a key's value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    /// On or off. For a key that restricts, only `restricted` may be imposed (« Statistiques » off, never on).
    Flag { restricted: Option<bool> },
    /// One of `choices`; for a key that restricts, only `restricted`.
    Choice {
        choices: &'static [&'static str],
        restricted: Option<&'static str>,
    },
    /// A list of what is allowed: crossed between the ranks that set it. `universe`: what it may name (none: any name,
    /// as the harnesses the plugins bring).
    Bound {
        universe: Option<&'static [&'static str]>,
    },
}

/// A key of the table.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Copy, Debug)]
pub struct LockKey {
    /// As the policies name it (`permissions.allowBypass`).
    pub key: &'static str,
    pub sort: Sort,
    pub form: Form,
    /// The rows of the settings' register it locks, or whose choices it bounds (`SETTINGS_ROWS` ids).
    pub rows: &'static [&'static str],
    /// The bound a value of it stays within (`permissions.defaultMode` within `permissions.modes`).
    pub within: Option<&'static str>,
}

/// The modes a bound of modes may name, from the one that allows the least to the one that allows the most.
pub const MODES: &[&str] = &crate::handover::MODES;
/// The harness's mode of no question (`bypass::BYPASS`).
const BYPASS: &str = crate::bypass::BYPASS;

pub const ALLOW_BYPASS: &str = "permissions.allowBypass";
pub const MODES_KEY: &str = "permissions.modes";
pub const DEFAULT_MODE: &str = "permissions.defaultMode";
pub const HARNESSES: &str = "harnesses.allowed";
pub const TELEMETRY: &str = "telemetry.enabled";
pub const CHANNEL: &str = "updates.channel";
pub const REMOTE: &str = "remoteControl.enabled";

/// The one table of the keys known so far (§6.4). Each says its sort and the rows it locks.
pub const KEYS: &[LockKey] = &[
    LockKey {
        key: ALLOW_BYPASS,
        sort: Sort::Restrict,
        form: Form::Flag {
            restricted: Some(false),
        },
        rows: &["allowBypass"],
        within: None,
    },
    LockKey {
        key: MODES_KEY,
        sort: Sort::Restrict,
        form: Form::Bound {
            universe: Some(MODES),
        },
        rows: &["agentMode", "mode"],
        within: None,
    },
    LockKey {
        key: DEFAULT_MODE,
        sort: Sort::Value,
        form: Form::Choice {
            choices: &["plan", "default", "acceptEdits", "auto"],
            restricted: None,
        },
        rows: &["agentMode"],
        within: Some(MODES_KEY),
    },
    LockKey {
        key: HARNESSES,
        sort: Sort::Restrict,
        form: Form::Bound { universe: None },
        rows: &["projectEngine"],
        within: None,
    },
    LockKey {
        key: TELEMETRY,
        sort: Sort::Restrict,
        form: Form::Flag {
            restricted: Some(false),
        },
        rows: &["telemetry"],
        within: None,
    },
    LockKey {
        key: CHANNEL,
        sort: Sort::Restrict,
        form: Form::Choice {
            choices: &["stable", "beta"],
            restricted: Some("stable"),
        },
        rows: &["betaUpdates"],
        within: None,
    },
    LockKey {
        key: REMOTE,
        sort: Sort::Restrict,
        form: Form::Flag {
            restricted: Some(false),
        },
        rows: &["remoteAll", "projectRemote"],
        within: None,
    },
];

/// The key of the table named so.
pub fn key_of<'a>(table: &'a [LockKey], key: &str) -> Option<&'a LockKey> {
    table.iter().find(|k| k.key == key)
}

/// The names of the table's keys: what `escouade-policy` knows.
pub fn names() -> Vec<&'static str> {
    KEYS.iter().map(|k| k.key).collect()
}

// ---------- the ranks ----------

/// The six ranks of §6.1. The app reads ranks 1 to 4 where they are today (the settings, the project and its file) and
/// brings them within ranks 5 and 6 (`Locks::clamp_settings`, `clamp_project`); `resolve` is the whole rule, which the
/// team's file (TS-13) reads through.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Rank {
    Defaults = 1,
    TeamRecommended = 2,
    Repository = 3,
    Device = 4,
    TeamEnforced = 5,
    Managed = 6,
}

/// Who imposes a lock: the policy of this device, or a team.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum LockedBy {
    /// Rank 6: where it was read (« the machine's policy », a profile, a file of /etc).
    Device { source: String },
    /// Rank 5: the team's name.
    Team { team: String },
}

impl LockedBy {
    pub fn rank(&self) -> Rank {
        match self {
            LockedBy::Device { .. } => Rank::Managed,
            LockedBy::Team { .. } => Rank::TeamEnforced,
        }
    }

    /// « par la stratégie de ce poste », « par l'équipe Acme ».
    pub fn by(&self) -> String {
        match self {
            LockedBy::Device { .. } => {
                tr!("par la stratégie de ce poste", "by this device’s policy")
            }
            LockedBy::Team { team } => tr!("par l’équipe {team}", "by the team {team}"),
        }
    }
}

/// What a team sets (ranks 2 and 5): injectable, none in the app until the team server comes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TeamPolicies {
    pub team: String,
    /// `defaults`: recommended, changed by the repository and the device (rank 2).
    pub recommended: BTreeMap<String, Value>,
    /// `enforced`: imposed (rank 5).
    pub enforced: BTreeMap<String, Value>,
}

/// What the managed device sets (rank 6): each value with where it was read.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Managed {
    pub values: BTreeMap<String, (Value, String)>,
    /// What reading said: to the log, and to Réglages.
    pub warnings: Vec<String>,
}

impl Managed {
    /// The policies `escouade-policy` read, with the table's keys.
    pub fn of(policies: &escouade_policy::Policies) -> Self {
        Managed {
            values: policies
                .iter()
                .map(|p| (p.key.clone(), (p.value.clone(), p.source.to_string())))
                .collect(),
            warnings: policies.warnings().iter().map(|w| w.to_string()).collect(),
        }
    }

    /// The policies of this device, read once at the start. In a build of development only, `ESCOUADE_TEST_POLICIES`
    /// names a JSON file (`{"telemetry.enabled": false}`) read in their place: the e2e's source. Never in a release
    /// (SEC-13 of the audit of 2.2.0: a variable must not lift what an administrator imposes, nor impose anything).
    pub fn read() -> Self {
        #[cfg(debug_assertions)]
        if let Some(managed) = test_source() {
            return managed;
        }
        Self::of(&escouade_policy::read_known(&names()))
    }
}

/// The e2e's source of policies: a JSON file named by `ESCOUADE_TEST_POLICIES`, in a build of development alone.
#[cfg(debug_assertions)]
fn test_source() -> Option<Managed> {
    let path = std::env::var_os("ESCOUADE_TEST_POLICIES")?;
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => {
            log::warn!("policies: test source not read: {e}");
            return None;
        }
    };
    let map: BTreeMap<String, Value> = match serde_json::from_str(&text) {
        Ok(m) => m,
        Err(e) => {
            log::warn!("policies: test source is not a JSON object: {e}");
            return None;
        }
    };
    let source = std::path::PathBuf::from(path).display().to_string();
    Some(Managed {
        values: map
            .into_iter()
            .map(|(k, v)| (k, (v, source.clone())))
            .collect(),
        warnings: Vec::new(),
    })
}

// ---------- the locks ----------

/// One rank's list of a bound, with who set it.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bound {
    pub by: LockedBy,
    pub allowed: Vec<String>,
}

/// What ranks 5 and 6 impose on a key: a value, or bounds (each rank's list; what all allow is left).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Lock {
    pub key: String,
    /// The value imposed, and who imposes it.
    pub value: Option<Value>,
    pub by: Option<LockedBy>,
    pub bounds: Vec<Bound>,
}

impl Lock {
    /// What every bound allows; None when there is none.
    pub fn allowed(&self) -> Option<Vec<String>> {
        let mut it = self.bounds.iter();
        let first = it.next()?;
        Some(
            first
                .allowed
                .iter()
                .filter(|x| self.bounds.iter().all(|b| b.allowed.contains(x)))
                .cloned()
                .collect(),
        )
    }

    /// Who denies `choice`, the device first; None when every bound allows it.
    pub fn denied_by(&self, choice: &str) -> Option<&LockedBy> {
        let mut bounds: Vec<&Bound> = self.bounds.iter().collect();
        bounds.sort_by_key(|b| std::cmp::Reverse(b.by.rank()));
        bounds
            .into_iter()
            .find(|b| !b.allowed.iter().any(|a| a == choice))
            .map(|b| &b.by)
    }
}

/// The locks of the moment, one per key locked, and what was left out while making them.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Locks {
    pub locks: BTreeMap<String, Lock>,
    /// Each with the row it locks: the window greys a row by it (`locked_by`).
    pub rows: BTreeMap<String, Vec<String>>,
    pub warnings: Vec<String>,
}

/// A value of a rank, read in the key's form; Err says why it is left out.
fn normalize(k: &LockKey, v: &Value) -> std::result::Result<Value, String> {
    match k.form {
        Form::Flag { .. } => match v {
            Value::Bool(b) => Ok(Value::Bool(*b)),
            // A Boolean ADMX is a REG_DWORD of 0 or 1.
            Value::Number(n) if n.as_u64() == Some(0) => Ok(Value::Bool(false)),
            Value::Number(n) if n.as_u64() == Some(1) => Ok(Value::Bool(true)),
            _ => Err(tr!("un booléen est attendu", "a Boolean is expected")),
        },
        Form::Choice { choices, .. } => match v.as_str() {
            Some(s) if choices.contains(&s) => Ok(Value::String(s.to_owned())),
            _ => Err(tr!(
                "une de ces valeurs est attendue : {list}",
                "one of these values is expected: {list}",
                list = choices.join(", ")
            )),
        },
        Form::Bound { universe } => {
            let items: Vec<&Value> = match v {
                Value::Array(a) => a.iter().collect(),
                Value::String(_) => vec![v],
                _ => return Err(tr!("une liste est attendue", "a list is expected")),
            };
            let mut out = Vec::new();
            for item in items {
                let Some(s) = item.as_str().map(str::trim).filter(|s| !s.is_empty()) else {
                    return Err(tr!(
                        "une liste de noms est attendue",
                        "a list of names is expected"
                    ));
                };
                if universe.is_some_and(|u| !u.contains(&s)) {
                    return Err(tr!("« {s} » n’est pas connu", "“{s}” is not known"));
                }
                if !out.iter().any(|o: &Value| o == s) {
                    out.push(Value::String(s.to_owned()));
                }
            }
            Ok(Value::Array(out))
        }
    }
}

/// Whether a value imposed only restricts: for a key that restricts, its restricted value.
fn restricts(k: &LockKey, v: &Value) -> bool {
    match k.form {
        Form::Flag {
            restricted: Some(r),
        } => v == &Value::Bool(r),
        Form::Choice {
            restricted: Some(r),
            ..
        } => v.as_str() == Some(r),
        _ => true,
    }
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

impl Locks {
    /// What ranks 5 (`team`) and 6 (`managed`) impose on the keys of `table`.
    pub fn compute(table: &[LockKey], team: Option<&TeamPolicies>, managed: &Managed) -> Locks {
        let mut out = Locks {
            warnings: managed.warnings.clone(),
            ..Default::default()
        };
        for key in managed.values.keys() {
            if key_of(table, key).is_none() {
                out.warnings.push(tr!(
                    "stratégie « {key} » du poste : clé inconnue, ignorée",
                    "policy “{key}” of the device: unknown key, ignored"
                ));
            }
        }
        if let Some(team) = team {
            for key in team.enforced.keys() {
                if key_of(table, key).is_none() {
                    out.warnings.push(tr!(
                        "réglage « {key} » de l’équipe {name} : clé inconnue, ignorée",
                        "setting “{key}” of the team {name}: unknown key, ignored",
                        name = team.team
                    ));
                }
            }
        }
        for k in table {
            let mut layers: Vec<(LockedBy, Value)> = Vec::new();
            if let Some((v, source)) = managed.values.get(k.key) {
                let by = LockedBy::Device {
                    source: source.clone(),
                };
                match normalize(k, v) {
                    Ok(v) if restricts(k, &v) => layers.push((by, v)),
                    Ok(_) => out.warnings.push(tr!(
                        "stratégie « {key} » du poste ignorée : elle ne peut que restreindre",
                        "policy “{key}” of the device ignored: it may only restrict",
                        key = k.key
                    )),
                    Err(why) => out.warnings.push(tr!(
                        "stratégie « {key} » du poste ignorée : {why}",
                        "policy “{key}” of the device ignored: {why}",
                        key = k.key
                    )),
                }
            }
            if let Some(team) = team {
                if let Some(v) = team.enforced.get(k.key) {
                    let name = &team.team;
                    let by = LockedBy::Team { team: name.clone() };
                    match (k.sort, normalize(k, v)) {
                        // §6.5: never imposed by a team, whatever it signs.
                        (Sort::Executable, _) => out.warnings.push(tr!(
                            "« {key} » de l’équipe {name} ignoré : une commande n’est jamais imposée par l’équipe",
                            "“{key}” of the team {name} ignored: a command is never imposed by the team",
                            key = k.key
                        )),
                        (Sort::Sensitive, _) => out.warnings.push(tr!(
                            "« {key} » de l’équipe {name} en attente : il demande l’accord de ce poste",
                            "“{key}” of the team {name} pending: it needs this device’s consent",
                            key = k.key
                        )),
                        (_, Ok(v)) if restricts(k, &v) => layers.push((by, v)),
                        (_, Ok(_)) => out.warnings.push(tr!(
                            "« {key} » de l’équipe {name} ignoré : il ne peut que restreindre",
                            "“{key}” of the team {name} ignored: it may only restrict",
                            key = k.key
                        )),
                        (_, Err(why)) => out.warnings.push(tr!(
                            "« {key} » de l’équipe {name} ignoré : {why}",
                            "“{key}” of the team {name} ignored: {why}",
                            key = k.key
                        )),
                    }
                }
            }
            if layers.is_empty() {
                continue;
            }
            let lock = if matches!(k.form, Form::Bound { .. }) {
                Lock {
                    key: k.key.to_owned(),
                    value: None,
                    by: None,
                    bounds: layers
                        .into_iter()
                        .map(|(by, v)| Bound {
                            by,
                            allowed: strings(&v),
                        })
                        .collect(),
                }
            } else {
                // A value: the managed device's wins (it comes first).
                let (by, value) = layers.into_iter().next().expect("one layer at least");
                Lock {
                    key: k.key.to_owned(),
                    value: Some(value),
                    by: Some(by),
                    bounds: Vec::new(),
                }
            };
            for row in k.rows {
                out.rows
                    .entry((*row).to_owned())
                    .or_default()
                    .push(k.key.to_owned());
            }
            out.locks.insert(k.key.to_owned(), lock);
        }
        out
    }

    pub fn get(&self, key: &str) -> Option<&Lock> {
        self.locks.get(key)
    }

    /// The value imposed on `key`.
    pub fn value(&self, key: &str) -> Option<&Value> {
        self.locks.get(key)?.value.as_ref()
    }

    /// Who imposes the value of `key`.
    fn by(&self, key: &str) -> Option<&LockedBy> {
        self.locks.get(key)?.by.as_ref()
    }

    /// Whether `choice` is allowed by the bound `key` (every choice when it has none).
    pub fn allows(&self, key: &str, choice: &str) -> bool {
        self.locks
            .get(key)
            .is_none_or(|l| l.denied_by(choice).is_none())
    }

    /// Why `choice` is not allowed by the bound `key`: « Le harness codex n'est pas permis par la stratégie de ce poste. »
    pub fn denial(&self, key: &str, choice: &str) -> Option<String> {
        let by = self.locks.get(key)?.denied_by(choice)?.by();
        Some(if key == MODES_KEY {
            tr!(
                "Le mode « {choice} » n’est pas permis {by}.",
                "The mode “{choice}” is not allowed {by}."
            )
        } else if key == HARNESSES {
            tr!(
                "Le harness « {choice} » n’est pas permis {by}.",
                "The harness “{choice}” is not allowed {by}."
            )
        } else {
            tr!(
                "« {choice} » n’est pas permis {by} ({key}).",
                "“{choice}” is not allowed {by} ({key})."
            )
        })
    }

    /// The mode the nearest to `mode` among those allowed, that never allows more (`handover::closest_mode`); the one
    /// that allows the least when every one allows more. `mode` itself when it is allowed.
    pub fn nearest_mode(&self, mode: &str) -> String {
        let mut allowed: Vec<String> = self
            .get(MODES_KEY)
            .and_then(Lock::allowed)
            .unwrap_or_else(|| MODES.iter().map(|m| (*m).to_owned()).collect());
        if self.off(ALLOW_BYPASS) {
            allowed.retain(|m| m != BYPASS);
        }
        let allowed: Vec<&str> = allowed.iter().map(String::as_str).collect();
        crate::handover::closest_mode(mode, &allowed)
            .or_else(|| {
                MODES
                    .iter()
                    .find(|m| allowed.contains(m))
                    .map(|m| (*m).to_owned())
            })
            // Nothing allowed at all: the one that asks before every change.
            .unwrap_or_else(|| "plan".to_owned())
    }

    /// The mode an agent goes in, `mode` asked: refused when a bound denies it, or when the mode of no question is
    /// locked off.
    pub fn check_mode(&self, mode: &str) -> Result<()> {
        if mode.is_empty() {
            return Ok(());
        }
        if mode == BYPASS && self.value(ALLOW_BYPASS) == Some(&Value::Bool(false)) {
            bail!(self.imposed(ALLOW_BYPASS));
        }
        if let Some(why) = self.denial(MODES_KEY, mode) {
            bail!(why);
        }
        Ok(())
    }

    /// The harness `engine`, refused when a bound denies it.
    pub fn check_engine(&self, engine: &str) -> Result<()> {
        match self.denial(HARNESSES, engine) {
            Some(why) => bail!(why),
            None => Ok(()),
        }
    }

    /// « Ce réglage est imposé par la stratégie de ce poste (telemetry.enabled). »
    pub fn imposed(&self, key: &str) -> String {
        let by = self
            .by(key)
            .map(LockedBy::by)
            .unwrap_or_else(|| tr!("par la stratégie de ce poste", "by this device’s policy"));
        tr!(
            "Ce réglage est imposé {by} ({key}) : il ne se change pas sur ce poste.",
            "This setting is imposed {by} ({key}): it cannot be changed on this device."
        )
    }

    /// The flag `key` locked off.
    fn off(&self, key: &str) -> bool {
        self.value(key) == Some(&Value::Bool(false))
    }

    // ---------- what they do to the settings ----------

    /// The app's settings brought within the locks; the keys that changed them.
    pub fn clamp_settings(&self, s: &mut Settings) -> Vec<&'static str> {
        let mut changed = Vec::new();
        if self.off(ALLOW_BYPASS) && s.allow_bypass != Some(false) {
            s.allow_bypass = Some(false);
            changed.push(ALLOW_BYPASS);
        }
        if self.off(TELEMETRY) && s.telemetry != Some(false) {
            s.telemetry = Some(false);
            changed.push(TELEMETRY);
        }
        if self.value(CHANNEL).and_then(Value::as_str) == Some("stable") && s.beta_updates {
            s.beta_updates = false;
            changed.push(CHANNEL);
        }
        if self.off(REMOTE) && s.remote_control_all {
            s.remote_control_all = false;
            changed.push(REMOTE);
        }
        if let Some(mode) = self.value(DEFAULT_MODE).and_then(Value::as_str) {
            let mode = self.nearest_mode(mode);
            if s.agent_mode != mode {
                s.agent_mode = mode;
                changed.push(DEFAULT_MODE);
            }
        }
        let effective = |m: &str| {
            if m.is_empty() {
                crate::accounts::RECOMMENDED_MODE.to_owned()
            } else {
                m.to_owned()
            }
        };
        let agent_mode = effective(&s.agent_mode);
        if !self.allows(MODES_KEY, &agent_mode) {
            s.agent_mode = self.nearest_mode(&agent_mode);
            changed.push(MODES_KEY);
        }
        for a in &mut s.accounts {
            if !a.default_mode.is_empty() && self.check_mode(&a.default_mode).is_err() {
                a.default_mode = self.nearest_mode(&a.default_mode);
                if a.default_mode == BYPASS {
                    a.default_mode.clear();
                }
                if !changed.contains(&MODES_KEY) {
                    changed.push(MODES_KEY);
                }
            }
        }
        changed
    }

    /// A save of the settings refused when it changes what is locked: said, with who imposes it.
    pub fn check_settings(&self, new: &Settings) -> Result<()> {
        let mut clamped = new.clone();
        match self.clamp_settings(&mut clamped).first() {
            None => Ok(()),
            Some(&MODES_KEY) => {
                let asked = if new.agent_mode.is_empty() {
                    new.accounts
                        .iter()
                        .map(|a| a.default_mode.as_str())
                        .find(|m| !m.is_empty() && !self.allows(MODES_KEY, m))
                        .unwrap_or(crate::accounts::RECOMMENDED_MODE)
                        .to_owned()
                } else if self.allows(MODES_KEY, &new.agent_mode) {
                    new.accounts
                        .iter()
                        .map(|a| a.default_mode.clone())
                        .find(|m| !m.is_empty() && self.check_mode(m).is_err())
                        .unwrap_or_default()
                } else {
                    new.agent_mode.clone()
                };
                bail!(self
                    .denial(MODES_KEY, &asked)
                    .unwrap_or_else(|| self.imposed(ALLOW_BYPASS)))
            }
            Some(key) => bail!(self.imposed(key)),
        }
    }

    /// A project brought within the locks: its Kanban's mode, its remote control. Its harness stays (an agent of a
    /// harness not allowed is refused at its start). The keys that changed it.
    pub fn clamp_project(&self, p: &mut Project) -> Vec<&'static str> {
        let mut changed = Vec::new();
        if !p.board.mode.is_empty() && self.check_mode(&p.board.mode).is_err() {
            let m = self.nearest_mode(&p.board.mode);
            p.board.mode = if m == BYPASS { String::new() } else { m };
            changed.push(MODES_KEY);
        }
        if self.off(REMOTE) && p.remote_control {
            p.remote_control = false;
            changed.push(REMOTE);
        }
        changed
    }

    /// A change of a project refused when it sets what is locked: a mode or a harness not allowed, the remote control.
    pub fn check_project(&self, before: &Project, after: &Project) -> Result<()> {
        if after.board.mode != before.board.mode {
            self.check_mode(&after.board.mode)?;
        }
        if after.engine() != before.engine() {
            self.check_engine(after.engine().as_str())?;
        }
        if after.remote_control && !before.remote_control && self.off(REMOTE) {
            bail!(self.imposed(REMOTE));
        }
        Ok(())
    }

    /// Why the `.escouade.yml` of the repository may not set `key` to `v` here: a mode or a harness not allowed.
    pub fn file_refusal(&self, key: &str, v: &Value) -> Option<String> {
        let s = v.as_str().unwrap_or_default();
        match key {
            "board.mode" if !s.is_empty() => self.check_mode(s).err().map(|e| e.to_string()),
            "agents.engine" if !s.is_empty() => self.check_engine(s).err().map(|e| e.to_string()),
            _ => None,
        }
    }

    /// Whether « Exporter vers .escouade.yml » leaves `key` out: its value is bounded here by a lock, which the other
    /// devices of the repository may not have.
    pub fn bounds_file_key(&self, key: &str) -> bool {
        match key {
            "board.mode" => self.get(MODES_KEY).is_some() || self.get(ALLOW_BYPASS).is_some(),
            "agents.engine" => self.get(HARNESSES).is_some(),
            _ => false,
        }
    }

    /// The mode an agent in `meta` goes to, when its own is no longer allowed.
    pub fn agent_mode_after(&self, meta: &AgentMeta) -> Option<String> {
        if meta.mode.is_empty() || self.check_mode(&meta.mode).is_ok() {
            return None;
        }
        let m = self.nearest_mode(&meta.mode);
        Some(if m == BYPASS { "plan".to_owned() } else { m })
    }
}

// ---------- the six ranks, resolved ----------

/// The six ranks of one key set, each a value when it sets it.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug, Default)]
pub struct Layers {
    pub defaults: BTreeMap<String, Value>,
    pub repository: BTreeMap<String, Value>,
    pub device: BTreeMap<String, Value>,
    /// Ranks 2 and 5.
    pub team: Option<TeamPolicies>,
    /// Rank 6.
    pub managed: Managed,
}

/// A key's value, and the rank it comes from; who locks it, when ranks 5 or 6 do.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug, PartialEq)]
pub struct Resolved {
    pub value: Value,
    pub rank: Rank,
    pub locked_by: Option<LockedBy>,
}

/// The value of `key` through the six ranks: ranks 2 to 4 in today's order (the device before the repository before
/// the team's recommendation before the defaults), then the locks of ranks 5 and 6 before everything. A bound's list is
/// crossed with the locks'; a value within a bound goes to the nearest allowed.
#[cfg_attr(not(test), allow(dead_code))]
pub fn resolve(table: &[LockKey], layers: &Layers, key: &str) -> Option<Resolved> {
    let k = key_of(table, key)?;
    let locks = Locks::compute(table, layers.team.as_ref(), &layers.managed);
    let recommended = layers.team.as_ref().map(|t| &t.recommended);
    let lower = [
        (Rank::Device, layers.device.get(key)),
        (Rank::Repository, layers.repository.get(key)),
        (Rank::TeamRecommended, recommended.and_then(|r| r.get(key))),
        (Rank::Defaults, layers.defaults.get(key)),
    ]
    .into_iter()
    .find_map(|(rank, v)| v.map(|v| (rank, v.clone())));
    let lock = locks.get(key);
    if let (Some(lock), Some(value)) = (lock, lock.and_then(|l| l.value.clone())) {
        let by = lock.by.clone().expect("a value has who imposes it");
        let value = within(k, &locks, value);
        return Some(Resolved {
            rank: by.rank(),
            value,
            locked_by: Some(by),
        });
    }
    if let Some(lock) = lock.filter(|l| !l.bounds.is_empty()) {
        let allowed = lock.allowed().unwrap_or_default();
        let value = match lower.as_ref().map(|(_, v)| strings(v)) {
            Some(list) => list.into_iter().filter(|x| allowed.contains(x)).collect(),
            None => allowed,
        };
        let strongest = lock
            .bounds
            .iter()
            .map(|b| b.by.clone())
            .max_by_key(LockedBy::rank)
            .expect("a bound");
        return Some(Resolved {
            value: Value::from(value),
            rank: strongest.rank(),
            locked_by: Some(strongest),
        });
    }
    let (rank, value) = lower?;
    Some(Resolved {
        value: within(k, &locks, value),
        rank,
        locked_by: None,
    })
}

/// A value kept within its bound: the nearest allowed (a mode by `nearest_mode`, else the first allowed).
#[cfg_attr(not(test), allow(dead_code))]
fn within(k: &LockKey, locks: &Locks, value: Value) -> Value {
    let Some(bound) = k.within.and_then(|b| locks.get(b)) else {
        return value;
    };
    let Some(s) = value.as_str() else {
        return value;
    };
    if bound.denied_by(s).is_none() {
        return value;
    }
    if k.within == Some(MODES_KEY) {
        return Value::String(locks.nearest_mode(s));
    }
    bound
        .allowed()
        .and_then(|a| a.into_iter().next())
        .map(Value::String)
        .unwrap_or(value)
}

// ---------- in the core ----------

impl<R: tauri::Runtime> crate::core::Core<R> {
    /// Rank 5, as a team sets it: none in the app until the team server comes (TS-13). The locks are made again and
    /// applied at once.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn set_team_policies(self: &std::sync::Arc<Self>, team: Option<TeamPolicies>) {
        *self.team_policies.write() = team;
        self.apply_locks();
    }

    /// Rank 6, as read (`Managed::read` at the start; a test gives its own). The locks are made again and applied.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn set_managed(self: &std::sync::Arc<Self>, managed: Managed) {
        *self.managed.write() = managed;
        self.apply_locks();
    }

    /// The locks made again from ranks 5 and 6, and what they lock brought within them: the settings, the projects, the
    /// agents (each at the end of its turn). The window is told.
    pub(crate) fn apply_locks(self: &std::sync::Arc<Self>) {
        let locks = Locks::compute(
            KEYS,
            self.team_policies.read().as_ref(),
            &self.managed.read(),
        );
        for w in &locks.warnings {
            log::warn!("policies: {w}");
        }
        *self.locks.write() = locks;
        let changed = {
            let locks = self.locks.read();
            locks.clamp_settings(&mut self.settings.write())
        };
        if !changed.is_empty() {
            log::info!("policies: settings brought within {changed:?}");
            let telemetry = self.settings.read().telemetry;
            self.telemetry.set_consent(telemetry);
            self.write_settings_file();
        }
        self.settle_locks();
        self.hub.emit(crate::model::UiEvent::Locks {
            locks: self.locks.read().clone(),
        });
    }

    /// The projects brought within the locks, and each agent whose mode is no longer allowed moved to the nearest one
    /// that is: at once when it is not at work, at the end of its turn otherwise (`settle_locked_mode`).
    pub(crate) fn settle_locks(self: &std::sync::Arc<Self>) {
        if self.locks.read().locks.is_empty() {
            return;
        }
        // Taken apart, never held under the projects' lock.
        let locks = self.locks.read().clone();
        let projects: Vec<Project> = {
            let mut projects = self.projects.write();
            projects
                .iter_mut()
                .filter_map(|p| (!locks.clamp_project(p).is_empty()).then(|| p.clone()))
                .collect()
        };
        for project in projects {
            log::info!("policies: project {} brought within them", project.id);
            self.hub.emit(crate::model::UiEvent::Project { project });
            self.request_save();
        }
        let idle: Vec<String> = self
            .agents
            .read()
            .iter()
            .filter(|(_, h)| {
                let rt = h.lock();
                !rt.meta.status.is_active() && locks.agent_mode_after(&rt.meta).is_some()
            })
            .map(|(id, _)| id.clone())
            .collect();
        for id in idle {
            let c = self.clone();
            tauri::async_runtime::spawn(async move { c.settle_locked_mode(&id).await });
        }
    }

    /// An agent in a mode the locks no longer allow goes to the nearest one allowed, and says it in its conversation.
    /// Called once its turn is over (`turn_ended`), and at once for an agent not at work.
    pub(crate) async fn settle_locked_mode(self: &std::sync::Arc<Self>, id: &str) {
        let Ok(h) = self.agent(id) else { return };
        let (from, to) = {
            let rt = h.lock();
            (
                rt.meta.mode.clone(),
                self.locks.read().agent_mode_after(&rt.meta),
            )
        };
        let Some(to) = to else { return };
        if let Err(e) = self
            .set_agent_options(id, None, None, Some(to.clone()))
            .await
        {
            log::warn!("policies: agent {id} not moved to {to}: {e:#}");
            return;
        }
        let why = self
            .locks
            .read()
            .denial(MODES_KEY, &from)
            .unwrap_or_else(|| self.locks.read().imposed(ALLOW_BYPASS));
        let text = tr!(
            "{why} L’agent passe en mode « {to} », le plus proche permis.",
            "{why} The agent goes to the mode “{to}”, the nearest allowed."
        );
        let _ = self.with_agent(id, |rt, fx| {
            rt.notice("warning", text, fx);
            Ok(())
        });
    }
}

#[cfg(test)]
#[path = "locks_tests.rs"]
mod tests;
