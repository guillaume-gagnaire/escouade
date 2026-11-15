//! The Overseer's global shortcut (ESC-468): written for the plugin, registered, changed, refused when taken.

use crate::hotkey::*;
use parking_lot::Mutex;
use std::sync::Arc;

/// The system's shortcuts: those another app holds, and ours.
#[derive(Default)]
pub(crate) struct FakeKeys {
    pub(crate) taken: Vec<String>,
    pub(crate) held: Mutex<Vec<String>>,
}

impl Keys for FakeKeys {
    fn register(&self, accel: &str) -> Result<(), String> {
        if self.taken.iter().any(|t| t == accel) || self.held.lock().iter().any(|h| h == accel) {
            return Err("HotKey already registered".into());
        }
        self.held.lock().push(accel.to_string());
        Ok(())
    }

    fn unregister(&self, accel: &str) {
        self.held.lock().retain(|h| h != accel);
    }
}

fn with(taken: &[&str]) -> (Hotkey, Arc<FakeKeys>) {
    let keys = Arc::new(FakeKeys {
        taken: taken.iter().map(|s| s.to_string()).collect(),
        ..Default::default()
    });
    let h = Hotkey::default();
    h.set_keys(keys.clone());
    (h, keys)
}

#[test]
fn a_shortcut_is_written_as_the_plugin_reads_it() {
    assert_eq!(
        accelerator("Mod+Alt+KeyO").unwrap(),
        "CommandOrControl+Alt+KeyO"
    );
    assert_eq!(accelerator("Alt+Shift+KeyO").unwrap(), "Alt+Shift+KeyO");
    assert_eq!(
        accelerator("Ctrl+Meta+Space").unwrap(),
        "Control+Super+Space"
    );
    assert_eq!(
        accelerator("Mod+Digit1").unwrap(),
        "CommandOrControl+Digit1"
    );
    assert_eq!(accelerator("F13").unwrap(), "F13");
    assert_eq!(accelerator("Shift+F2").unwrap(), "Shift+F2");
    // A letter alone, with Shift alone, an unknown key or modifier: no shortcut.
    for bad in [
        "KeyO",
        "Shift+KeyO",
        "Mod+Alt+Escape",
        "Hyper+KeyO",
        "",
        "Mod+",
        "F25",
    ] {
        assert_eq!(accelerator(bad), Err(HotkeyError::Invalid), "{bad}");
    }
}

#[test]
fn the_default_shortcut_avoids_altgr_and_is_one() {
    let d = crate::model::default_overseer_shortcut();
    assert!(accelerator(&d).is_ok(), "{d}");
    if !cfg!(target_os = "macos") {
        // Ctrl+Alt is AltGr on many keyboards of Windows and Linux: it types letters.
        assert!(!(d.contains("Mod") && d.contains("Alt")), "{d}");
    }
}

#[test]
fn a_shortcut_is_registered_changed_and_let_go() {
    let (h, keys) = with(&[]);
    h.apply(Some("Alt+Shift+KeyO")).unwrap();
    assert_eq!(*keys.held.lock(), vec!["Alt+Shift+KeyO".to_string()]);
    assert_eq!(h.state().active.as_deref(), Some("Alt+Shift+KeyO"));
    // The same again: nothing done.
    h.apply(Some("Alt+Shift+KeyO")).unwrap();
    assert_eq!(keys.held.lock().len(), 1);
    // Changed: the new one held, the old one let go.
    h.apply(Some("Mod+Alt+KeyJ")).unwrap();
    assert_eq!(
        *keys.held.lock(),
        vec!["CommandOrControl+Alt+KeyJ".to_string()]
    );
    // None (the Overseer turned off, or the shortcut emptied): let go.
    h.apply(None).unwrap();
    assert!(keys.held.lock().is_empty());
    assert_eq!(h.state().active, None);
    h.apply(Some("  ")).unwrap();
    assert!(keys.held.lock().is_empty());
}

#[test]
fn a_shortcut_another_app_holds_is_refused_and_the_one_before_stays() {
    let (h, keys) = with(&["CommandOrControl+Shift+KeyK"]);
    h.apply(Some("Alt+Shift+KeyO")).unwrap();
    let e = h.apply(Some("Mod+Shift+KeyK")).unwrap_err();
    assert!(matches!(e, HotkeyError::Taken(_)), "{e:?}");
    assert!(
        e.text().contains("déjà pris") || e.text().contains("already taken"),
        "{}",
        e.text()
    );
    // The one before still works, and the settings say why the new one does not.
    assert_eq!(*keys.held.lock(), vec!["Alt+Shift+KeyO".to_string()]);
    let s = h.state();
    assert_eq!(s.active.as_deref(), Some("Alt+Shift+KeyO"));
    assert!(s.error.is_some());
    // An invalid one is refused before the system is asked.
    assert_eq!(h.apply(Some("KeyO")), Err(HotkeyError::Invalid));
    assert_eq!(keys.held.lock().len(), 1);
    // A good one clears the error.
    h.apply(Some("Mod+Alt+KeyO")).unwrap();
    assert_eq!(h.state().error, None);
}

#[test]
fn without_global_shortcuts_here_it_is_said() {
    let h = Hotkey::default();
    assert_eq!(
        h.apply(Some("Alt+Shift+KeyO")),
        Err(HotkeyError::Unsupported)
    );
    assert!(HotkeyError::Unsupported
        .text()
        .contains("escouade://overseer"));
    assert_eq!(h.state().active, None);
}
