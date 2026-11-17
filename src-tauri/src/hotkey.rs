//! The Overseer's global shortcut (ESC-468): a key that opens its screen from anywhere, the app in front or not. It is
//! kept as the dictation's is (`Mod+Alt+KeyO`: its modifiers, `Mod` for Ctrl or Cmd, then the key by its place) and
//! registered with the system by `tauri-plugin-global-shortcut` (`Keys`, a fake in the tests) while the Overseer is on.
//!
//! A shortcut another app already holds is refused, and the one before is kept: the new one is registered before the
//! old one is let go. Windows says so at once (`RegisterHotKey`), macOS and X11 too for a shortcut registered the same
//! way. Under Wayland an app cannot have a global shortcut: none is registered, and the settings say to bind
//! `xdg-open escouade://overseer` in the desktop's own shortcuts (the deep link opens the screen, `deeplink`).

use parking_lot::Mutex;
use serde::Serialize;
use std::sync::{Arc, OnceLock};

/// The link a shortcut opens: the Overseer's screen (`deeplink::DeepLink::Overseer`).
pub const OVERSEER_LINK: &str = "escouade://overseer";

/// Where shortcuts are registered with the system.
pub trait Keys: Send + Sync {
    /// Registers `accel` (as `tauri-plugin-global-shortcut` writes it); why not when it cannot be.
    fn register(&self, accel: &str) -> Result<(), String>;
    fn unregister(&self, accel: &str);
}

/// Why a shortcut is not registered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HotkeyError {
    /// Not a shortcut: a key without Ctrl, Alt or Cmd, an unknown key or modifier.
    Invalid,
    /// Another app holds it, or the system refused it (its words).
    Taken(String),
    /// No global shortcut here: Wayland, or the plugin could not start.
    Unsupported,
}

impl HotkeyError {
    pub fn text(&self) -> String {
        match self {
            HotkeyError::Invalid => tr!(
                "Ce n’est pas un raccourci : une touche avec Ctrl, Alt ou Cmd (ou une touche F1 à F24).",
                "This is not a shortcut: a key with Ctrl, Alt or Cmd (or a key F1 to F24)."
            ),
            HotkeyError::Taken(why) => tr!(
                "Ce raccourci est déjà pris par une autre app ou par le système ({why}) : choisis-en un autre.",
                "This shortcut is already taken by another app or by the system ({why}): choose another one."
            ),
            HotkeyError::Unsupported => tr!(
                "Ici, une app ne peut pas avoir de raccourci global : sous Wayland, ajoute dans les raccourcis de ton bureau la commande « xdg-open escouade://overseer ».",
                "Here an app cannot have a global shortcut: under Wayland, add the command “xdg-open escouade://overseer” to your desktop’s shortcuts."
            ),
        }
    }
}

/// Whether the shortcut works here, and why not, as Réglages › Overseer shows it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyState {
    /// The shortcut registered now, as the settings keep it.
    pub active: Option<String>,
    /// Why the one of the settings is not.
    pub error: Option<String>,
    /// No global shortcut here (Wayland): the settings say what to do instead.
    pub unsupported: bool,
}

/// The accelerator of `tauri-plugin-global-shortcut` for `spec`, a shortcut as the settings keep it.
pub fn accelerator(spec: &str) -> Result<String, HotkeyError> {
    let mut parts: Vec<&str> = spec.split('+').map(str::trim).collect();
    let code = parts
        .pop()
        .filter(|c| !c.is_empty())
        .ok_or(HotkeyError::Invalid)?;
    let fkey = code
        .strip_prefix('F')
        .and_then(|n| n.parse::<u8>().ok())
        .is_some_and(|n| (1..=24).contains(&n));
    let letter = code
        .strip_prefix("Key")
        .is_some_and(|k| k.len() == 1 && k.chars().all(|c| c.is_ascii_uppercase()));
    let digit = ["Digit", "Numpad"].iter().any(|p| {
        code.strip_prefix(p)
            .is_some_and(|k| k.len() == 1 && k.chars().all(|c| c.is_ascii_digit()))
    });
    const OTHERS: &[&str] = &[
        "Space",
        "Enter",
        "Backquote",
        "Minus",
        "Equal",
        "BracketLeft",
        "BracketRight",
        "Backslash",
        "Semicolon",
        "Quote",
        "Comma",
        "Period",
        "Slash",
    ];
    if !(fkey || letter || digit || OTHERS.contains(&code)) {
        return Err(HotkeyError::Invalid);
    }
    let mut mods = Vec::new();
    for p in &parts {
        let m = match *p {
            "Mod" => "CommandOrControl",
            "Ctrl" => "Control",
            "Meta" => "Super",
            "Alt" => "Alt",
            "Shift" => "Shift",
            _ => return Err(HotkeyError::Invalid),
        };
        if !mods.contains(&m) {
            mods.push(m);
        }
    }
    // Shift alone types a letter: a shortcut needs Ctrl, Alt or Cmd, but for the keys F1 to F24.
    if !fkey && !mods.iter().any(|m| *m != "Shift") {
        return Err(HotkeyError::Invalid);
    }
    mods.push(code);
    Ok(mods.join("+"))
}

/// Whether this session has no global shortcut for an app: Wayland.
pub fn wayland() -> bool {
    cfg!(target_os = "linux")
        && (std::env::var("XDG_SESSION_TYPE").is_ok_and(|v| v.eq_ignore_ascii_case("wayland"))
            || std::env::var_os("WAYLAND_DISPLAY").is_some())
}

/// The Overseer's shortcut: where it is registered, and what is now.
#[derive(Default)]
pub struct Hotkey {
    keys: OnceLock<Arc<dyn Keys>>,
    /// The shortcut registered (as the settings keep it) and its accelerator.
    current: Mutex<Option<(String, String)>>,
    error: Mutex<Option<String>>,
}

impl Hotkey {
    /// Where the shortcuts go: the plugin, set as the app starts (none: no global shortcut here).
    pub fn set_keys(&self, keys: Arc<dyn Keys>) {
        let _ = self.keys.set(keys);
    }

    /// Makes `spec` the shortcut (none: no shortcut). The new one is registered first: refused, the one before stays,
    /// and the reason is kept for the settings.
    pub fn apply(&self, spec: Option<&str>) -> Result<(), HotkeyError> {
        let spec = spec.map(str::trim).filter(|s| !s.is_empty());
        let r = self.change(spec);
        *self.error.lock() = r.as_ref().err().map(HotkeyError::text);
        r
    }

    fn change(&self, spec: Option<&str>) -> Result<(), HotkeyError> {
        let mut current = self.current.lock();
        if current.as_ref().map(|(s, _)| s.as_str()) == spec {
            return Ok(());
        }
        let Some(spec) = spec else {
            if let (Some((_, accel)), Some(keys)) = (current.take(), self.keys.get()) {
                keys.unregister(&accel);
            }
            return Ok(());
        };
        let accel = accelerator(spec)?;
        let keys = self.keys.get().ok_or(HotkeyError::Unsupported)?;
        keys.register(&accel).map_err(HotkeyError::Taken)?;
        if let Some((_, old)) = current.replace((spec.to_string(), accel)) {
            keys.unregister(&old);
        }
        Ok(())
    }

    /// What Réglages › Overseer shows.
    pub fn state(&self) -> HotkeyState {
        HotkeyState {
            active: self.current.lock().as_ref().map(|(s, _)| s.clone()),
            error: self.error.lock().clone(),
            unsupported: self.keys.get().is_none() && wayland(),
        }
    }
}

/// The keys of a sandbox (the end-to-end tests): recorded, never the system's; `overseer_hotkey_press` presses them.
#[derive(Default)]
pub struct Recorded(Mutex<Vec<String>>);

impl Keys for Recorded {
    fn register(&self, accel: &str) -> Result<(), String> {
        self.0.lock().push(accel.to_string());
        Ok(())
    }

    fn unregister(&self, accel: &str) {
        self.0.lock().retain(|a| a != accel);
    }
}

/// The shortcut was pressed: the Overseer's screen opens, the window brought forward (as by its link).
#[cfg(feature = "desktop")]
pub fn pressed<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    crate::deeplink::deliver(app, [OVERSEER_LINK.to_string()]);
}

/// The keys of `tauri-plugin-global-shortcut`, on the app's handle.
#[cfg(feature = "desktop")]
pub struct PluginKeys<R: tauri::Runtime>(pub tauri::AppHandle<R>);

#[cfg(feature = "desktop")]
impl<R: tauri::Runtime> Keys for PluginKeys<R> {
    fn register(&self, accel: &str) -> Result<(), String> {
        use tauri_plugin_global_shortcut::GlobalShortcutExt;
        self.0
            .global_shortcut()
            .register(accel)
            .map_err(|e| e.to_string())
    }

    fn unregister(&self, accel: &str) {
        use tauri_plugin_global_shortcut::GlobalShortcutExt;
        if let Err(e) = self.0.global_shortcut().unregister(accel) {
            log::warn!("overseer: shortcut {accel} not let go: {e}");
        }
    }
}
