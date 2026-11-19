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
//! [`KEYS`] is the single list of the policies (`escouade_policy::list`, TS-12): each key with its sort, its form, its
//! type and the rows of the settings it locks (`SETTINGS_ROWS`, src/lib/settings-pages.ts). The ADMX, the profiles, the
//! schema of /etc and the site's page are generated from it.
//!
//! Rank 6 is read at the start (`Managed::read`), then again at each change while the app runs (`Managed::watch`,
//! `Core::watch_policies`); rank 5 has no source yet (the team server comes later): it is
//! injectable (`Core::set_team_policies`) and empty in the app. The core refuses a write of what is locked, by the
//! window, the MCP and the `.escouade.yml` (`Locks::check_settings`, `Locks::check_project`, `Locks::file_refusal`).

use crate::model::{AgentMeta, Project, Settings};
use anyhow::{bail, Result};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

// The single list of the policies (TS-12, `escouade_policy::list`) is the core's table: each key with its sort, its form,
// its type and the rows of the settings it locks. The ADMX, the profiles, the schema of /etc and the site's page are
// generated from it. Some keys are only read and kept (their feature comes later): named by the tests alone.
#[allow(unused_imports)]
pub use escouade_policy::list::{
    Def as LockKey, Form, Sort, AGGREGATE_VIEW, ALLOW_BYPASS, AUTHORITIES, AUTO_UPDATE, CHANNEL,
    CONTENTLESS, DEFAULT_MODE, DENIED_FILES, DENY_INSECURE_TLS, DICTATION, EXTENSIONS_ALLOWED,
    FORCE_PRIVATE, HARNESSES, MAX_EFFORT, MCP_EXTERNAL, MODELS, MODES_KEY, ONLY_THIS_SERVER,
    PERSONAL_SPACE, PLUGINS_ALLOWED, PLUGINS_DENIED, PLUGIN_ORIGINS_KEY, POLICIES as KEYS,
    PROXY_EXCLUSIONS, PROXY_URL, REMOTE, REMOTE_API, SECRETS_FILE, SERVERS, TEAM_FINGERPRINT,
    TEAM_SERVER, TELEMETRY, VERIFIED_ONLY, VOICE_ONLINE,
};

/// The modes a bound of modes may name, from the one that allows the least to the one that allows the most
/// (`modes::ORDER`, one and the same).
pub const MODES: &[&str] = crate::modes::ORDER;
/// The efforts, from the least to the most.
pub const EFFORTS: &[&str] = escouade_policy::list::EFFORTS;
/// The harness's mode of no question (`bypass::BYPASS`).
const BYPASS: &str = crate::bypass::BYPASS;

/// The key of the table named so.
pub fn key_of<'a>(table: &'a [LockKey], key: &str) -> Option<&'a LockKey> {
    table.iter().find(|k| k.key == key)
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

/// What a team sets (ranks 2 and 5): from its signed `.escouade.yml` (TS-13, `team_settings.rs`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TeamPolicies {
    /// The team's id on this device: its projects name it (`Project::team`).
    pub id: String,
    /// Its name, as the locks say it (« par l'équipe Acme »).
    pub team: String,
    /// Who signed the version in force, and when (seconds): « signé par Claire Martin le 12 novembre ».
    pub signed: Option<(String, u64)>,
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
            // A key outside the list is said by `Locks::compute`, in the language of the interface.
            warnings: policies
                .warnings()
                .iter()
                .filter(|w| !matches!(w, escouade_policy::Warning::UnknownKey { .. }))
                .map(|w| w.to_string())
                .collect(),
        }
    }

    /// The policies of this device, read at the start (then watched, `Managed::watch`). In a build of development
    /// only, `ESCOUADE_TEST_POLICIES` names a JSON file (`{"telemetry.enabled": false}`) read in their place: the e2e's
    /// source, watched too. Never in a release (SEC-13 of the audit of 2.2.0: a variable must not lift what an
    /// administrator imposes, nor impose anything).
    pub fn read() -> Self {
        #[cfg(debug_assertions)]
        if let Some(managed) = test_source() {
            return Self::of(&managed.read());
        }
        Self::of(&escouade_policy::read())
    }

    /// The policies watched while the app runs (TS-12): each change is given to `on_change`, on the watch's thread. The
    /// registry is told by Windows, the files of `/Library/Managed Preferences` and `/etc/escouade` are looked at
    /// every second; everything is read again every 30 s as well.
    pub fn watch(on_change: impl FnMut(Managed) + Send + 'static) -> escouade_policy::watch::Watch {
        let mut on_change = on_change;
        let tell = move |p: escouade_policy::Policies| on_change(Managed::of(&p));
        #[cfg(debug_assertions)]
        if let Some(test) = test_source() {
            let changes = escouade_policy::watch::Poll::new(
                vec![test.path.clone()],
                std::time::Duration::from_millis(200),
            );
            let first = test.read();
            return escouade_policy::watch::spawn(
                move || test.read(),
                Box::new(changes),
                first,
                WATCH_PERIOD,
                tell,
            );
        }
        escouade_policy::watch::system_watch(escouade_policy::read(), WATCH_PERIOD, tell)
    }
}

/// How often the policies are read again, whatever the signs of the system.
const WATCH_PERIOD: std::time::Duration = std::time::Duration::from_secs(30);

/// The e2e's source of policies: a JSON file, read again at each change.
#[cfg(debug_assertions)]
struct TestSource {
    path: std::path::PathBuf,
}

#[cfg(debug_assertions)]
impl TestSource {
    /// Its policies, typed as the system's are; a file gone or broken gives none, said in the log.
    fn read(&self) -> escouade_policy::Policies {
        let text = match std::fs::read_to_string(&self.path) {
            Ok(t) => t,
            Err(e) => {
                log::warn!("policies: test source not read: {e}");
                return escouade_policy::Policies::default();
            }
        };
        match serde_json::from_str::<BTreeMap<String, Value>>(&text) {
            Ok(map) => escouade_policy::Policies::from_values(
                map,
                escouade_policy::Source::File(self.path.clone()),
            ),
            Err(e) => {
                log::warn!("policies: test source is not a JSON object: {e}");
                escouade_policy::Policies::default()
            }
        }
    }
}

/// The e2e's source of policies: a JSON file named by `ESCOUADE_TEST_POLICIES`, in a build of development alone.
#[cfg(debug_assertions)]
fn test_source() -> Option<TestSource> {
    let path = std::env::var_os("ESCOUADE_TEST_POLICIES")?;
    Some(TestSource { path: path.into() })
}

// ---------- the locks ----------

/// One rank's list of a bound, with who set it.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bound {
    pub by: LockedBy,
    pub allowed: Vec<String>,
}

/// One rank's value of a key, and who set it.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Imposed {
    pub by: LockedBy,
    pub value: Value,
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
    /// Each rank's value of a JSON key that restricts (`extensions.allowed`), the strongest first: each holds, crossed
    /// by whoever reads it (`check_extension`). Empty for the other keys.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub layers: Vec<Imposed>,
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
        Form::Choice { choices, .. } | Form::Ceiling { choices } => match v.as_str() {
            Some(s) if choices.contains(&s) => Ok(Value::String(s.to_owned())),
            _ => Err(tr!(
                "une de ces valeurs est attendue : {list}",
                "one of these values is expected: {list}",
                list = choices.join(", ")
            )),
        },
        Form::Bound { .. } | Form::Union => {
            let universe = k.form.choices();
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
        Form::Text => match v.as_str().map(str::trim) {
            Some(s) if !s.is_empty() => Ok(Value::String(s.to_owned())),
            _ => Err(tr!("une chaîne est attendue", "a string is expected")),
        },
        // Typed by escouade-policy already (a JSON string parsed): what is left is an object or not.
        Form::Json | Form::Number { .. } => escouade_policy::list::convert(k, v),
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
                        // What is sensitive comes here once this device consented to it (TS-13: `team_settings.rs`
                        // gives only those); a value, the managed device's wins.
                        (Sort::Sensitive, Ok(v)) => layers.push((by, v)),
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
                    layers: Vec::new(),
                }
            } else if let Form::Ceiling { choices } = k.form {
                // A ceiling: the lower one holds, the managed device's when they are equal.
                let rank = |v: &Value| choices.iter().position(|c| Some(*c) == v.as_str());
                let (by, value) = layers
                    .into_iter()
                    .min_by_key(|(_, v)| rank(v))
                    .expect("one layer at least");
                Lock {
                    key: k.key.to_owned(),
                    value: Some(value),
                    by: Some(by),
                    bounds: Vec::new(),
                    layers: Vec::new(),
                }
            } else if k.form == Form::Union {
                // What each rank denies or adds, together; said imposed by the strongest.
                let mut all: Vec<String> = Vec::new();
                for (_, v) in &layers {
                    for x in strings(v) {
                        if !all.contains(&x) {
                            all.push(x);
                        }
                    }
                }
                Lock {
                    key: k.key.to_owned(),
                    value: Some(Value::from(all)),
                    by: Some(layers[0].0.clone()),
                    bounds: Vec::new(),
                    layers: Vec::new(),
                }
            } else {
                // A value: the managed device's wins (it comes first).
                let each = if k.form == Form::Json && k.sort == Sort::Restrict {
                    layers
                        .iter()
                        .map(|(by, value)| Imposed {
                            by: by.clone(),
                            value: value.clone(),
                        })
                        .collect()
                } else {
                    Vec::new()
                };
                let (by, value) = layers.into_iter().next().expect("one layer at least");
                Lock {
                    key: k.key.to_owned(),
                    value: Some(value),
                    by: Some(by),
                    bounds: Vec::new(),
                    layers: each,
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

    /// The mode the nearest to `mode` among those allowed, that never allows more (`modes::closest`); the one
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
        crate::modes::closest(mode, &allowed)
            .or_else(|| {
                MODES
                    .iter()
                    .find(|m| allowed.contains(m))
                    .map(|m| (*m).to_owned())
            })
            // Nothing allowed at all: the one that asks before every change.
            .unwrap_or_else(|| "plan".to_owned())
    }

    /// The « Mode par défaut des agents » a new agent goes from: the one imposed (`permissions.defaultMode`, brought within
    /// the bound of the modes), else `chosen`, the app's. A team's holds for its projects alone (`Core::locks_for`),
    /// where the setting of the app does not have it (ESC-497).
    pub fn default_mode(&self, chosen: &str) -> String {
        match self.value(DEFAULT_MODE).and_then(Value::as_str) {
            Some(m) => self.nearest_mode(m),
            None => chosen.to_owned(),
        }
    }

    /// The mode the nearest to `mode` among those allowed that a harness of the modes `offered` runs in, never one that
    /// allows more (ESC-497): a harness of one mode runs in it, that one when it is allowed. None when none of its modes
    /// allowed allows as little.
    pub fn nearest_mode_of(&self, mode: &str, offered: &[&str]) -> Option<String> {
        let allowed: Vec<&str> = offered
            .iter()
            .copied()
            .filter(|m| self.check_mode(m).is_ok())
            .collect();
        match offered {
            [one] => allowed.contains(one).then(|| (*one).to_owned()),
            _ => crate::modes::closest(mode, &allowed),
        }
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

    /// The flag `key` locked on.
    fn on(&self, key: &str) -> bool {
        self.value(key) == Some(&Value::Bool(true))
    }

    /// The string imposed on `key`.
    pub fn text(&self, key: &str) -> Option<&str> {
        self.value(key).and_then(Value::as_str)
    }

    // ---------- what they refuse elsewhere than in the settings ----------

    /// The effort `effort`, refused above the ceiling `harnesses.maxEffort` (an effort a harness names that is not one
    /// of the list's passes).
    pub fn check_effort(&self, effort: &str) -> Result<()> {
        let Some(max) = self.text(MAX_EFFORT) else {
            return Ok(());
        };
        let rank = |e: &str| EFFORTS.iter().position(|x| *x == e);
        match (rank(effort), rank(max)) {
            (Some(e), Some(m)) if e > m => bail!(tr!(
                "L’effort « {effort} » dépasse « {max} », le plus haut permis {by}.",
                "The effort “{effort}” is above “{max}”, the highest allowed {by}.",
                by = self.by_line(MAX_EFFORT)
            )),
            _ => Ok(()),
        }
    }

    /// `effort` brought down to the ceiling, when it is above it.
    pub fn effort_within(&self, effort: &str) -> String {
        match (self.check_effort(effort), self.text(MAX_EFFORT)) {
            (Err(_), Some(max)) => max.to_owned(),
            _ => effort.to_owned(),
        }
    }

    /// The model `model` of the harness `engine`, refused when `harnesses.allowedModels` names the harness and not
    /// the model. An empty model (the harness's default) passes.
    pub fn check_model(&self, engine: &str, model: &str) -> Result<()> {
        if model.is_empty() {
            return Ok(());
        }
        let Some(allowed) = self.value(MODELS).and_then(|m| m.get(engine)) else {
            return Ok(());
        };
        if strings(allowed).iter().any(|m| m == model) {
            return Ok(());
        }
        bail!(tr!(
            "Le modèle « {model} » de {engine} n’est pas permis {by}.",
            "The model “{model}” of {engine} is not allowed {by}.",
            by = self.by_line(MODELS)
        ))
    }

    /// A plugin `id` installed from `origin` (one of `plugins.allowedOrigins`' names: `store-verified`, `store`,
    /// `git`, `folder`, `dev`), refused when the policies do not allow it.
    pub fn check_plugin(&self, id: &str, origin: &str) -> Result<()> {
        if self
            .value(PLUGINS_DENIED)
            .is_some_and(|d| strings(d).iter().any(|x| x == id))
        {
            bail!(tr!(
                "Le plugin « {id} » est interdit {by}.",
                "The plugin “{id}” is denied {by}.",
                by = self.by_line(PLUGINS_DENIED)
            ));
        }
        if let Some(by) = self.get(PLUGINS_ALLOWED).and_then(|l| l.denied_by(id)) {
            bail!(tr!(
                "Le plugin « {id} » n’est pas permis {by}.",
                "The plugin “{id}” is not allowed {by}.",
                by = by.by()
            ));
        }
        if origin == "store" && self.on(VERIFIED_ONLY) {
            bail!(self.imposed(VERIFIED_ONLY));
        }
        // A verified version of the store is a version of the store: `store` allows it too.
        let allowed = |o: &str| self.allows(PLUGIN_ORIGINS_KEY, o);
        if !(allowed(origin) || origin == "store-verified" && allowed("store")) {
            let by = self
                .get(PLUGIN_ORIGINS_KEY)
                .and_then(|l| l.denied_by(origin))
                .map(LockedBy::by)
                .unwrap_or_default();
            bail!(tr!(
                "Un plugin de cette origine ({origin}) n’est pas permis {by}.",
                "A plugin of this origin ({origin}) is not allowed {by}."
            ));
        }
        Ok(())
    }

    /// Each imposed rank's `extensions.allowed` (the managed device's, then the team's), as the extensions of the editor
    /// cross them with the device's and the projects' (`vsx::layers`).
    pub fn extension_layers(&self) -> Vec<Imposed> {
        self.get(EXTENSIONS_ALLOWED)
            .map(|l| l.layers.clone())
            .unwrap_or_default()
    }

    /// Whether the dictation may start.
    pub fn check_dictation(&self) -> Result<()> {
        if self.off(DICTATION) {
            bail!(self.imposed(DICTATION));
        }
        Ok(())
    }

    /// Whether a plugin's secret may be kept in the file readable by its user alone, where the system has no keychain
    /// (ESC-499): `secrets.fileFallback` off denies it, the file chosen is then no longer read (ESC-520, `Kept::deny`).
    pub fn check_secrets_file(&self) -> Result<()> {
        if self.off(SECRETS_FILE) {
            bail!(tr!(
                "Garder les secrets des plugins dans un fichier est interdit {by} ({SECRETS_FILE}) : seul le trousseau système les garde.",
                "Keeping the plugins’ secrets in a file is denied {by} ({SECRETS_FILE}): only the system keychain keeps them.",
                by = self.by_line(SECRETS_FILE)
            ));
        }
        Ok(())
    }

    /// Whether the remote API may serve (ESC-478): at its start and at each connection.
    pub fn check_remote_api(&self) -> Result<()> {
        if self.off(REMOTE_API) {
            bail!(tr!(
                "L’API distante est interdite {by} ({REMOTE_API}).",
                "The remote API is denied {by} ({REMOTE_API}).",
                by = self.by_line(REMOTE_API)
            ));
        }
        Ok(())
    }

    /// What Claude outside Escouade may do through its MCP server: `denied`, `readOnly` or `allowed`.
    pub fn mcp_external(&self) -> &str {
        self.text(MCP_EXTERNAL).unwrap_or("allowed")
    }

    /// What the managed device enrolls: the team server, its team's fingerprint, and whether any other server is
    /// refused. None when it sets no team server. The screen to join a team (TS-4) reads it.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn enrollment(&self) -> Option<Enrollment> {
        let server = self.text(TEAM_SERVER)?.to_owned();
        Some(Enrollment {
            server,
            fingerprint: self.text(TEAM_FINGERPRINT).map(str::to_owned),
            only_this_server: self.on(ONLY_THIS_SERVER),
        })
    }

    /// A server the app is asked to join (a team server, a server in server mode): refused when the managed device
    /// allows only its own team server, or a list that does not hold it. `fingerprint`, the team's, is checked against
    /// the one enrolled when both are known. Joining a team (TS-4) and a server in server mode go through it.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn check_server(&self, url: &str, fingerprint: Option<&str>) -> Result<()> {
        let asked = server_address(url);
        if self.on(ONLY_THIS_SERVER) {
            match self.text(TEAM_SERVER).map(server_address) {
                Some(own) if own == asked => {}
                Some(own) => bail!(tr!(
                    "Le serveur {asked} est refusé : la stratégie de ce poste ne permet que {own}.",
                    "The server {asked} is refused: this device’s policy allows {own} alone."
                )),
                None => bail!(tr!(
                    "Le serveur {asked} est refusé : la stratégie de ce poste refuse tout autre serveur d’équipe que le sien, et n’en pose aucun.",
                    "The server {asked} is refused: this device’s policy refuses any team server but its own, and sets none."
                )),
            }
        }
        if let Some(lock) = self.get(SERVERS) {
            let allowed = lock.allowed().unwrap_or_default();
            if !allowed.iter().any(|a| server_address(a) == asked) {
                let by = lock
                    .bounds
                    .iter()
                    .find(|b| !b.allowed.iter().any(|a| server_address(a) == asked))
                    .map(|b| b.by.by())
                    .unwrap_or_default();
                bail!(tr!(
                    "Le serveur {asked} n’est pas permis {by}.",
                    "The server {asked} is not allowed {by}."
                ));
            }
        }
        if let (Some(want), Some(got)) = (self.text(TEAM_FINGERPRINT), fingerprint) {
            let same = self
                .text(TEAM_SERVER)
                .is_some_and(|own| server_address(own) == asked);
            if same && !want.trim().eq_ignore_ascii_case(got.trim()) {
                bail!(tr!(
                    "L’équipe de {asked} n’a pas l’empreinte que pose la stratégie de ce poste : refusée.",
                    "The team of {asked} does not have the fingerprint this device’s policy sets: refused."
                ));
            }
        }
        Ok(())
    }

    /// A server of the server mode reached by SSH (ESC-537), `host` its address (the `HostName` of `~/.ssh/config`, or
    /// the host typed): refused before any connection when `servers.allowed` does not hold it. An entry is a host
    /// (`escouade.acme.internal`, `ssh://escouade.acme.internal`, `user@host:port` read for its host) or a pattern of a
    /// domain (`*.acme.internal`).
    ///
    /// `servers.personalSpace` to `false` (ESC-539, S16): a server must be one of `servers.allowed`; without that list,
    /// none is.
    pub fn check_ssh_server(&self, host: &str) -> Result<()> {
        let Some(lock) = self.get(SERVERS) else {
            if self.off(PERSONAL_SPACE) {
                let asked = ssh_host_of(host);
                bail!(tr!(
                    "Le serveur {asked} est refusé : la stratégie de ce poste n’ajoute que les serveurs de sa liste, et n’en pose aucune.",
                    "Server {asked} is refused: this computer’s policy only adds the servers of its list, and sets none."
                ));
            }
            return Ok(());
        };
        let asked = ssh_host_of(host);
        let allowed = lock.allowed().unwrap_or_default();
        let fits = |entry: &str| {
            let entry = ssh_host_of(entry);
            match entry.strip_prefix("*.") {
                Some(domain) => asked.ends_with(&format!(".{domain}")),
                None => !entry.is_empty() && entry == asked,
            }
        };
        if allowed.iter().any(|a| fits(a)) {
            return Ok(());
        }
        let list = if allowed.is_empty() {
            tr!("aucun", "none")
        } else {
            allowed
                .iter()
                .map(|a| ssh_host_of(a))
                .collect::<Vec<_>>()
                .join(", ")
        };
        bail!(tr!(
            "Le serveur {asked} est refusé : la stratégie de ce poste ne permet que {list}.",
            "Server {asked} is refused: this computer’s policy only allows {list}."
        ))
    }

    /// « par la stratégie de ce poste »: who imposes `key`.
    fn by_line(&self, key: &str) -> String {
        self.by(key)
            .map(LockedBy::by)
            .unwrap_or_else(|| tr!("par la stratégie de ce poste", "by this device’s policy"))
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
        if self.off(AUTO_UPDATE) && s.auto_update {
            s.auto_update = false;
            changed.push(AUTO_UPDATE);
        }
        if self.on(DENY_INSECURE_TLS) && s.insecure_tls {
            s.insecure_tls = false;
            changed.push(DENY_INSECURE_TLS);
        }
        if self.on(VERIFIED_ONLY) && !s.plugins_verified_only {
            s.plugins_verified_only = true;
            changed.push(VERIFIED_ONLY);
        }
        if self.on(FORCE_PRIVATE) && !s.private_projects {
            s.private_projects = true;
            changed.push(FORCE_PRIVATE);
        }
        if let Some(url) = self.text(PROXY_URL) {
            if s.proxy_url.trim() != url {
                s.proxy_url = url.to_owned();
                changed.push(PROXY_URL);
            }
        }
        if let Some(list) = self.value(PROXY_EXCLUSIONS).map(strings) {
            let set: Vec<&str> = s
                .no_proxy
                .split(',')
                .map(str::trim)
                .filter(|x| !x.is_empty())
                .collect();
            if set != list.iter().map(String::as_str).collect::<Vec<_>>() {
                s.no_proxy = list.join(",");
                changed.push(PROXY_EXCLUSIONS);
            }
        }
        if let Some(ca) = self.text(AUTHORITIES) {
            if s.extra_ca_file.trim() != ca {
                s.extra_ca_file = ca.to_owned();
                changed.push(AUTHORITIES);
            }
        }
        if self.off(VOICE_ONLINE)
            && (s.voice.transcription == "online" || s.voice.speech.engine == "online")
        {
            if s.voice.transcription == "online" {
                s.voice.transcription = "local".into();
            }
            if s.voice.speech.engine == "online" {
                s.voice.speech.engine = "local".into();
            }
            changed.push(VOICE_ONLINE);
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
        if self.on(FORCE_PRIVATE) && !p.private {
            p.private = true;
            changed.push(FORCE_PRIVATE);
        }
        if self.check_effort(&p.board.effort).is_err() {
            p.board.effort = self.effort_within(&p.board.effort);
            changed.push(MAX_EFFORT);
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
        if !after.private && self.on(FORCE_PRIVATE) {
            bail!(self.imposed(FORCE_PRIVATE));
        }
        if after.board.effort != before.board.effort {
            self.check_effort(&after.board.effort)?;
        }
        Ok(())
    }

    /// Why the `.escouade.yml` of the repository may not set `key` to `v` here: a mode or a harness not allowed.
    pub fn file_refusal(&self, key: &str, v: &Value) -> Option<String> {
        let s = v.as_str().unwrap_or_default();
        match key {
            "board.mode" if !s.is_empty() => self.check_mode(s).err().map(|e| e.to_string()),
            "agents.engine" if !s.is_empty() => self.check_engine(s).err().map(|e| e.to_string()),
            "board.effort" if !s.is_empty() => self.check_effort(s).err().map(|e| e.to_string()),
            _ => None,
        }
    }

    /// Whether « Exporter vers .escouade.yml » leaves `key` out: its value is bounded here by a lock, which the other
    /// devices of the repository may not have.
    pub fn bounds_file_key(&self, key: &str) -> bool {
        match key {
            "board.mode" => self.get(MODES_KEY).is_some() || self.get(ALLOW_BYPASS).is_some(),
            "agents.engine" => self.get(HARNESSES).is_some(),
            "board.effort" => self.get(MAX_EFFORT).is_some(),
            _ => false,
        }
    }

    /// Whether the mode of the agent in `meta` is no longer allowed.
    pub fn agent_mode_denied(&self, meta: &AgentMeta) -> bool {
        !meta.mode.is_empty() && self.check_mode(&meta.mode).is_err()
    }

    /// The mode an agent in `meta` goes to, when its own is no longer allowed: one its harness has (`offered`, ESC-497),
    /// the nearest allowed that allows no more. None when its own is allowed, or when none of its harness's is: it then
    /// stops (`settle_locked_mode`).
    pub fn agent_mode_after(&self, meta: &AgentMeta, offered: &[&str]) -> Option<String> {
        if !self.agent_mode_denied(meta) {
            return None;
        }
        // Every allowed mode of its harness allows more: the one that allows the least, never Bypass.
        self.nearest_mode_of(&meta.mode, offered).or_else(|| {
            offered
                .iter()
                .filter(|m| **m != BYPASS && self.check_mode(m).is_ok())
                .min_by_key(|m| crate::modes::rank(m))
                .map(|m| (*m).to_owned())
        })
    }
}

/// The name `plugins.allowedOrigins` gives the origin of a plugin. The plugins of the repository's workspace (a build of
/// development alone) and of the seed are Escouade's own, verified: `store-verified`.
pub fn origin_name(origin: &crate::plugins::Origin) -> &'static str {
    use crate::plugins::Origin;
    match origin {
        Origin::Folder { .. } => "folder",
        Origin::Git { .. } => "git",
        Origin::Dev { .. } => "dev",
        Origin::Workspace => "store-verified",
        Origin::Store { seed: true, .. } | Origin::Store { verified: true, .. } => "store-verified",
        Origin::Store { .. } => "store",
    }
}

/// What the managed device enrolls (§6.3): the team server and the team's fingerprint. The screen to join a team (TS-4)
/// fills them in; until then the core keeps them and refuses another server (`Locks::check_server`).
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Enrollment {
    pub server: String,
    pub fingerprint: Option<String>,
    pub only_this_server: bool,
}

/// The host of an address as a server of the server mode is compared (ESC-537): its scheme, its user, its port and
/// its path left out, in lower case (`ssh://escouade@Vps.Example.net:2222/` → `vps.example.net`).
pub fn ssh_host_of(address: &str) -> String {
    let a = address.trim();
    let a = a.split_once("://").map(|(_, rest)| rest).unwrap_or(a);
    let a = a.split('/').next().unwrap_or("");
    let a = a.rsplit('@').next().unwrap_or("");
    let a = if let Some(rest) = a.strip_prefix('[') {
        rest.split(']').next().unwrap_or("")
    } else if a.matches(':').count() == 1 {
        a.split(':').next().unwrap_or("")
    } else {
        a
    };
    a.trim_end_matches('.').to_lowercase()
}

/// A server's address as it is compared: trimmed, its scheme and host in lower case, without the final `/`.
#[cfg_attr(not(test), allow(dead_code))]
pub fn server_address(url: &str) -> String {
    let url = url.trim().trim_end_matches('/');
    match url.split_once("://") {
        Some((scheme, rest)) => {
            let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
            let mut out = format!("{}://{}", scheme.to_lowercase(), host.to_lowercase());
            if !path.is_empty() {
                out.push('/');
                out.push_str(path);
            }
            out
        }
        None => url.to_lowercase(),
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

// ---------- a team's keys: its projects, or the app ----------

/// Whether a key of the team holds for the team's projects alone (§6.1: « une stratégie d'équipe ne vaut que dans son
/// espace »): the modes, the harnesses, the projects' closure, a project's remote control. The other keys (the
/// network, the telemetry, the voice, the updates, the plugins, the outside MCP clients) are of the app: with no
/// spaces yet (TS-4), they hold for the app while the device is in the team.
pub fn project_scoped(key: &str) -> bool {
    ["permissions.", "harnesses.", "projects."]
        .iter()
        .any(|p| key.starts_with(p))
        || key == REMOTE
}

/// The keys of the app of every team, as one rank 5 (the first team that sets a key holds it).
fn app_wide(teams: &BTreeMap<String, TeamPolicies>) -> Option<TeamPolicies> {
    let mut out: Option<TeamPolicies> = None;
    for t in teams.values() {
        let keep = |m: &BTreeMap<String, Value>| -> BTreeMap<String, Value> {
            m.iter()
                .filter(|(k, _)| !project_scoped(k))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        };
        let o = out.get_or_insert_with(|| TeamPolicies {
            id: t.id.clone(),
            team: t.team.clone(),
            signed: t.signed.clone(),
            ..Default::default()
        });
        for (k, v) in keep(&t.enforced) {
            o.enforced.entry(k).or_insert(v);
        }
        for (k, v) in keep(&t.recommended) {
            o.recommended.entry(k).or_insert(v);
        }
    }
    out.filter(|o| !o.enforced.is_empty() || !o.recommended.is_empty())
}

// ---------- in the core ----------

impl crate::core::Core {
    /// Rank 5 of one team, as a test gives it: it replaces every team's. The locks are made again and applied at once.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn set_team_policies(self: &std::sync::Arc<Self>, team: Option<TeamPolicies>) {
        {
            let mut all = self.team_policies.write();
            all.clear();
            if let Some(t) = team {
                all.insert(t.id.clone(), t);
            }
        }
        self.apply_locks();
    }

    /// Rank 5 and rank 2 of the team `id`, from its signed file (TS-13); None when the device leaves it. Applied at once
    /// when they changed.
    pub(crate) fn set_team_policies_of(
        self: &std::sync::Arc<Self>,
        id: &str,
        team: Option<TeamPolicies>,
    ) {
        {
            let mut all = self.team_policies.write();
            if all.get(id) == team.as_ref() {
                return;
            }
            match team {
                Some(t) => all.insert(id.to_owned(), t),
                None => all.remove(id),
            };
        }
        self.apply_locks();
    }

    /// The locks of a project: its team's when it is of one (ranks 5 and 6), the app's otherwise.
    pub(crate) fn locks_for(&self, project_id: &str) -> Locks {
        let team = self
            .projects
            .read()
            .iter()
            .find(|p| p.id == project_id)
            .and_then(|p| p.team.clone());
        team.and_then(|t| self.team_locks.read().get(&t).cloned())
            .unwrap_or_else(|| self.locks.read().clone())
    }

    /// Rank 6, as read (`Managed::read` at the start, `watch_policies` at each change; a test gives its own). The locks
    /// are made again and applied.
    pub(crate) fn set_managed(self: &std::sync::Arc<Self>, managed: Managed) {
        *self.managed.write() = managed;
        self.apply_locks();
    }

    /// Rank 6 watched while the app runs (TS-12): a policy changed is applied at once, as at the start. The agents at
    /// work follow at the end of their turn (`settle_locked_mode`).
    pub(crate) fn watch_policies(self: &std::sync::Arc<Self>) {
        let weak = std::sync::Arc::downgrade(self);
        let watch = Managed::watch(move |managed| {
            let Some(core) = weak.upgrade() else { return };
            if *core.managed.read() == managed {
                return;
            }
            log::info!(
                "policies: changed while the app runs ({} set)",
                managed.values.len()
            );
            core.set_managed(managed);
        });
        *self.policy_watch.lock() = Some(watch);
    }

    /// The locks made again from ranks 5 and 6, and what they lock brought within them: the settings, the projects, the
    /// agents (each at the end of its turn). The window is told.
    pub(crate) fn apply_locks(self: &std::sync::Arc<Self>) {
        let teams = self.team_policies.read().clone();
        let managed = self.managed.read().clone();
        let locks = Locks::compute(KEYS, app_wide(&teams).as_ref(), &managed);
        for w in &locks.warnings {
            log::warn!("policies: {w}");
        }
        let by_team: BTreeMap<String, Locks> = teams
            .iter()
            .map(|(id, t)| (id.clone(), Locks::compute(KEYS, Some(t), &managed)))
            .collect();
        // The file of the plugins' secrets, of the app (never a project's): denied or allowed again at once (ESC-520).
        self.kept
            .deny(locks.check_secrets_file().err().map(|e| e.to_string()));
        *self.locks.write() = locks;
        *self.team_locks.write() = by_team;
        let changed = {
            let locks = self.locks.read();
            locks.clamp_settings(&mut self.settings.write())
        };
        let settings = (!changed.is_empty()).then(|| {
            log::info!("policies: settings brought within {changed:?}");
            let telemetry = self.settings.read().telemetry;
            self.telemetry.set_consent(telemetry);
            self.write_settings_file();
            Box::new(self.settings.read().clone())
        });
        self.settle_locks();
        self.hub.emit(crate::model::UiEvent::Locks {
            locks: self.locks.read().clone(),
            teams: self.team_locks.read().clone(),
            settings,
        });
    }

    /// The projects brought within the locks, and each agent whose mode is no longer allowed moved to the nearest one
    /// that is: at once when it is not at work, at the end of its turn otherwise (`settle_locked_mode`).
    pub(crate) fn settle_locks(self: &std::sync::Arc<Self>) {
        // The agents first: a policy taken off locks nothing, but « Autoriser le mode Bypass » may still refuse one.
        self.settle_agents();
        if self.locks.read().locks.is_empty()
            && self.team_locks.read().values().all(|l| l.locks.is_empty())
        {
            return;
        }
        // Taken apart, never held under the projects' lock.
        let base = self.locks.read().clone();
        let teams = self.team_locks.read().clone();
        let of = |team: &Option<String>| -> &Locks {
            team.as_ref().and_then(|t| teams.get(t)).unwrap_or(&base)
        };
        let projects: Vec<Project> = {
            let mut projects = self.projects.write();
            projects
                .iter_mut()
                .filter_map(|p| {
                    let locks = of(&p.team);
                    (!locks.clamp_project(p).is_empty()).then(|| p.clone())
                })
                .collect()
        };
        for project in projects {
            log::info!("policies: project {} brought within them", project.id);
            self.hub.emit(crate::model::UiEvent::Project { project });
            self.request_save();
        }
    }

    /// Each agent at rest that may not run on as it is — its mode no longer allowed, or what runs of it (its process,
    /// its interface) in a harness or a mode the policies or « Autoriser le mode Bypass » no longer allow — settled at
    /// once (`settle_locked_mode`); one at work, at the end of its turn (ESC-497). Called when the policies or that
    /// setting change.
    pub(crate) fn settle_agents(self: &std::sync::Arc<Self>) {
        // Taken apart, never held under an agent's lock.
        let base = self.locks.read().clone();
        let teams = self.team_locks.read().clone();
        let bypass = self.bypass_allowed();
        let lang = crate::i18n::ui();
        let of = |team: &Option<String>| -> &Locks {
            team.as_ref().and_then(|t| teams.get(t)).unwrap_or(&base)
        };
        let team_of: BTreeMap<String, Option<String>> = self
            .projects
            .read()
            .iter()
            .map(|p| (p.id.clone(), p.team.clone()))
            .collect();
        let idle: Vec<String> = self
            .agents
            .read()
            .iter()
            .filter(|(_, h)| {
                let rt = h.lock();
                let locks = of(team_of.get(&rt.meta.project_id).unwrap_or(&None));
                !rt.meta.status.is_active()
                    && (locks.agent_mode_denied(&rt.meta)
                        || (rt.proc.is_some() || rt.tui.is_some())
                            && crate::modes::refusal_of(&rt.meta, locks, bypass, lang).is_some())
            })
            .map(|(id, _)| id.clone())
            .collect();
        for id in idle {
            let c = self.clone();
            tauri::async_runtime::spawn(async move { c.settle_locked_mode(&id).await });
        }
    }

    /// An agent in a mode the locks no longer allow goes to the nearest one allowed that its harness has (ESC-497), and
    /// says it in its conversation. Called once its turn is over (`turn_ended`), and at once for an agent not at work.
    pub(crate) async fn settle_locked_mode(self: &std::sync::Arc<Self>, id: &str) {
        let Ok(h) = self.agent(id) else { return };
        let locks = self.locks_for(&h.lock().meta.project_id.clone());
        let (from, to) = {
            let rt = h.lock();
            let offered = crate::modes::offered(crate::engine::of(&rt.meta));
            (
                rt.meta.mode.clone(),
                locks.agent_mode_after(&rt.meta, &offered),
            )
        };
        // A harness no longer allowed, or none of whose modes is: its process stops between two turns, and the agent
        // says why (§6.2). Its next message is refused at the start of a process (`check_engine`, `settle_mode`).
        let denied = {
            let rt = h.lock();
            locks
                .check_engine(rt.meta.engine.as_str())
                .err()
                .map(|e| e.to_string())
                .or_else(|| {
                    (to.is_none() && locks.agent_mode_denied(&rt.meta)).then(|| {
                        let why = locks
                            .denial(MODES_KEY, &from)
                            .unwrap_or_else(|| locks.imposed(ALLOW_BYPASS));
                        let name = crate::engine::of(&rt.meta).name();
                        tr!(
                            "{why} {name} n’a aucun autre mode permis.",
                            "{why} {name} has no other mode allowed."
                        )
                    })
                })
        };
        let ends = |why: &str| {
            tr!(
                "{why} L’agent s’arrête à la fin de son tour.",
                "{why} The agent stops at the end of its turn."
            )
        };
        if let Some(why) = denied {
            // Its process and its interface (ESC-497).
            self.stop_out_of_rule(id, &ends(&why));
            return;
        }
        if let Some(to) = to {
            self.move_locked_mode(id, &locks, &from, &to).await;
        }
        // What still refuses it as it runs: « Autoriser le mode Bypass » turned off, or a mode its harness runs it in
        // that the move did not settle (ESC-497). Its mode stays; what runs of it stops, said.
        if let Some(why) = self.run_refusal(id) {
            self.stop_out_of_rule(id, &ends(&why));
        }
    }

    /// The agent `id`, in `from` that `locks` no longer allow, moved to `to`, said.
    async fn move_locked_mode(
        self: &std::sync::Arc<Self>,
        id: &str,
        locks: &Locks,
        from: &str,
        to: &str,
    ) {
        if let Err(e) = self
            .set_agent_options(id, None, None, Some(to.to_string()))
            .await
        {
            log::warn!("policies: agent {id} not moved to {to}: {e:#}");
            return;
        }
        let why = locks
            .denial(MODES_KEY, from)
            .unwrap_or_else(|| locks.imposed(ALLOW_BYPASS));
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

#[cfg(test)]
#[path = "locks_policies_tests.rs"]
mod policies_tests;
