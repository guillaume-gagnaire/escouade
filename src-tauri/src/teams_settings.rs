//! The team's signed `.escouade.yml` on this device (TS-13): read at each reading of the team, applied by the core,
//! shown to the member (the band, the history, the consents) and written by an administrator (propose, cosign).

use escouade_team::settings::{self, Part, TeamFile, View};
use serde::Serialize;

use super::{say, Teams};
use crate::team_settings::{parse, to_yaml, CORE_KEYS};

/// What the window shows of a team's settings.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamSettingsView {
    pub team: String,
    pub name: String,
    /// This device is an administrator's: it proposes and cosigns.
    pub admin: bool,
    /// This device's member id.
    pub me: String,
    pub view: View,
    /// The version in force as YAML: what the next one starts from.
    pub yaml: String,
    /// The last version the member saw (« Voir » on the band).
    pub seen: Option<u64>,
    pub signer_name: String,
}

/// A version written in the window, read before it is signed.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsDraft {
    pub file: Option<TeamFile>,
    pub warnings: Vec<String>,
    /// It cannot be read, or it holds what never enters: it is not signed.
    pub error: Option<String>,
    /// The signatures it will need: 2 under the quorum, for a sensitive key or the signers.
    pub need: usize,
}

/// What the window says of a line of what never enters.
pub fn never_line(hit: &settings::NeverHit) -> String {
    use settings::Never;
    let at = format!("{}.{}", hit.part.name(), hit.key);
    match hit.rule {
        Never::Bypass => tr!(
            "{at} : le mode bypassPermissions n’entre jamais dans le fichier d’équipe, même signé.",
            "{at}: the bypassPermissions mode never enters the team’s file, even signed."
        ),
        Never::CommandWithoutConsent => tr!(
            "{at} : une commande ne tourne jamais sans l’accord de chaque poste ; elle va dans « proposed », jamais marquée acceptée.",
            "{at}: a command never runs without each device’s consent; it goes in “proposed”, never marked accepted."
        ),
        Never::PluginWithoutGesture => tr!(
            "{at} : un plugin ne s’installe jamais sans le geste du membre ; il va dans « proposed ».",
            "{at}: a plugin is never installed without the member’s gesture; it goes in “proposed”."
        ),
        Never::FirstParty => tr!(
            "{at} : l’équipe ne signe jamais un plugin dev.escouade.*.",
            "{at}: the team never signs a dev.escouade.* plugin."
        ),
        Never::MadePublic => tr!(
            "{at} : le fichier d’équipe ne rend jamais un projet public.",
            "{at}: the team’s file never makes a project public."
        ),
    }
}

fn part_of(name: &str) -> Result<Part, String> {
    match name {
        "defaults" => Ok(Part::Defaults),
        "enforced" => Ok(Part::Enforced),
        "proposed" => Ok(Part::Proposed),
        _ => Err(format!("{name}: not a part of the file")),
    }
}

fn applied_file(view: &View) -> Option<&TeamFile> {
    let n = view.applied?;
    view.versions
        .iter()
        .find(|v| v.number == n)
        .map(|v| &v.file)
}

impl Teams {
    /// Where the versions in force go (the core): each team opened is told at once, from what it read last.
    pub fn set_settings_sink(&self, sink: super::SettingsSink) {
        *self.on_settings.lock() = Some(sink);
    }

    /// The settings of each team opened, from what this device read last (offline): applied at the start.
    pub async fn start_settings(&self) {
        let open: Vec<(String, super::Shared)> = self
            .open
            .lock()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        for (id, team) in open {
            let view = {
                let t = team.lock().await;
                settings::view(&t, &CORE_KEYS)
            };
            match view {
                Ok(v) => self.put_settings(&id, v, true),
                Err(e) => log::warn!("teams: the settings of a team not read: {}", say(&e)),
            }
        }
    }

    /// Reads the team's settings after its stream: what is new, applied; an administrator's device cosigns the
    /// versions whose signer left. An error is logged: the team's reading goes on.
    pub(super) async fn read_settings(&self, id: &str, t: &mut escouade_team::Team) {
        let mut view = match settings::sync(t, &CORE_KEYS).await {
            Ok(v) => v,
            Err(e) => {
                log::warn!("teams: settings not read: {}", say(&e));
                return;
            }
        };
        if t.is_admin().unwrap_or(false) {
            match settings::serve(t, &CORE_KEYS).await {
                Ok(n) if n > 0 => {
                    if let Ok(v) = settings::view(t, &CORE_KEYS) {
                        view = v;
                    }
                }
                Ok(_) => {}
                Err(e) => log::warn!("teams: settings not cosigned: {}", say(&e)),
            }
        }
        self.put_settings(id, view, false);
    }

    fn put_settings(&self, id: &str, view: View, always: bool) {
        let changed = {
            let mut views = self.settings_views.lock();
            let changed = views.get(id) != Some(&view);
            views.insert(id.to_owned(), view.clone());
            changed
        };
        if !(changed || always) {
            return;
        }
        let name = self.known(id).map(|k| k.name).unwrap_or_default();
        let sink = self.on_settings.lock().clone();
        if let Some(sink) = sink {
            sink(id, &name, Some(&view));
        }
        if changed {
            (self.changed)();
        }
    }

    pub(super) fn forget_settings(&self, id: &str) {
        self.settings_views.lock().remove(id);
        let sink = self.on_settings.lock().clone();
        if let Some(sink) = sink {
            sink(id, "", None);
        }
    }

    async fn settings_of(&self, id: &str) -> Result<TeamSettingsView, String> {
        let known = self.known(id)?;
        let team = self.team(id)?;
        let t = team.lock().await;
        let view = match self.settings_views.lock().get(id).cloned() {
            Some(v) => v,
            None => settings::view(&t, &CORE_KEYS).map_err(|e| say(&e))?,
        };
        Ok(TeamSettingsView {
            team: id.to_owned(),
            name: known.name.clone(),
            admin: t.is_admin().unwrap_or(false),
            me: t.identity().member.to_string(),
            yaml: applied_file(&view)
                .map(to_yaml)
                .unwrap_or_else(|| "version: 1\n".into()),
            view,
            seen: known.settings_seen,
            signer_name: known.signer_name,
        })
    }

    /// The team's settings, as this device read them.
    pub async fn settings(&self, id: &str) -> Result<TeamSettingsView, String> {
        self.settings_of(id).await
    }

    /// A version written in the window, read: its file, what will not apply, what refuses it, the signatures it needs.
    pub fn settings_check(&self, id: &str, yaml: &str) -> SettingsDraft {
        let view = self
            .settings_views
            .lock()
            .get(id)
            .cloned()
            .unwrap_or_default();
        match parse(yaml) {
            Err(e) => SettingsDraft {
                file: None,
                warnings: Vec::new(),
                error: Some(e),
                need: 1,
            },
            Ok((file, warnings)) => {
                let error = settings::never(&file, &CORE_KEYS).map(|h| never_line(&h));
                // The quorum of the version in force holds for the next one.
                let need = if view.quorum
                    && settings::needs_quorum(applied_file(&view), &file, &CORE_KEYS)
                {
                    2
                } else {
                    1
                };
                SettingsDraft {
                    file: Some(file),
                    warnings,
                    error,
                    need,
                }
            }
        }
    }

    /// An administrator signs a version written in the window, chained to the one in force.
    pub async fn settings_propose(
        &self,
        id: &str,
        yaml: &str,
        note: &str,
        by: &str,
    ) -> Result<TeamSettingsView, String> {
        self.unblocked(id)?;
        let (file, _) = parse(yaml)?;
        if let Some(hit) = settings::never(&file, &CORE_KEYS) {
            return Err(never_line(&hit));
        }
        let by = by.trim().chars().take(80).collect::<String>();
        if by.is_empty() {
            return Err(tr!(
                "Dis sous quel nom tu signes : les membres le verront.",
                "Say what name you sign with: the members will see it."
            ));
        }
        self.update(id, |k| k.signer_name = by.clone());
        let team = self.team(id)?;
        let view = {
            let mut t = team.lock().await;
            settings::propose(&mut t, &CORE_KEYS, file, &by, note, super::now())
                .await
                .map_err(|e| say(&e))?
        };
        self.put_settings(id, view, false);
        self.settings_of(id).await
    }

    /// An administrator signs another's version: the second signature of the quorum.
    pub async fn settings_cosign(
        &self,
        id: &str,
        digest: &str,
    ) -> Result<TeamSettingsView, String> {
        self.unblocked(id)?;
        let team = self.team(id)?;
        let view = {
            let mut t = team.lock().await;
            settings::cosign(&mut t, &CORE_KEYS, digest)
                .await
                .map_err(|e| say(&e))?
        };
        self.put_settings(id, view, false);
        self.settings_of(id).await
    }

    /// This device accepts a sensitive key, or a command proposed, at the fingerprint the member read.
    pub async fn settings_consent(
        &self,
        id: &str,
        part: &str,
        key: &str,
        fingerprint: &str,
    ) -> Result<TeamSettingsView, String> {
        let part = part_of(part)?;
        let team = self.team(id)?;
        let view = {
            let t = team.lock().await;
            settings::consent(&t, &CORE_KEYS, part, key, fingerprint).map_err(|e| match e {
                escouade_team::Error::Refused(_) => tr!(
                    "Ce réglage a changé depuis : relis-le.",
                    "This setting changed since: read it again."
                ),
                e => say(&e),
            })?
        };
        self.put_settings(id, view, false);
        self.settings_of(id).await
    }

    /// This device takes its consent back.
    pub async fn settings_withdraw(
        &self,
        id: &str,
        part: &str,
        key: &str,
    ) -> Result<TeamSettingsView, String> {
        let part = part_of(part)?;
        let team = self.team(id)?;
        let view = {
            let t = team.lock().await;
            settings::withdraw(&t, &CORE_KEYS, part, key).map_err(|e| say(&e))?
        };
        self.put_settings(id, view, false);
        self.settings_of(id).await
    }

    /// « Voir » on the band: the member saw the version `number`.
    pub fn settings_seen(&self, id: &str, number: u64) {
        self.update(id, |k| k.settings_seen = Some(number));
        (self.changed)();
    }

    /// « Réessayer » after an alarm of the settings.
    pub async fn settings_retry(&self, id: &str) -> Result<TeamSettingsView, String> {
        let team = self.team(id)?;
        let view = {
            let mut t = team.lock().await;
            settings::retry(&mut t, &CORE_KEYS)
                .await
                .map_err(|e| say(&e))?
        };
        self.put_settings(id, view, false);
        self.settings_of(id).await
    }
}
