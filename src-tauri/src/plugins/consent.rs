//! The agreement to a harness a plugin of a third party describes (audit SEC-01, ESC-589). Such a harness is a program
//! of this machine, found on the `PATH` by the names its manifest gives, launched out of the plugin's sandbox with its
//! arguments and its variables: what used to guard it was a list of shells and interpreters refused (`manifest::
//! interpreter`), which a launcher such as `npx` passes. Its first launch now asks the user, its whole command said
//! (`LaunchLine`: the program found, its arguments, those it resumes a session with, those that read its version, the
//! variables it adds and those it takes off). The agreement is kept by the fingerprint of that line, in the plugin's
//! choices (`Choices::harnesses_approved`): a line an update of the plugin changes asks again. Refused, its agent does
//! not start. « Harness des plugins tiers : Refuser » (`ThirdPartyHarnesses::Refuse`, what a managed device's policy
//! imposes) runs none. The harnesses of Escouade's own hand (`dev.escouade.*`, signed) ask nothing.
//!
//! The core keeps what was agreed here (`set`), and every place that runs such a program asks it first (`check`):
//! the start of an agent, the question of one shot, « Se connecter… »; the launch the plugin then describes is held to
//! the line agreed (`LaunchLine::covers`): the agreement reads the command again when it is about to run.

use crate::model::EngineId;
use parking_lot::RwLock;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};

/// A variable the line adds, and its value (never a secret: a manifest is read by all).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EnvLine {
    pub name: String,
    pub value: String,
}

/// The whole command a harness of a third party runs, as the user agrees to it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchLine {
    pub engine: EngineId,
    /// The harness's name (« Goose »).
    pub harness: String,
    /// The id of the plugin whose manifest describes it.
    pub plugin: String,
    /// The program found, as it runs (its path).
    pub program: String,
    /// Its arguments at each start.
    pub args: Vec<String>,
    /// Those added to resume a session, `{session}` standing for its id.
    pub resume_args: Vec<String>,
    /// Those that make it print its version.
    pub version_args: Vec<String>,
    /// The variables it adds.
    pub env: Vec<EnvLine>,
    /// The variables of the app's environment it takes off.
    pub remove_env: Vec<String>,
    /// The variables Escouade sets itself, on its manifest's word (the account's folder, its key): their names.
    pub escouade_env: Vec<String>,
    /// The SHA-256 of all the above but the harness's name: what an agreement keeps.
    pub fingerprint: String,
}

impl LaunchLine {
    /// The line of `program`, its fingerprint made.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        engine: EngineId,
        harness: &str,
        plugin: &str,
        program: String,
        args: Vec<String>,
        resume_args: Vec<String>,
        version_args: Vec<String>,
        env: &BTreeMap<String, String>,
        remove_env: Vec<String>,
        escouade_env: Vec<String>,
    ) -> Self {
        let env: Vec<EnvLine> = env
            .iter()
            .map(|(k, v)| EnvLine {
                name: k.clone(),
                value: v.clone(),
            })
            .collect();
        let mut line = Self {
            engine,
            harness: harness.to_string(),
            plugin: plugin.to_string(),
            program,
            args,
            resume_args,
            version_args,
            env,
            remove_env,
            escouade_env,
            fingerprint: String::new(),
        };
        line.fingerprint = line.digest();
        line
    }

    fn digest(&self) -> String {
        let mut h = Sha256::new();
        let mut part = |label: &str, items: &[&str]| {
            h.update(label.as_bytes());
            h.update([0]);
            h.update((items.len() as u64).to_le_bytes());
            for i in items {
                h.update((i.len() as u64).to_le_bytes());
                h.update(i.as_bytes());
            }
        };
        part("engine", &[self.engine.as_str()]);
        part("plugin", &[&self.plugin]);
        part("program", &[&self.program]);
        part("args", &strs(&self.args));
        part("resume", &strs(&self.resume_args));
        part("version", &strs(&self.version_args));
        let env: Vec<String> = self
            .env
            .iter()
            .map(|v| format!("{}={}", v.name, v.value))
            .collect();
        part("env", &strs(&env));
        part("remove", &strs(&self.remove_env));
        part("escouade", &strs(&self.escouade_env));
        h.finalize().iter().map(|b| format!("{b:02x}")).collect()
    }

    /// Whether the launch a plugin describes is this line: its arguments those of a start, or of a start that resumes a
    /// session (`{session}` the id given), its variables those it adds, those it takes off the same.
    pub fn covers(&self, args: &[String], env: &[(String, String)], remove: &[String]) -> bool {
        let starts = args.len() >= self.args.len() && args[..self.args.len()] == self.args[..];
        let rest = &args[self.args.len().min(args.len())..];
        let resumes = rest.is_empty() || resumed_by(&self.resume_args, rest);
        let mut said: Vec<(String, String)> = self
            .env
            .iter()
            .map(|v| (v.name.clone(), v.value.clone()))
            .collect();
        let mut env = env.to_vec();
        said.sort();
        env.sort();
        let mut removed = remove.to_vec();
        let mut declared = self.remove_env.clone();
        removed.sort();
        declared.sort();
        starts && resumes && said == env && removed == declared
    }
}

fn strs(v: &[String]) -> Vec<&str> {
    v.iter().map(String::as_str).collect()
}

/// Whether `rest` is `template` with `{session}` in place of one session's id (the same everywhere).
fn resumed_by(template: &[String], rest: &[String]) -> bool {
    if template.len() != rest.len() || template.is_empty() {
        return false;
    }
    let mut session: Option<&str> = None;
    for (t, r) in template.iter().zip(rest) {
        match t.split_once("{session}") {
            None if t == r => {}
            None => return false,
            Some((before, after)) => {
                let Some(id) = r
                    .strip_prefix(before)
                    .and_then(|x| x.strip_suffix(after))
                    .filter(|id| !id.is_empty())
                else {
                    return false;
                };
                if session.is_some_and(|s| s != id) {
                    return false;
                }
                session = Some(id);
            }
        }
    }
    true
}

/// What the core agreed to, as the places that run a program read it.
#[derive(Debug, Default)]
struct Agreed {
    /// « Harness des plugins tiers : Refuser ».
    refuse_all: bool,
    /// The fingerprints of the lines agreed to, by harness.
    approved: HashMap<EngineId, Vec<String>>,
}

static AGREED: RwLock<Option<Agreed>> = RwLock::new(None);

/// What the core holds now: « Refuser » or not, and the lines agreed to by harness (`Core::sync_consents`).
pub fn set(refuse_all: bool, approved: HashMap<EngineId, Vec<String>>) {
    *AGREED.write() = Some(Agreed {
        refuse_all,
        approved,
    });
}

/// Why a harness of a third party does not run its line now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Held {
    /// « Harness des plugins tiers : Refuser ».
    Refused,
    /// Its line is not agreed to: the user is asked.
    Asks,
}

/// Whether `line` may run: agreed to, and not refused by the setting.
pub fn check(line: &LaunchLine) -> Result<(), Held> {
    let agreed = AGREED.read();
    let Some(a) = agreed.as_ref() else {
        return Err(Held::Asks);
    };
    if a.refuse_all {
        return Err(Held::Refused);
    }
    if a.approved
        .get(&line.engine)
        .is_some_and(|f| f.contains(&line.fingerprint))
    {
        Ok(())
    } else {
        Err(Held::Asks)
    }
}

/// The refusal of `line` held, in the interface's language: what the conversation says, and what the window reads to ask.
pub fn refusal(line: &LaunchLine, held: &Held) -> String {
    let name = &line.harness;
    let plugin = &line.plugin;
    match held {
        Held::Refused => tr!(
            "{name} n’est pas lancé : il vient du plugin tiers {plugin}, et « Harness des plugins tiers » est sur « Refuser » (Réglages › Plugins).",
            "{name} is not launched: it comes from the third-party plugin {plugin}, and “Third-party harnesses” is set to “Refuse” (Settings › Plugins).",
        ),
        Held::Asks => tr!(
            "{name} attend ton accord pour lancer sa commande : il vient du plugin tiers {plugin} et lance un programme de ce poste hors du bac à sable. « Voir la commande… » la redit entière.",
            "{name} awaits your agreement to launch its command: it comes from the third-party plugin {plugin} and runs a program of this machine out of the sandbox. “See the command…” says it whole.",
        ),
    }
}

/// Nothing known yet, as before the core's first `set`.
#[cfg(test)]
pub(crate) fn reset() {
    *AGREED.write() = None;
}

/// The tests' agreement to `line`, beside what was agreed already (the tests of a harness of a third party that are not
/// about its agreement).
#[cfg(test)]
pub(crate) fn agree(line: &LaunchLine) {
    let mut agreed = AGREED.write();
    let a = agreed.get_or_insert_with(Agreed::default);
    a.approved
        .entry(line.engine)
        .or_default()
        .push(line.fingerprint.clone());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(args: &[&str], env: &[(&str, &str)]) -> LaunchLine {
        LaunchLine::new(
            EngineId::new("plugin:dev.example.outil:outil"),
            "Outil",
            "dev.example.outil",
            "C:/outils/npx.cmd".into(),
            args.iter().map(|a| a.to_string()).collect(),
            vec!["--resume".into(), "{session}".into()],
            vec!["--version".into()],
            &env.iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            vec!["OUTIL_TOKEN".into()],
            vec!["OUTIL_HOME".into()],
        )
    }

    /// The fingerprint is the line's: the same line the same, a changed argument, value or program another.
    #[test]
    fn the_fingerprint_of_a_line_changes_with_any_of_its_words() {
        let a = line(&["un-paquet"], &[("COULEUR", "bleu")]);
        assert_eq!(
            a.fingerprint,
            line(&["un-paquet"], &[("COULEUR", "bleu")]).fingerprint
        );
        assert_eq!(a.fingerprint.len(), 64);
        for other in [
            line(&["un-paquet", "--autre"], &[("COULEUR", "bleu")]),
            line(&["un-autre-paquet"], &[("COULEUR", "bleu")]),
            line(&["un-paquet"], &[("COULEUR", "rouge")]),
            line(&["un-paquet"], &[]),
        ] {
            assert_ne!(a.fingerprint, other.fingerprint, "{other:?}");
        }
        let mut moved = a.clone();
        moved.program = "D:/ailleurs/npx.cmd".into();
        assert_ne!(a.fingerprint, moved.digest());
    }

    /// The launch a plugin describes is held to the line: its arguments, those of a resume with one session's id, its
    /// variables; anything more is not the line.
    #[test]
    fn a_launch_is_the_line_agreed_or_nothing_runs() {
        let l = line(&["un-paquet"], &[("COULEUR", "bleu")]);
        let env = vec![("COULEUR".to_string(), "bleu".to_string())];
        let removed = vec!["OUTIL_TOKEN".to_string()];
        let args = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(l.covers(&args(&["un-paquet"]), &env, &removed));
        assert!(l.covers(&args(&["un-paquet", "--resume", "abc-12"]), &env, &removed));
        assert!(!l.covers(&args(&["un-paquet", "--resume", ""]), &env, &removed));
        assert!(!l.covers(&args(&["un-paquet", "--autre"]), &env, &removed));
        assert!(!l.covers(&args(&["autre-paquet"]), &env, &removed));
        assert!(!l.covers(&args(&[]), &env, &removed));
        assert!(!l.covers(&args(&["un-paquet"]), &[], &removed));
        let more = vec![
            ("COULEUR".to_string(), "bleu".to_string()),
            ("NODE_OPTIONS".to_string(), "--require x".to_string()),
        ];
        assert!(!l.covers(&args(&["un-paquet"]), &more, &removed));
        assert!(!l.covers(&args(&["un-paquet"]), &env, &[]));
    }

    /// Agreed by its fingerprint, for its harness alone; « Refuser » holds every line; nothing known yet asks.
    #[test]
    fn a_line_runs_once_agreed_and_never_with_refuse() {
        let l = line(&["un-paquet"], &[]);
        reset();
        assert_eq!(check(&l), Err(Held::Asks));
        set(false, HashMap::new());
        assert_eq!(check(&l), Err(Held::Asks));
        set(
            false,
            HashMap::from([(l.engine, vec![l.fingerprint.clone()])]),
        );
        assert_eq!(check(&l), Ok(()));
        let changed = line(&["un-paquet", "--autre"], &[]);
        assert_eq!(check(&changed), Err(Held::Asks));
        set(
            true,
            HashMap::from([(l.engine, vec![l.fingerprint.clone()])]),
        );
        assert_eq!(check(&l), Err(Held::Refused));
        let asks = refusal(&l, &Held::Asks);
        assert!(
            asks.contains("attend ton accord") && asks.contains("dev.example.outil"),
            "{asks}"
        );
        assert!(refusal(&l, &Held::Refused).contains("« Refuser »"));
    }
}
