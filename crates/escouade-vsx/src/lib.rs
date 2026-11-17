//! The extensions of the editor's view (ESC-481, A5 of docs/EDITOR-STUDY.md), by the core: the page never reaches Open
//! VSX nor the disk for them.
//!
//! - [`registry`]: Open VSX's API (search, a card, its files), through the client the app gives (its proxy, its
//!   authorities); every address it is handed is checked to be the registry's own.
//! - [`verify`]: a VSIX's SHA-256 and Open VSX's signature of it (Ed25519 over the VSIX's bytes, with the public key Open
//!   VSX publishes). A version without a signature is not installed.
//! - [`unpack`]: the VSIX unpacked within its folder: no `..`, no link, no absolute path, sizes and counts bounded.
//! - [`allowed`]: `extensions.allowed`, VS Code's rule (by publisher or by extension), each layer that sets it crossed.
//! - [`store`]: what is installed, in Escouade's data folder; the publishers trusted; the theme chosen.
//! - [`serve`]: an installed extension's files, read-only, with their MIME type (the protocol `escouade-ext:` and the
//!   page's command go through it), never another file of the disk.
//! - [`install`]: the whole of an install or an update: allowed first (nothing downloaded otherwise), the card, the
//!   trust of its publisher, the download, the checks, the unpacking, the store.

#[macro_use]
extern crate escouade_plugin_cli;

/// A text in the interface's language: `tr!("français", "English", args…)`, as the app's.
macro_rules! tr {
    ($($t:tt)+) => {
        tr_in!(escouade_base::i18n::ui(), $($t)+)
    };
}

pub mod allowed;
pub mod install;
pub mod manifest;
pub mod registry;
pub mod serve;
pub mod store;
pub mod unpack;
pub mod verify;

#[cfg(any(test, feature = "testing"))]
pub mod fake;

#[cfg(test)]
mod tests;

use serde::Serialize;

/// An extension's id, `publisher.name`, as Open VSX and VS Code name it (case kept, compared without it).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtId {
    pub namespace: String,
    pub name: String,
}

/// A name of Open VSX (a namespace, an extension): letters, digits, `-` and `_`, starting with a letter or a digit.
/// What goes into an address of the registry and into a folder's name: nothing else passes.
pub fn is_name(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.chars().next().is_some_and(|c| c.is_ascii_alphanumeric())
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// A version as Open VSX gives it (`1.2.3`, `1.2.3-beta.1`): digits, letters, `.`, `-` and `+`, no other sign.
pub fn is_version(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.chars().next().is_some_and(|c| c.is_ascii_digit())
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+'))
        && !s.contains("..")
}

impl ExtId {
    /// `publisher.name`, checked.
    pub fn parse(id: &str) -> std::result::Result<ExtId, Refusal> {
        let (namespace, name) = id.split_once('.').unwrap_or((id, ""));
        if !is_name(namespace) || !is_name(name) {
            return Err(Refusal::new(
                Code::BadId,
                tr!(
                    "« {id} » n’est pas l’identifiant d’une extension (éditeur.nom)",
                    "“{id}” is not an extension’s id (publisher.name)"
                ),
            ));
        }
        Ok(ExtId {
            namespace: namespace.to_owned(),
            name: name.to_owned(),
        })
    }

    /// `publisher.name`, in lower case: how it is compared and kept.
    pub fn key(&self) -> String {
        format!("{}.{}", self.namespace, self.name).to_lowercase()
    }
}

impl std::fmt::Display for ExtId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.namespace, self.name)
    }
}

/// Why an extension is not searched, installed or served: what the page says, by its code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Code {
    BadId,
    /// Outside the allowed list: refused before anything is downloaded.
    NotAllowed,
    /// Its publisher is not trusted yet: the page asks the question, then asks again.
    Untrusted,
    /// A version Open VSX did not sign.
    Unsigned,
    /// Its SHA-256 is not the one Open VSX gives.
    BadSum,
    /// Its signature does not hold with Open VSX's key.
    BadSignature,
    /// Its archive holds what never goes in a folder of extension (`..`, a link, an absolute path), or too much.
    Trapped,
    /// Its archive is not the extension of the card (another id or version).
    Mismatch,
    /// A private project: nothing is asked of Open VSX from it.
    Private,
    NotFound,
    Network,
    Io,
}

/// A refusal, said in the interface's language.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Refusal {
    pub code: Code,
    pub message: String,
}

impl Refusal {
    pub fn new(code: Code, message: impl Into<String>) -> Self {
        Refusal {
            code,
            message: message.into(),
        }
    }

    pub fn io(e: impl std::fmt::Display) -> Self {
        Refusal::new(
            Code::Io,
            tr!(
                "écriture des extensions impossible : {e}",
                "the extensions cannot be written: {e}"
            ),
        )
    }

    pub fn network(e: impl std::fmt::Display) -> Self {
        Refusal::new(
            Code::Network,
            tr!(
                "Open VSX n’a pas répondu : {e}",
                "Open VSX did not answer: {e}"
            ),
        )
    }
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Refusal {}

pub type Result<T> = std::result::Result<T, Refusal>;
