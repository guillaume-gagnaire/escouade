//! The team's private store of plugins on this device (TS-14, `docs/TEAM-SERVER.md` §7): what the window and the
//! commands do with it. The team's client (`escouade_team::plugins`, through `Teams`) keeps the stream; here:
//!
//! - **the team's key** ([`Core::team_plugin_key_create`]): a minisign pair and its backup, drawn on this
//!   administrator's machine, never on the server. The key of every day is kept sealed by its password in
//!   `<data>/team-signing/<team>/` (out of the team's folder: forgetting the team does not lose it); the backup key,
//!   sealed by the same password, goes to the folder the administrator chooses, to keep elsewhere. Both public keys and
//!   the namespace are declared to the team. A namespace that holds a plugin installed here from another origin is
//!   refused, as `dev.escouade.*`.
//! - **a proposal** ([`Core::team_plugin_submit`]): the folder packed as `escouade-plugin` packs it, the public store's
//!   checks played (`escouade_plugin_cli::team`, the compilation twice), and only when they pass, the archive and the
//!   module encrypted on the server with the digest found.
//! - **the signature** ([`Core::team_plugin_sign`]): the administrator's machine reads the archive back from the server,
//!   plays the same checks again, and signs only when its digest is the proposal's.
//! - **the revocation list** ([`Core::team_plugin_revoke`]): the team's list, its serial one past the last, signed by
//!   the team's key; every device applies it to that team's plugins at each reading ([`Core::team_plugins_after_read`]),
//!   as the store's: a version withdrawn is disabled and its process stopped. The kill switch of TS-15 reads the same
//!   list (`Plugins::apply_team`).
//! - **installing stays the member's gesture** ([`Core::team_plugin_inspect`]): the version checked again, then set
//!   aside for the window of its permissions; `plugin_install` installs it, under the policies of the team and the
//!   device (`plugins.allowedOrigins`: `team-store`, `plugins.allowed`, `plugins.denied`).
//!
//! An agent never publishes nor signs: no tool of the MCP server reaches these, and the commands are the window's.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use escouade_plugin_cli::team as checks;
use escouade_team::plugins::{self, Checked, Store, Submission};
use escouade_team::signing::{self, SigningKey};
use serde::Serialize;
use serde_json::Value;

use crate::core::Core;
use crate::teams::{StoreOf, Teams};

/// The text of a manifest's name or description, in the window's language.
fn text_in(v: &Value, lang: crate::i18n::Lang) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Object(m) => {
            let own = match lang {
                crate::i18n::Lang::Fr => "fr",
                crate::i18n::Lang::En => "en",
            };
            [own, "en", "fr"]
                .iter()
                .find_map(|k| m.get(*k).and_then(Value::as_str))
                .unwrap_or_default()
                .to_owned()
        }
        _ => String::new(),
    }
}

/// The team's key, as the window shows it.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KeyInfo {
    pub namespace: String,
    /// The ids of the key of every day and of the backup key.
    pub ids: Vec<String>,
    /// The ids the team's revocation list revoked.
    pub revoked: Vec<String>,
    pub by: String,
    pub at: u64,
    /// This device keeps the sealed key: it can sign.
    pub here: bool,
}

/// A plugin the team published, as the window shows it.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TeamPluginRow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub by: String,
    pub signed_at: u64,
    pub digest: String,
    /// The version installed here from this team, if any.
    pub installed: Option<String>,
    /// Installed here from another origin: never replaced by the team's.
    pub elsewhere: bool,
    /// Why the team's list withdraws this version, if it does.
    pub withdrawn: Option<String>,
}

/// A proposal waiting for an administrator.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProposalRow {
    pub id: String,
    pub plugin: String,
    pub name: String,
    pub version: String,
    pub tag: String,
    pub by: String,
    pub mine: bool,
    pub at: u64,
    pub digest: String,
    pub checks: Vec<Checked>,
}

/// The team's store, as the window shows it (Réglages › Équipe, the page Plugins).
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TeamPluginsView {
    pub team: String,
    pub name: String,
    pub admin: bool,
    pub key: Option<KeyInfo>,
    /// A plugin installed here from another origin, in the team's namespace: the team's store is refused here.
    pub conflict: Option<String>,
    pub plugins: Vec<TeamPluginRow>,
    pub proposed: Vec<ProposalRow>,
    /// The serial of the team's revocation list.
    pub serial: Option<u64>,
    /// Entries of the stream refused at the reading.
    pub refused: usize,
    pub alarm: bool,
}

/// The checks of a proposal, and the proposal made when they passed.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Submitted {
    pub report: checks::Report,
    pub proposal: Option<String>,
    pub view: Option<TeamPluginsView>,
}

/// The version of Escouade a plugin is judged for: this one.
fn app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

fn compiler() -> checks::Toolchains {
    checks::Toolchains {
        data: escouade_plugin_cli::paths::tools_data_dir(),
    }
}

impl Core {
    fn team_link_or(&self) -> Result<Arc<Teams>, String> {
        self.teams_link().ok_or_else(|| {
            tr!(
                "Les équipes ne sont pas encore prêtes.",
                "The teams are not ready yet."
            )
        })
    }

    /// Where this device keeps the sealed key of every day of the team `team`.
    fn team_key_dir(&self, team: &str) -> PathBuf {
        self.data.root().join("team-signing").join(team)
    }

    fn team_key_file(&self, team: &str) -> PathBuf {
        self.team_key_dir(team).join("team.key")
    }

    fn plugins_view_of(&self, of: &StoreOf, team: &str) -> TeamPluginsView {
        let lang = crate::i18n::ui();
        let store = &of.store;
        let installed = self.plugins.list(lang);
        let key = store.key.as_ref().map(|k| {
            let trusted = store.trusted_keys();
            KeyInfo {
                namespace: k.namespace.clone(),
                ids: k.ids.clone(),
                revoked: k
                    .keys
                    .iter()
                    .filter(|key| !trusted.contains(key))
                    .filter_map(|key| signing::key_id(key))
                    .collect(),
                by: k.by.clone(),
                at: k.at,
                here: self.team_key_file(team).is_file(),
            }
        });
        let list = store.revocations.as_ref().and_then(|r| {
            crate::plugins::store::check_list(&store.trusted_keys(), &r.json, &r.minisig, 0).ok()
        });
        let plugins = store
            .latest()
            .into_values()
            .map(|p| {
                let s = &p.statement;
                let here = installed.iter().find(|v| v.id == s.id);
                TeamPluginRow {
                    id: s.id.clone(),
                    name: text_in(&s.name, lang),
                    description: text_in(&s.description, lang),
                    version: s.version.clone(),
                    by: s.by.clone(),
                    signed_at: s.signed_at,
                    digest: s.digest.clone(),
                    installed: here
                        .filter(|v| v.origin.team() == Some(team))
                        .map(|v| v.version.clone()),
                    elsewhere: here.is_some_and(|v| v.origin.team() != Some(team)),
                    withdrawn: list
                        .as_ref()
                        .and_then(|l| l.find(&s.id, &s.version, "disable"))
                        .map(|e| e.reason.get(lang).to_owned()),
                }
            })
            .collect();
        let proposed = store
            .proposed
            .iter()
            .map(|p| ProposalRow {
                id: p.id.clone(),
                plugin: p.submission.plugin.clone(),
                name: text_in(&p.submission.name, lang),
                version: p.submission.version.clone(),
                tag: p.submission.tag.clone(),
                by: p.by.clone(),
                mine: p.by == of.me,
                at: p.at,
                digest: p.submission.digest.clone(),
                checks: p.submission.checks.clone(),
            })
            .collect();
        TeamPluginsView {
            team: team.to_owned(),
            name: of.name.clone(),
            admin: of.admin,
            conflict: store
                .key
                .as_ref()
                .and_then(|k| self.plugins.team_conflict(&k.prefix, team)),
            key,
            plugins,
            proposed,
            serial: store.revocations.as_ref().map(|r| r.serial),
            refused: store.refusals.len(),
            alarm: store.alarm.is_some(),
        }
    }

    /// The team's store, read from the server (offline: as last read).
    pub(crate) async fn team_plugins(&self, team: &str) -> Result<TeamPluginsView, String> {
        let teams = self.team_link_or()?;
        let of = teams.plugins_store(team).await?;
        self.team_plugins_apply(team, &of.store);
        Ok(self.plugins_view_of(&of, team))
    }

    /// The stores of every team this device is in: what the page Plugins shows.
    pub(crate) async fn team_plugins_all(&self) -> Vec<TeamPluginsView> {
        let Some(teams) = self.teams_link() else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for t in teams.list().await.into_iter().filter(|t| t.joined) {
            // Read from the team's server (offline: as last read), its revocation list applied on the way.
            match teams.plugins_store(&t.id).await {
                Ok(of) => {
                    self.team_plugins_apply(&t.id, &of.store);
                    out.push(self.plugins_view_of(&of, &t.id));
                }
                Err(e) => log::info!("team {}: its store not read: {e}", t.id),
            }
        }
        out
    }

    /// The team's revocation list, its signature checked by the team's keys; None when it has none, or it does not hold.
    fn team_revocation_list(
        team: &str,
        store: &Store,
    ) -> Option<crate::plugins::store::RevocationList> {
        store.revocations.as_ref().and_then(|r| {
            crate::plugins::store::check_list(&store.trusted_keys(), &r.json, &r.minisig, 0)
                .map_err(|e| log::warn!("team {team}: its revocation list refused: {e:?}"))
                .ok()
        })
    }

    /// Applies the team's revocation list to its plugins here, as the store's list is (TS-14, the kill switch of §9).
    fn team_plugins_apply(&self, team: &str, store: &Store) {
        let list = Self::team_revocation_list(team, store);
        if list.is_some() {
            self.plugins.apply_team(team, list);
        }
    }

    /// After each reading of the team: its revocation list applied to its plugins here, and read by the circuit breaker
    /// (TS-15, `team_breakers.rs`): a plugin it withdraws in every version is taken out through the locks.
    pub(crate) fn team_plugins_after_read(self: &Arc<Self>, team: &str) {
        if let Some(of) = self.teams_link().and_then(|t| t.plugins_view(team)) {
            self.team_plugins_apply(team, &of.store);
            let list = Self::team_revocation_list(team, &of.store);
            self.set_team_withdrawals(team, list.as_ref());
        }
    }

    /// An administrator makes the team's key and its backup on this machine, and declares them with the namespace.
    /// The backup key's file goes to `backup_dir`: its path.
    pub(crate) async fn team_plugin_key_create(
        &self,
        team: &str,
        namespace: &str,
        password: &str,
        backup_dir: &Path,
    ) -> Result<(TeamPluginsView, String), String> {
        let teams = self.team_link_or()?;
        let prefix = plugins::namespace(namespace).map_err(|e| match e {
            plugins::BadNamespace::FirstParty => tr!(
                "dev.escouade.* est à Escouade : jamais l’espace d’une équipe.",
                "dev.escouade.* is Escouade’s own: never a team’s namespace."
            ),
            plugins::BadNamespace::Form => tr!(
                "Un espace de noms est un domaine à l’envers, de deux parties au moins, suivi de .* (fr.acme.*).",
                "A namespace is a reversed domain of two parts at least, then .* (fr.acme.*)."
            ),
        })?;
        if let Some(id) = self.plugins.team_conflict(&prefix, team) {
            return Err(tr!(
                "{id} est installé ici d’une autre origine et il est dans {prefix}.* : choisis un autre espace.",
                "{id} is installed here from another origin and is in {prefix}.*: choose another namespace."
            ));
        }
        if self.team_key_file(team).exists() {
            return Err(tr!(
                "Ce poste garde déjà une clé de cette équipe.",
                "This device keeps a key of this team already."
            ));
        }
        let (daily, backup) = (
            SigningKey::generate().map_err(|e| e.to_string())?,
            SigningKey::generate().map_err(|e| e.to_string())?,
        );
        let short = |e: escouade_team::Error| match e {
            escouade_team::Error::Refused(_) => tr!(
                "Un mot de passe de {n} caractères au moins.",
                "A password of {n} characters at least.",
                n = signing::MIN_PASSWORD
            ),
            e => e.to_string(),
        };
        let sealed = daily.seal(password).map_err(short)?;
        let backup_sealed = backup.seal(password).map_err(short)?;
        std::fs::create_dir_all(backup_dir)
            .map_err(|e| format!("{}: {e}", backup_dir.display()))?;
        let backup_file = backup_dir.join(format!("escouade-team-{}-backup.key", backup.id()));
        crate::paths::write_atomic(&backup_file, backup_sealed.as_bytes())
            .and_then(|()| {
                crate::paths::write_atomic(
                    &backup_file.with_extension("pub"),
                    backup.public_file().as_bytes(),
                )
            })
            .map_err(|e| format!("{}: {e}", backup_file.display()))?;
        let of = teams
            .plugins_declare(
                team,
                vec![daily.public(), backup.public()],
                &format!("{prefix}.*"),
            )
            .await?;
        let dir = self.team_key_dir(team);
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        crate::paths::write_atomic(&self.team_key_file(team), sealed.as_bytes())
            .and_then(|()| {
                crate::paths::write_atomic(&dir.join("team.pub"), daily.public_file().as_bytes())
            })
            .map_err(|e| e.to_string())?;
        Ok((
            self.plugins_view_of(&of, team),
            backup_file.display().to_string(),
        ))
    }

    /// Another administrator's machine takes the team's sealed key (or the backup key) from a file: kept only when it
    /// opens with its password and is one of the team's trusted keys.
    pub(crate) async fn team_plugin_key_import(
        &self,
        team: &str,
        file: &Path,
        password: &str,
    ) -> Result<TeamPluginsView, String> {
        let teams = self.team_link_or()?;
        let text = std::fs::read_to_string(file).map_err(|e| format!("{}: {e}", file.display()))?;
        let key = SigningKey::open(&text, password).map_err(|_| self.wrong_password())?;
        let of = teams.plugins_store(team).await?;
        if !of.store.trusted_keys().contains(&key.public()) {
            return Err(tr!(
                "Cette clé n’est pas une clé de confiance de l’équipe.",
                "This key is not one of the team’s trusted keys."
            ));
        }
        let dir = self.team_key_dir(team);
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        crate::paths::write_atomic(&self.team_key_file(team), text.as_bytes())
            .map_err(|e| e.to_string())?;
        Ok(self.plugins_view_of(&of, team))
    }

    fn wrong_password(&self) -> String {
        tr!(
            "Le mot de passe n’ouvre pas la clé de l’équipe.",
            "The password does not open the team’s key."
        )
    }

    /// The team's key kept on this device, opened by `password`, and checked to be one the team trusts.
    fn team_key(&self, team: &str, store: &Store, password: &str) -> Result<SigningKey, String> {
        let text = std::fs::read_to_string(self.team_key_file(team)).map_err(|_| {
            tr!(
                "Ce poste ne garde pas la clé de l’équipe : importe-la d’abord.",
                "This device does not keep the team’s key: import it first."
            )
        })?;
        let key = SigningKey::open(&text, password).map_err(|_| self.wrong_password())?;
        if !store.trusted_keys().contains(&key.public()) {
            return Err(tr!(
                "La clé de ce poste n’est plus une clé de confiance de l’équipe (révoquée ou remplacée).",
                "This device’s key is no longer one of the team’s trusted keys (revoked or replaced)."
            ));
        }
        Ok(key)
    }

    /// The last version of `id` the team published, if any.
    fn last_published(store: &Store, id: &str) -> Option<String> {
        store.latest().get(id).map(|p| p.statement.version.clone())
    }

    /// The id of the plugin in `folder`, as its manifest says (to know its last version before the checks).
    fn id_in(folder: &Path) -> Option<String> {
        let text =
            std::fs::read_to_string(folder.join(escouade_plugin_cli::manifest::FILE)).ok()?;
        serde_json::from_str::<Value>(&text)
            .ok()?
            .get("id")?
            .as_str()
            .map(str::to_owned)
    }

    /// A member proposes the plugin of `folder` at the tag `tag`: packed, checked (the compilation twice), and only when
    /// every check passes, encrypted on the server with its digest.
    pub(crate) async fn team_plugin_submit(
        &self,
        team: &str,
        folder: &Path,
        tag: &str,
    ) -> Result<Submitted, String> {
        let teams = self.team_link_or()?;
        let lang = crate::i18n::ui();
        let of = teams.plugins_store(team).await?;
        let key = of.store.key.clone().ok_or_else(|| {
            tr!(
                "L’équipe n’a pas encore de clé pour son store : un administrateur la crée.",
                "The team has no key for its store yet: an administrator makes it."
            )
        })?;
        let gz = checks::pack(folder, lang)?;
        let last = Self::id_in(folder).and_then(|id| Self::last_published(&of.store, &id));
        let report = tokio::task::block_in_place(|| {
            checks::check(
                &gz,
                &checks::Against {
                    tag,
                    last: last.as_deref(),
                    app_version: app_version(),
                },
                &compiler(),
                lang,
            )
        });
        if !report.ok {
            return Ok(Submitted {
                report,
                proposal: None,
                view: None,
            });
        }
        if !plugins::may_sign(&key.prefix, &report.id) {
            return Err(tr!(
                "{id} n’est pas dans l’espace de noms de l’équipe ({ns}) : refusé.",
                "{id} is not in the team’s namespace ({ns}): refused.",
                id = report.id,
                ns = key.namespace
            ));
        }
        if let Some(other) = self.plugins.team_conflict(&key.prefix, team) {
            return Err(tr!(
                "{other} est installé ici d’une autre origine dans {ns} : le store de l’équipe est refusé sur ce poste.",
                "{other} is installed here from another origin in {ns}: the team’s store is refused on this device.",
                ns = key.namespace
            ));
        }
        let m = report.manifest.as_ref();
        let submission = Submission {
            plugin: report.id.clone(),
            version: report.version.clone(),
            tag: tag.trim().to_owned(),
            name: m
                .and_then(|m| serde_json::to_value(&m.name).ok())
                .unwrap_or_default(),
            description: m
                .and_then(|m| m.description.as_ref())
                .and_then(|d| serde_json::to_value(d).ok())
                .unwrap_or_default(),
            archive: [0; 16],
            module: None,
            archive_sha256: report.archive_sha256.clone(),
            module_sha256: report.module_sha256.clone(),
            digest: report.digest.clone(),
            checks: report
                .checks
                .iter()
                .map(|c| Checked {
                    key: c.key.clone(),
                    ok: c.ok,
                    detail: c.detail.clone(),
                })
                .collect(),
        };
        let (id, of) = teams
            .plugins_submit(team, submission, &gz, report.module.as_deref())
            .await?;
        Ok(Submitted {
            view: Some(self.plugins_view_of(&of, team)),
            proposal: Some(id),
            report,
        })
    }

    /// An administrator signs the proposal `proposal`: its archive read back from the server, the same checks played
    /// again here, signed by the team's key only when this machine's digest is the proposal's.
    pub(crate) async fn team_plugin_sign(
        &self,
        team: &str,
        proposal: &str,
        password: &str,
    ) -> Result<(checks::Report, TeamPluginsView), String> {
        let teams = self.team_link_or()?;
        let lang = crate::i18n::ui();
        let of = teams.plugins_store(team).await?;
        if !of.admin {
            return Err(tr!(
                "Seul un administrateur de l’équipe signe un plugin.",
                "Only an administrator of the team signs a plugin."
            ));
        }
        let p = of
            .store
            .proposed
            .iter()
            .find(|p| p.id == proposal)
            .cloned()
            .ok_or_else(|| {
                tr!(
                    "Cette proposition n’attend plus.",
                    "This proposal is no longer waiting."
                )
            })?;
        let key = self.team_key(team, &of.store, password)?;
        let s = &p.submission;
        let (archive, _module) = teams
            .plugins_files(
                team,
                s.archive,
                &s.archive_sha256,
                s.module,
                s.module_sha256.as_deref(),
            )
            .await?;
        let last = Self::last_published(&of.store, &s.plugin);
        let report = tokio::task::block_in_place(|| {
            checks::check(
                &archive,
                &checks::Against {
                    tag: &s.tag,
                    last: last.as_deref(),
                    app_version: app_version(),
                },
                &compiler(),
                lang,
            )
        });
        if !report.ok {
            return Err(tr!(
                "Les contrôles échouent sur ce poste : {why}. Rien n’est signé.",
                "The checks fail on this device: {why}. Nothing is signed.",
                why = report.failure().unwrap_or_default()
            ));
        }
        if report.digest != s.digest || report.id != s.plugin || report.version != s.version {
            return Err(tr!(
                "Les empreintes diffèrent : {theirs} chez qui a proposé, {mine} ici. Rien n’est signé.",
                "The digests differ: {theirs} on the machine of who proposed, {mine} here. Nothing is signed.",
                theirs = &s.digest[..12.min(s.digest.len())],
                mine = &report.digest[..12]
            ));
        }
        let of = teams
            .plugins_publish(team, proposal, &report.digest, &key)
            .await?;
        Ok((report, self.plugins_view_of(&of, team)))
    }

    /// An administrator withdraws versions of a plugin of the team (`versions` None: every one): the team's revocation
    /// list, its serial one past the last, signed by the team's key. `revoke_key`: the id of a team's key the list
    /// revokes too (the backup key takes over from a key of every day that was lost).
    pub(crate) async fn team_plugin_revoke(
        &self,
        team: &str,
        plugin: &str,
        versions: Option<Vec<String>>,
        reason: &str,
        password: &str,
        revoke_key: Option<String>,
    ) -> Result<TeamPluginsView, String> {
        let teams = self.team_link_or()?;
        let of = teams.plugins_store(team).await?;
        let prefix = of
            .store
            .key
            .as_ref()
            .map(|k| k.prefix.clone())
            .ok_or_else(|| tr!("L’équipe n’a pas de clé.", "The team has no key."))?;
        let key = self.team_key(team, &of.store, password)?;
        let (mut entries, mut revoked, serial) = match &of.store.revocations {
            Some(r) => {
                let v: Value = serde_json::from_slice(&r.json).unwrap_or_default();
                (
                    v.get("entries")
                        .and_then(Value::as_array)
                        .cloned()
                        .unwrap_or_default(),
                    v.get("keys_revoked")
                        .and_then(Value::as_array)
                        .cloned()
                        .unwrap_or_default(),
                    r.serial + 1,
                )
            }
            None => (Vec::new(), Vec::new(), 1),
        };
        if !plugin.is_empty() {
            let reason = reason.trim();
            entries.push(serde_json::json!({
                "id": plugin,
                "versions": versions.map(Value::from).unwrap_or_else(|| Value::from("*")),
                "action": "disable",
                "reason": { "fr": reason, "en": reason },
                "date": chrono::Utc::now().to_rfc3339(),
            }));
        }
        if let Some(id) = revoke_key {
            if id == key.id() {
                return Err(tr!(
                    "Une clé ne se révoque pas elle-même : signe avec l’autre clé de l’équipe.",
                    "A key does not revoke itself: sign with the team’s other key."
                ));
            }
            revoked.push(Value::from(id));
        }
        let mut list = serde_json::json!({
            "serial": serial,
            "issued_at": chrono::Utc::now().to_rfc3339(),
            "entries": entries,
        });
        if !revoked.is_empty() {
            list["keys_revoked"] = Value::from(revoked);
        }
        let json = serde_json::to_vec(&list).map_err(|e| e.to_string())?;
        let minisig = key
            .sign(
                &json,
                &format!("{} {serial} {prefix}", plugins::REVOCATIONS_COMMENT),
            )
            .map_err(|e| e.to_string())?;
        let of = teams.plugins_revoke(team, json, minisig).await?;
        self.team_plugins_apply(team, &of.store);
        Ok(self.plugins_view_of(&of, team))
    }

    /// A proposal set aside (an administrator, or its author).
    pub(crate) async fn team_plugin_dismiss(
        &self,
        team: &str,
        proposal: &str,
    ) -> Result<TeamPluginsView, String> {
        let teams = self.team_link_or()?;
        let of = teams.plugins_dismiss(team, proposal).await?;
        Ok(self.plugins_view_of(&of, team))
    }

    /// What installing the version `version` (None: the last) of the team's plugin `id` would do: read from the
    /// team's server, checked again here, set aside for the window of its permissions. Installing is the member's
    /// gesture: `plugin_install` with its token.
    pub(crate) async fn team_plugin_inspect(
        &self,
        team: &str,
        id: &str,
        version: Option<&str>,
    ) -> Result<crate::plugins::Preview, crate::plugins::Refusal> {
        let lang = crate::i18n::ui();
        let other = crate::plugins::Refusal::other;
        let teams = self.team_link_or().map_err(other)?;
        let of = teams.plugins_store(team).await.map_err(other)?;
        let key = of.store.key.clone().ok_or_else(|| {
            other(tr!(
                "L’équipe n’a pas de clé pour son store.",
                "The team has no key for its store."
            ))
        })?;
        let p = match version {
            Some(v) => of.store.version(id, v).cloned(),
            None => of.store.latest().get(id).map(|p| (*p).clone()),
        }
        .ok_or_else(|| {
            other(tr!(
                "Cette version n’est pas au store de l’équipe.",
                "This version is not in the team’s store."
            ))
        })?;
        let view = self.plugins_view_of(&of, team);
        if let Some(why) = view
            .plugins
            .iter()
            .find(|r| r.id == id && r.version == p.statement.version)
            .and_then(|r| r.withdrawn.clone())
        {
            return Err(other(tr!(
                "Cette version est retirée par l’équipe : {why}",
                "This version is withdrawn by the team: {why}"
            )));
        }
        let (archive, module) = teams
            .plugins_files(
                team,
                p.archive,
                &p.statement.archive_sha256,
                p.module,
                p.statement.module_sha256.as_deref(),
            )
            .await
            .map_err(other)?;
        self.plugins.inspect_team(
            crate::plugins::team::TeamFetched {
                team: team.to_owned(),
                name: of.name.clone(),
                mark: of.mark.clone(),
                prefix: key.prefix.clone(),
                keys: of.store.trusted_keys(),
                statement: p.statement_bytes.clone(),
                minisig: p.minisig.clone(),
                archive,
                module,
            },
            lang,
        )
    }
}
