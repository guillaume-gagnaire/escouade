//! Whisper's models (`escouade-voice-local`'s `models`), and the client of their download, which the settings make.

pub use escouade_voice_local::models::*;

use anyhow::Result;
use std::time::Duration;

/// The client of the download: the proxy and the TLS setting of the app's own requests, no limit
/// on the whole (1.6 GB behind a slow proxy takes its time), but one on a connection that stalls.
pub fn client(settings: &crate::model::Settings) -> Result<reqwest::Client> {
    Ok(crate::tls::app_builder(settings)?
        .connect_timeout(Duration::from_secs(15))
        .read_timeout(Duration::from_secs(60))
        .build()?)
}

/// What a download that failed tells the user (ESC-664), `what` the file it was: a file whose size or SHA-256 is no
/// longer the catalog's says plainly that the file published has changed and that it is for Escouade to update,
/// not « 6369 bytes and more, 4875 expected »; any other error as it is.
pub fn download_error(e: anyhow::Error, what: Downloaded) -> anyhow::Error {
    download_error_in(crate::i18n::ui(), e, what)
}

/// `download_error` in `lang`.
pub fn download_error_in(
    lang: crate::i18n::Lang,
    e: anyhow::Error,
    what: Downloaded,
) -> anyhow::Error {
    let (fr, en) = match what {
        Downloaded::Model => ("ce modèle", "this model"),
        Downloaded::Voice => ("cette voix", "this voice"),
    };
    if let Some(c) = e.downcast_ref::<FileChanged>() {
        let (got, expected) = (c.got, c.expected);
        let size = if c.more {
            tr_in!(lang, "plus de {expected} octets", "over {expected} bytes")
        } else {
            tr_in!(lang, "{got} octets", "{got} bytes")
        };
        return anyhow::anyhow!(tr_in!(
            lang,
            "Le fichier publié pour {fr} a changé ({size} au lieu de {expected} attendus) : cette version d’Escouade ne le reconnaît plus, il n’est pas gardé. Ce n’est pas de ton côté : c’est à Escouade de se mettre à jour. Installe la dernière version, ou signale-le si elle ne règle rien.",
            "The file published for {en} has changed ({size} instead of the {expected} expected): this version of Escouade no longer recognizes it, and it was not kept. It is not on your side: Escouade has to be updated. Install the latest version, or report it if that does not fix it."
        ));
    }
    if e.is::<ChecksumMismatch>() {
        return anyhow::anyhow!(tr_in!(
            lang,
            "Le fichier téléchargé pour {fr} n’est pas celui qu’attend Escouade (somme de contrôle différente) : il n’est pas gardé. Soit le fichier publié a changé et c’est à Escouade de se mettre à jour (installe la dernière version, ou signale-le), soit un proxy le modifie en route : télécharge-le ailleurs.",
            "The file downloaded for {en} is not the one Escouade expects (checksum mismatch): it was not kept. Either the file published has changed and Escouade has to be updated (install the latest version, or report it), or a proxy alters it on the way: download it elsewhere."
        ));
    }
    e
}

/// What a download fetched, for its message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Downloaded {
    Model,
    Voice,
}
