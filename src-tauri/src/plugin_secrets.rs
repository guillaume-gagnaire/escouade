//! A plugin's secrets as its settings keep them (« Garder », « Oublier »), and the file the user may choose for them
//! where the system has no keychain (issue #37, ESC-499; `integrations::kept`).
//!
//! « Garder » never fails in silence: a secret the keychain did not keep is an error the window shows beside its field,
//! with its cause (`refusal`), and the offer of the file; the file is the user's choice, made at the field, never taken
//! by itself. A plugin reads its secret by `secrets/get` wherever it is kept: absent, `null`, as before.

use crate::core::Core;
use crate::integrations::secrets::{keep_secret, SecretStore};
use crate::plugins::runtime::secret_name;
use serde::Serialize;

/// A secret of a plugin, as its settings show it: kept or not, and where.
#[derive(Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SecretState {
    pub key: String,
    pub kept: bool,
    /// In the file the user chose, not in the keychain.
    pub in_file: bool,
    /// A secret on several lines (`multiline`), kept: how many lines and bytes, never its text (« Gardé · 84 lignes ·
    /// 5,1 Ko »). It is not read back in the window: it is replaced.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<SecretSize>,
}

/// What the page of a plugin says of a secret on several lines it keeps: its lines and its bytes, never a character of it.
#[derive(Serialize, Debug, PartialEq, Eq, Clone, Copy)]
pub struct SecretSize {
    pub lines: usize,
    pub bytes: usize,
}

impl SecretSize {
    pub fn of(value: &str) -> Self {
        Self {
            lines: value.lines().count().max(1),
            bytes: value.len(),
        }
    }
}

/// The keys of the secrets of `manifest` declared on several lines (`"multiline": true`).
pub fn multiline_secrets(manifest: Option<&crate::plugins::manifest::Manifest>) -> Vec<String> {
    use serde_json::Value;
    manifest
        .and_then(|m| m.contributes.get("secrets"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|s| s.get("multiline").and_then(Value::as_bool) == Some(true))
        .filter_map(|s| s.get("key").and_then(Value::as_str).map(str::to_string))
        .collect()
}

/// What « Garder » says when the secret was not kept: its cause, in the user's language, and whether the keychain was
/// the cause (the file is then offered).
#[derive(Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SecretRefused {
    pub message: String,
    pub keychain_refused: bool,
}

impl SecretRefused {
    fn other(message: String) -> Self {
        Self {
            message,
            keychain_refused: false,
        }
    }
}

/// A secret the file holds, as Réglages › Plugins lists it.
#[derive(Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileSecret {
    pub plugin: String,
    /// The plugin's name, or its id when it is no longer installed.
    pub plugin_name: String,
    pub key: String,
}

/// The file of the plugins' secrets on this machine: chosen or not, where, what it holds, and whether a keychain is
/// there to move them to.
#[derive(Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SecretsFile {
    pub chosen: bool,
    pub path: String,
    pub keychain: bool,
    pub secrets: Vec<FileSecret>,
}

/// What « Garder » says of a secret longer than 64 KiB (ESC-527): refused, in the keychain as in the file.
pub fn too_long() -> String {
    too_long_in(crate::i18n::ui())
}

/// `too_long`, in `lang`.
pub fn too_long_in(lang: crate::i18n::Lang) -> String {
    tr_in!(
        lang,
        "Ce secret dépasse 64 Kio : il n’a pas été gardé.",
        "This secret is longer than 64 KiB: it was not kept."
    )
}

/// Why the keychain did not keep a secret, said simply: no keychain on this machine, or the one there refused (its
/// reason, the secret never in it).
pub fn refusal(e: &anyhow::Error, secret: &str) -> String {
    refusal_in(crate::i18n::ui(), e, secret)
}

/// `refusal`, in `lang`.
pub fn refusal_in(lang: crate::i18n::Lang, e: &anyhow::Error, secret: &str) -> String {
    if escouade_plugins::secrets::is_no_keychain(e) {
        return tr_in!(
            lang,
            "Pas de trousseau système sur ce poste : le secret n’a pas été gardé.",
            "No system keychain on this machine: the secret was not kept."
        );
    }
    if escouade_plugins::secrets::is_too_long(e) {
        return too_long_in(lang);
    }
    let why = format!("{e:#}");
    let why = if secret.is_empty() {
        why
    } else {
        why.replace(secret, "••••")
    };
    tr_in!(
        lang,
        "Le trousseau système a refusé le secret ({why}) : il n’a pas été gardé.",
        "The system keychain refused the secret ({why}): it was not kept."
    )
}

impl Core {
    /// The secrets `keys` of the plugin `id`: kept or not, and where; of those on several lines (`multiline`), their
    /// lines and bytes.
    pub fn plugin_secret_states(
        &self,
        id: &str,
        keys: Vec<String>,
        multiline: &[String],
    ) -> Vec<SecretState> {
        keys.into_iter()
            .map(|key| {
                let name = secret_name(id, &key);
                // In the file, read first; or in it still while a policy denies it (ESC-520), to move to the keychain.
                let in_file = self.kept.in_file(&name);
                let value = self.secrets.get(&name).ok().flatten();
                let kept = in_file || value.is_some();
                SecretState {
                    in_file,
                    kept,
                    size: value
                        .filter(|_| multiline.contains(&key))
                        .map(|v| SecretSize::of(&v)),
                    key,
                }
            })
            .collect()
    }

    /// « Garder » (`value`; empty: « Oublier »): in the keychain, read back to check, or where it already is when the
    /// user put it in the file. `in_file`: the user's choice at the field, « Garder dans un fichier lisible de toi
    /// seul ». A keychain that refuses is an error that says why, and offers the file; the file is never taken by itself.
    pub fn plugin_secret_set(
        &self,
        id: &str,
        key: &str,
        value: &str,
        in_file: bool,
    ) -> Result<(), SecretRefused> {
        self.plugin_secret_set_named(id, key, &secret_name(id, key), value, in_file)
    }

    /// `plugin_secret_set` under the name `name` in the keychain (a profile's secret, ESC-530): `key` is what the logs
    /// say. The name is noted in the plugin's list of the names kept (`Choices::secret_names`), and taken out of it once
    /// forgotten: its uninstall erases them, the keychain listing nothing by prefix.
    pub fn plugin_secret_set_named(
        &self,
        id: &str,
        key: &str,
        name: &str,
        value: &str,
        in_file: bool,
    ) -> Result<(), SecretRefused> {
        let done = self.keep_named(id, key, name, value, in_file);
        if done.is_ok() {
            self.note_secret_name(id, name, !value.is_empty());
        }
        done
    }

    /// Notes (`kept`) or takes out the name `name` in the list of the names of the plugin `id`'s secrets.
    pub(crate) fn note_secret_name(&self, id: &str, name: &str, kept: bool) {
        let _ = self.plugins.choose(id, |c| {
            c.secret_names.retain(|n| n != name);
            if kept {
                c.secret_names.push(name.to_string());
            }
        });
    }

    fn keep_named(
        &self,
        id: &str,
        key: &str,
        name: &str,
        value: &str,
        in_file: bool,
    ) -> Result<(), SecretRefused> {
        let name = name.to_string();
        if value.is_empty() {
            return self.secrets.delete(&name).map_err(|e| {
                log::warn!("plugin {id}: its secret {key} was not forgotten: {e:#}");
                SecretRefused::other(tr!(
                    "Le secret n’a pas été oublié : {e:#}",
                    "The secret was not forgotten: {e:#}"
                ))
            });
        }
        if value.len() > escouade_plugins::secrets::SECRET_MAX {
            return Err(SecretRefused::other(too_long()));
        }
        if in_file {
            // Denied by a policy of the device or of a team (`secrets.fileFallback`, ESC-520): said as it is.
            if let Some(why) = self.kept.denied() {
                return Err(SecretRefused::other(why));
            }
            return self.kept.keep_in_file(&name, value).map_err(|e| {
                log::warn!("plugin {id}: its secret {key} could not be written to its file: {e:#}");
                SecretRefused::other(tr!(
                    "Le fichier n’a pas pu être écrit ({e:#}) : le secret n’a pas été gardé.",
                    "The file could not be written ({e:#}): the secret was not kept."
                ))
            });
        }
        keep_secret(&*self.secrets, &name, value).map_err(|e| {
            log::warn!(
                "plugin {id}: its secret {key} was not kept, the system keychain refused it: {}",
                format!("{e:#}").replace(value, "••••")
            );
            SecretRefused {
                message: refusal(&e, value),
                keychain_refused: true,
            }
        })
    }

    /// Uninstalls the plugin `id`: its process stopped, its copy, its settings, its secrets (each one whole, its pieces
    /// with it: `Chunked`) and its journal removed; `keep`: its settings and its secrets kept for a later install.
    pub fn plugin_uninstall(&self, id: &str, keep: bool) -> Result<bool, String> {
        let mut names: Vec<String> = self
            .plugins
            .get(id, crate::i18n::ui())
            .map(|v| v.secret_keys)
            .unwrap_or_default()
            .iter()
            .map(|k| secret_name(id, k))
            .collect();
        // The names it kept besides (its profiles', ESC-530): the keychain lists nothing by prefix (§2.3). Only its own.
        let prefix = secret_name(id, "");
        for n in self
            .plugins
            .choices(id)
            .map(|c| c.secret_names)
            .unwrap_or_default()
        {
            if n.starts_with(&prefix) && !names.contains(&n) {
                names.push(n);
            }
        }
        self.plugin_runtime.forget(id);
        let gone = self.plugins.remove(id, keep).map_err(|e| e.to_string())?;
        if gone && !keep {
            for n in names {
                if let Err(e) = self.secrets.delete(&n) {
                    log::warn!("plugin {id}: its secret {n} stays in the keychain: {e}");
                }
            }
            let log = self.plugin_runtime.journal_file(id);
            let _ = std::fs::remove_file(&log);
            let mut old = log.into_os_string();
            old.push(".1");
            let _ = std::fs::remove_file(old);
        }
        Ok(gone)
    }

    /// « Déplacer vers le trousseau »: the secret `key` of the plugin `id` out of the file, into the keychain.
    pub fn plugin_secret_move(&self, id: &str, key: &str) -> Result<(), String> {
        self.kept
            .move_to_keychain(&secret_name(id, key))
            .map_err(|e| refusal(&e, ""))
    }

    /// The file of the plugins' secrets, for Réglages › Plugins.
    pub fn secrets_file(&self) -> SecretsFile {
        let lang = crate::i18n::ui();
        let secrets = self
            .kept
            .names()
            .into_iter()
            .filter_map(|name| {
                let rest = name.strip_prefix("plugin:")?;
                // A plugin's id has no `:` (§3.1); its key may.
                let (plugin, key) = rest.split_once(':')?;
                let plugin_name = self
                    .plugins
                    .get(plugin, lang)
                    .map(|v| v.name)
                    .unwrap_or_else(|| plugin.to_string());
                Some(FileSecret {
                    plugin: plugin.to_string(),
                    plugin_name,
                    key: key.to_string(),
                })
            })
            .collect();
        SecretsFile {
            chosen: self.kept.chosen(),
            path: self.kept.path().display().to_string(),
            keychain: self.kept.present(),
            secrets,
        }
    }

    /// The choice taken back: the file goes, with the secrets it holds.
    pub fn secrets_file_remove(&self) -> Result<(), String> {
        self.kept.give_up().map_err(|e| format!("{e:#}"))
    }
}
