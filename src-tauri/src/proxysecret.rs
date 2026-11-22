//! The proxy's password in the system's keychain (SEC-10, ESC-595), never in the settings' file nor in the window: the
//! address is written without it (`escouade_model::proxy`), and the password is put back in it in memory, where the
//! requests and the agents' variables read it.
//!
//! - At the start (`on_load`): an address written with its password by an earlier version gives it to the keychain,
//!   and the file is written again without it (the migration); an address with a user's name gets its password back
//!   from the keychain, or from the file apart (`Settings::proxy_password_in_file`) where the keychain refused it.
//! - At each save (`on_save`): the window holds the address without its password; given back with the same user's name,
//!   it keeps the password it had. A new password goes to the keychain, a removed one leaves it. A keychain that
//!   refuses (none under Linux without a Secret Service) leaves the password in the file apart, readable by its owner
//!   alone, as the accounts' secrets.

use crate::integrations::secrets::SecretStore;
use crate::model::Settings;
use escouade_model::proxy::{password_of, user_of, with_password, SECRET_ENTRY};

/// The password of `settings` given to the keychain: kept there, the file apart emptied; refused, it stays in the file
/// apart.
fn keep(settings: &mut Settings, password: &str, secrets: &dyn SecretStore) {
    match secrets.set(SECRET_ENTRY, password) {
        Ok(()) => {
            settings.proxy_password_in_file.clear();
            settings.proxy_password_kept = true;
        }
        Err(e) => {
            settings.proxy_password_kept = false;
            if !escouade_plugins::secrets::is_no_keychain(&e) {
                log::warn!(
                    "proxy: the keychain refused the password, kept in the settings' file: {e:#}"
                );
            }
            settings.proxy_password_in_file = password.to_string();
        }
    }
}

/// The settings as read at the start, their proxy's password put back in memory. True when the file is to be written
/// again: the password was in the address (an earlier version), or in the file apart while the keychain takes it now.
pub fn on_load(settings: &mut Settings, secrets: &dyn SecretStore) -> bool {
    let url = settings.proxy_url.clone();
    if let Some(password) = password_of(&url) {
        keep(settings, &password, secrets);
        log::info!("proxy: the password left the settings' address (SEC-10)");
        return true;
    }
    if user_of(&url).is_none() {
        return false;
    }
    let kept = secrets.get(SECRET_ENTRY).ok().flatten();
    let in_file = std::mem::take(&mut settings.proxy_password_in_file);
    let Some(password) = kept
        .clone()
        .or_else(|| (!in_file.is_empty()).then(|| in_file.clone()))
    else {
        // Kept in the keychain once, and gone (the keychain emptied, another account of the system): said, and with
        // « Ne jamais sortir sans le proxy » no agent starts (`refuse_without_password`).
        if settings.proxy_password_kept {
            log::warn!("proxy: its password is not in the keychain any more");
            settings.proxy_password_missing = true;
        }
        return false;
    };
    settings.proxy_url = with_password(&url, &password);
    if in_file.is_empty() {
        return false;
    }
    // In the file apart: to the keychain if it takes it now.
    keep(settings, &password, secrets);
    settings.proxy_password_in_file != in_file
}

/// `s`, the settings the window saves, with the proxy's password as it is to be: the one `before` had when the address
/// comes back without one and with the same user's name; the keychain told of a change.
pub fn on_save(before: &Settings, s: &mut Settings, secrets: &dyn SecretStore) {
    let had = password_of(&before.proxy_url);
    if password_of(&s.proxy_url).is_none() {
        if let (Some(p), Some(user)) = (&had, user_of(&s.proxy_url)) {
            if user_of(&before.proxy_url).as_deref() == Some(user.as_str()) {
                s.proxy_url = with_password(&s.proxy_url, p);
            }
        }
    }
    // The window's copy of the file apart is not the truth: the backend's is.
    s.proxy_password_in_file = before.proxy_password_in_file.clone();
    s.proxy_password_kept = before.proxy_password_kept;
    s.proxy_password_missing = before.proxy_password_missing;
    let now = password_of(&s.proxy_url);
    if now == had {
        return;
    }
    s.proxy_password_missing = false;
    match now {
        Some(p) => keep(s, &p, secrets),
        None => {
            s.proxy_password_kept = false;
            if let Err(e) = secrets.delete(SECRET_ENTRY) {
                log::warn!("proxy: the password could not leave the keychain: {e:#}");
            }
            s.proxy_password_in_file.clear();
        }
    }
}

/// Refused (SEC-14): an agent's process, or its interface, while « Ne jamais sortir sans le proxy » is on and the proxy's
/// password kept in the keychain was not found there at this start. It would go out with an address the proxy refuses,
/// or around the proxy: it does not start, and says how to give the password again.
pub fn refuse_without_password(settings: &Settings) -> anyhow::Result<()> {
    if settings.proxy_held_back() {
        anyhow::bail!(tr!(
            "Le mot de passe du proxy n’est plus dans le trousseau du système, et « Ne jamais sortir sans le proxy » est allumé : l’agent n’est pas lancé. Réécris l’adresse du proxy avec son mot de passe dans Réglages › Avancé › Réseau.",
            "The proxy’s password is no longer in the system keychain, and “Never go out without the proxy” is on: the agent is not started. Type the proxy’s address again with its password in Settings › Advanced › Network."
        ));
    }
    Ok(())
}
