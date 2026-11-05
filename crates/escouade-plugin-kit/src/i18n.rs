//! The language of the interface, in which a plugin of the first hand writes what the user reads (an error, a state's
//! name): French or English, as Escouade's. A plugin is told it by `initialize` (a module has no environment,
//! docs/PLUGINS.md §8.4.8); a process reads it from `ESCOUADE_LANG` (`fr`, `en`) before that. The core sets it as its
//! own changes (`set_ui`).

use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Fr,
    En,
}

impl Lang {
    pub fn code(self) -> &'static str {
        match self {
            Lang::Fr => "fr",
            Lang::En => "en",
        }
    }

    /// `en` is English; anything else French, as Escouade's default.
    pub fn of_code(code: &str) -> Lang {
        if code.trim().eq_ignore_ascii_case("en") {
            Lang::En
        } else {
            Lang::Fr
        }
    }
}

static UI: AtomicU8 = AtomicU8::new(0);

/// The interface's language.
pub fn ui() -> Lang {
    match UI.load(Ordering::Relaxed) {
        1 => Lang::En,
        _ => Lang::Fr,
    }
}

pub fn set_ui(lang: Lang) {
    UI.store(u8::from(lang == Lang::En), Ordering::Relaxed);
}

/// The language Escouade gave the plugin's process (`ESCOUADE_LANG`), when it gave one.
pub fn from_env() {
    if let Ok(code) = std::env::var("ESCOUADE_LANG") {
        set_ui(Lang::of_code(&code));
    }
}

/// The language `initialize` said, when it said one.
pub fn from_greeting() {
    let said = escouade_plugin_protocol::greeting().and_then(|g| g.lang.as_deref());
    if let Some(code) = said {
        set_ui(Lang::of_code(code));
    }
}

/// A text in `lang`: `tr_in!(lang, "français {x}", "English {x}")`.
#[macro_export]
macro_rules! tr_in {
    ($lang:expr, $fr:literal, $en:literal $(,)?) => {
        match $lang {
            $crate::i18n::Lang::Fr => format!($fr),
            $crate::i18n::Lang::En => format!($en),
        }
    };
    ($lang:expr, $fr:literal, $en:literal, $($args:tt)+) => {
        match $lang {
            $crate::i18n::Lang::Fr => format!($fr, $($args)+),
            $crate::i18n::Lang::En => format!($en, $($args)+),
        }
    };
}

/// A text in the interface's language.
#[macro_export]
macro_rules! tr {
    ($fr:literal, $en:literal $(, $($args:tt)+)?) => {
        $crate::tr_in!($crate::i18n::ui(), $fr, $en $(, $($args)+)?)
    };
}
