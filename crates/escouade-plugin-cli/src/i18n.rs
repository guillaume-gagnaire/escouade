//! The two languages of what Escouade writes, and how a text is written in both, French first (the app's `i18n` tells
//! the rest: `tr!`, the language of the interface). `escouade-plugin` writes in the language it is given
//! (`--lang fr|en`), else the system's.

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    Fr,
    En,
}

impl Lang {
    pub const fn code(self) -> u8 {
        match self {
            Lang::Fr => 0,
            Lang::En => 1,
        }
    }

    /// As a setting names it (`fr`, `en`): what a plugin is told.
    pub const fn tag(self) -> &'static str {
        match self {
            Lang::Fr => "fr",
            Lang::En => "en",
        }
    }

    pub const fn of_code(code: u8) -> Self {
        match code {
            1 => Lang::En,
            _ => Lang::Fr,
        }
    }

    /// The language a setting names ("fr", "en"), if it names one.
    pub fn named(value: &str) -> Option<Self> {
        match value {
            "fr" => Some(Lang::Fr),
            "en" => Some(Lang::En),
            _ => None,
        }
    }
}

/// The system's language, read once.
pub fn system() -> Lang {
    static SYSTEM: OnceLock<Lang> = OnceLock::new();
    *SYSTEM.get_or_init(|| of_locale(sys_locale::get_locale().as_deref().unwrap_or_default()))
}

/// The language of a locale tag (`fr-FR`, `fr_CA`, `en-US`, POSIX `fr_FR.UTF-8`): French when
/// its language subtag is, English otherwise.
pub fn of_locale(tag: &str) -> Lang {
    let language = tag.split(['-', '_', '.', '@']).next().unwrap_or_default();
    if language.eq_ignore_ascii_case("fr") {
        Lang::Fr
    } else {
        Lang::En
    }
}

/// A text in `lang`: `tr_in!(lang, "français", "English", args…)`.
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
