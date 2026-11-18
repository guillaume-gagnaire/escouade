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
    /// The secrets `keys` of the plugin `id`: kept or not, and where.
    pub fn plugin_secret_states(&self, id: &str, keys: Vec<String>) -> Vec<SecretState> {
        keys.into_iter()
            .map(|key| {
                let name = secret_name(id, &key);
                // In the file, read first; or in it still while a policy denies it (ESC-520), to move to the keychain.
                let in_file = self.kept.in_file(&name);
                let kept = in_file || self.secrets.get(&name).ok().flatten().is_some();
                SecretState { in_file, kept, key }
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
        let name = secret_name(id, key);
        if value.is_empty() {
            return self.secrets.delete(&name).map_err(|e| {
                log::warn!("plugin {id}: its secret {key} was not forgotten: {e:#}");
                SecretRefused::other(tr!(
                    "Le secret n’a pas été oublié : {e:#}",
                    "The secret was not forgotten: {e:#}"
                ))
            });
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
