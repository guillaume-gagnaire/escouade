//! The languages written in now, for the whole app: the interface's (errors, notifications, menus) and that of the texts
//! Claude is made to write. Read from anywhere without a lock; the app's `i18n` sets them from the settings (never in
//! its tests) and says the rest.

pub use escouade_plugin_cli::i18n::Lang;
use std::sync::atomic::{AtomicU8, Ordering};

// French until the settings are read: the app was French only up to 1.6.
static UI: AtomicU8 = AtomicU8::new(Lang::Fr.code());
static CLAUDE: AtomicU8 = AtomicU8::new(Lang::Fr.code());

/// The language of the interface: what the window, the menus and the notifications are written in.
pub fn ui() -> Lang {
    Lang::of_code(UI.load(Ordering::Relaxed))
}

/// The language of the texts Claude is made to write, and of what agents are told. The core reads
/// its own (`Core::lang().claude`, the same but in tests): this is for what has no core at hand.
pub fn claude() -> Lang {
    Lang::of_code(CLAUDE.load(Ordering::Relaxed))
}

/// Writes from now on in `ui` and `claude`: the app's `i18n::set`, from the settings.
pub fn store(ui: Lang, claude: Lang) {
    UI.store(ui.code(), Ordering::Relaxed);
    CLAUDE.store(claude.code(), Ordering::Relaxed);
}
