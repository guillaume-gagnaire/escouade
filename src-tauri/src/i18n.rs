//! The languages of what the backend writes: the interface's (errors, notifications, menus) and
//! that of the texts Claude is made to write (commit messages, comments, instructions to agents).
//! Both are read from anywhere without a lock: `ui()`, `claude()`.
//!
//! A text is written in both languages where it is used, French first:
//!
//! ```text
//! tr!("Afficher", "Show")                                   // the interface's language
//! tr!("{name} est introuvable", "{name} not found", name = path.display())
//! tr_claude!("Corrige les tests.", "Fix the tests.")        // the texts for Claude
//! tr_in!(Lang::En, "{n} agents en attente", "{n} agents waiting", n = n)
//! // A count: French singular for 0 and 1, English for 1 only (`is_one`).
//! tr_n!(n, "{n} fichier", "{n} fichiers", "{n} file", "{n} files", n = n)
//! ```
//!
//! Each gives a `String`, its arguments as `format!` takes them (named, positional or captured
//! from the scope). Both formats get the same arguments: a named one that either text leaves out
//! does not compile (« named argument never used »). Tests run side by side in one process and
//! the language is global: a test of an English text passes its language (`tr_in!`, or a
//! function that takes a `Lang`), never `set`.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    Fr,
    En,
}

impl Lang {
    const fn code(self) -> u8 {
        match self {
            Lang::Fr => 0,
            Lang::En => 1,
        }
    }

    const fn of_code(code: u8) -> Self {
        match code {
            1 => Lang::En,
            _ => Lang::Fr,
        }
    }

    /// The language a setting names ("fr", "en"), if it names one.
    fn named(value: &str) -> Option<Self> {
        match value {
            "fr" => Some(Lang::Fr),
            "en" => Some(Lang::En),
            _ => None,
        }
    }
}

/// The languages as the window is told them: the interface's, the system's (the option
/// « Système (…) » names it) and that of the texts Claude writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LangInfo {
    pub ui: Lang,
    pub system: Lang,
    pub claude: Lang,
}

// French until the settings are read: the app was French only up to 1.6.
static UI: AtomicU8 = AtomicU8::new(Lang::Fr.code());
static CLAUDE: AtomicU8 = AtomicU8::new(Lang::Fr.code());

/// The language of the interface: what the window, the menus and the notifications are written in.
pub fn ui() -> Lang {
    Lang::of_code(UI.load(Ordering::Relaxed))
}

/// The language of the texts Claude is made to write, and of what agents are told.
// Allowed unused until the texts for Claude go through `tr_claude!` (then drop the allow).
#[allow(dead_code)]
pub fn claude() -> Lang {
    Lang::of_code(CLAUDE.load(Ordering::Relaxed))
}

/// The system's language: French when its first preferred locale is French, English otherwise.
/// Read once per run: a change of the system's language while the app runs (rare, and it often
/// takes a new session) would otherwise make what the window was told and what a later save
/// compares disagree. French in tests, whatever the machine's.
pub fn system() -> Lang {
    if cfg!(test) {
        return Lang::Fr;
    }
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

/// The languages the settings stand for, the system's being `system`: « Langue de l'interface »
/// (`language`: "system", "fr", "en") and « Langue des textes rédigés par Claude »
/// (`claude_language`: "ui", "fr", "en"). A value this version does not know (saved by a later
/// one) is the default: the system's, the interface's.
pub fn resolve_with(language: &str, claude_language: &str, system: Lang) -> LangInfo {
    let ui = Lang::named(language).unwrap_or(system);
    LangInfo {
        ui,
        system,
        claude: Lang::named(claude_language).unwrap_or(ui),
    }
}

/// The languages the settings stand for, on this system.
pub fn resolve(language: &str, claude_language: &str) -> LangInfo {
    resolve_with(language, claude_language, system())
}

/// Writes from now on in the languages of `info`. Not in tests: the others running beside would
/// switch with it (their texts are French).
pub fn set(info: LangInfo) {
    if cfg!(test) {
        return;
    }
    UI.store(info.ui.code(), Ordering::Relaxed);
    CLAUDE.store(info.claude.code(), Ordering::Relaxed);
}

/// Writes from now on in the languages the settings stand for, and returns them.
pub fn configure(language: &str, claude_language: &str) -> LangInfo {
    let info = resolve(language, claude_language);
    set(info);
    info
}

/// A text in `lang`: `tr_in!(lang, "français", "English", args…)`.
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

/// Whether `count` takes the singular form (CLDR's « one ») in `lang`, as the window's
/// `Intl.PluralRules` says: 0 and 1 in French, only 1 in English.
pub fn is_one(lang: Lang, count: i128) -> bool {
    match lang {
        Lang::Fr => count.unsigned_abs() <= 1,
        Lang::En => count.unsigned_abs() == 1,
    }
}

/// A text that depends on a count, in `lang`: `tr_n_in!(lang, count, "fr singulier",
/// "fr pluriel", "en singular", "en plural", args…)`. `count` (any integer) picks the form by the
/// rule of the language (`is_one`); it is not an argument of the texts by itself: pass it
/// (`n = count`) or capture it. Every form writes every argument, « {n} file » and not
/// « one file » (`format!` refuses one left out).
macro_rules! tr_n_in {
    ($lang:expr, $count:expr, $fr_one:literal, $fr_other:literal, $en_one:literal, $en_other:literal $(, $($args:tt)+)?) => {{
        let lang = $lang;
        match (lang, $crate::i18n::is_one(lang, ($count) as i128)) {
            ($crate::i18n::Lang::Fr, true) => format!($fr_one $(, $($args)+)?),
            ($crate::i18n::Lang::Fr, false) => format!($fr_other $(, $($args)+)?),
            ($crate::i18n::Lang::En, true) => format!($en_one $(, $($args)+)?),
            ($crate::i18n::Lang::En, false) => format!($en_other $(, $($args)+)?),
        }
    }};
}

/// A text that depends on a count, in the interface's language: `tr_n!(count, "fr singulier",
/// "fr pluriel", "en singular", "en plural", args…)`.
// Allowed unused until the backend's texts go through it (then drop the allow).
#[allow(unused_macros)]
macro_rules! tr_n {
    ($($t:tt)+) => {
        tr_n_in!($crate::i18n::ui(), $($t)+)
    };
}

/// A text in the interface's language: `tr!("français", "English", args…)`.
// Allowed unused until the backend's texts go through it, module by module (then drop the allow).
#[allow(unused_macros)]
macro_rules! tr {
    ($($t:tt)+) => {
        tr_in!($crate::i18n::ui(), $($t)+)
    };
}

/// A text in the language of the texts Claude writes: `tr_claude!("français", "English", args…)`.
// Allowed unused until the texts for Claude go through it (then drop the allow).
#[allow(unused_macros)]
macro_rules! tr_claude {
    ($($t:tt)+) => {
        tr_in!($crate::i18n::claude(), $($t)+)
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use Lang::{En, Fr};

    #[test]
    fn the_settings_give_the_interface_and_claude_their_language_whatever_the_system() {
        // Each choice of « Langue de l'interface » × « Langue des textes rédigés par Claude »,
        // on a French system and on another.
        let cases = [
            // (language, claude_language, system) → (ui, claude)
            ("system", "ui", Fr, Fr, Fr),
            ("system", "ui", En, En, En),
            ("system", "fr", En, En, Fr),
            ("system", "en", Fr, Fr, En),
            ("fr", "ui", En, Fr, Fr),
            ("fr", "fr", En, Fr, Fr),
            ("fr", "en", Fr, Fr, En),
            ("en", "ui", Fr, En, En),
            ("en", "fr", Fr, En, Fr),
            ("en", "en", Fr, En, En),
        ];
        for (language, claude_language, system, ui, claude) in cases {
            assert_eq!(
                resolve_with(language, claude_language, system),
                LangInfo { ui, system, claude },
                "{language} × {claude_language} on a {system:?} system"
            );
        }
    }

    #[test]
    fn a_language_saved_by_a_later_version_is_the_systems_and_the_interfaces() {
        // A value this version does not know (a language added later, the app rolled back).
        assert_eq!(resolve_with("de", "ui", En).ui, En);
        assert_eq!(
            resolve_with("", "", Fr),
            LangInfo {
                ui: Fr,
                system: Fr,
                claude: Fr
            }
        );
        assert_eq!(resolve_with("en", "de", Fr).claude, En);
    }

    #[test]
    fn the_system_is_french_when_its_locale_is() {
        for tag in ["fr-FR", "fr", "FR", "fr_CA", "fr-CH", "fr_FR.UTF-8"] {
            assert_eq!(of_locale(tag), Fr, "{tag}");
        }
        // Northern Frisian starts with « fr » too: only the language's own subtag counts.
        for tag in ["en-US", "en", "de-DE", "", "frr", "C"] {
            assert_eq!(of_locale(tag), En, "{tag:?}");
        }
    }

    #[test]
    fn tests_never_switch_the_language_of_the_others() {
        // Tests run side by side in one process: the language stays French for all of them, and
        // the system reads as French whatever the machine's.
        assert_eq!(system(), Fr);
        let info = configure("en", "en");
        assert_eq!(
            info,
            LangInfo {
                ui: En,
                system: Fr,
                claude: En
            }
        );
        assert_eq!((ui(), claude()), (Fr, Fr));
        assert_eq!(tr!("Afficher", "Show"), "Afficher");
        assert_eq!(
            tr_claude!("Corrige les tests.", "Fix the tests."),
            "Corrige les tests."
        );
    }

    #[test]
    fn a_text_is_written_in_the_language_asked_for_with_its_arguments() {
        let n = 3;
        assert_eq!(tr_in!(Fr, "Texte", "Text"), "Texte");
        assert_eq!(tr_in!(En, "Texte", "Text"), "Text");
        assert_eq!(
            tr_in!(Fr, "{n} agents en attente", "{n} agents waiting", n = n),
            "3 agents en attente"
        );
        assert_eq!(
            tr_in!(En, "{n} agents en attente", "{n} agents waiting", n = n),
            "3 agents waiting"
        );
        // Captured from the scope, and in another order in each language.
        let name = "app.ts";
        let dir = "src";
        assert_eq!(
            tr_in!(En, "{name} est dans {dir}", "{dir} holds {name}"),
            "src holds app.ts"
        );
        assert_eq!(
            tr_in!(Fr, "{name} est dans {dir}", "{dir} holds {name}"),
            "app.ts est dans src"
        );
        // The language is an expression, evaluated once.
        let langs = [En];
        let mut i = 0;
        let text = tr_in!(
            {
                i += 1;
                langs[i - 1]
            },
            "{} fichier",
            "{} file",
            1
        );
        assert_eq!((text.as_str(), i), ("1 file", 1));
    }

    #[test]
    fn a_language_is_kept_as_its_code_and_read_back() {
        // What `set` stores and `ui()` / `claude()` read (`set` itself does nothing in tests).
        for lang in [Fr, En] {
            assert_eq!(Lang::of_code(lang.code()), lang);
        }
        assert_eq!(Lang::of_code(7), Fr);
    }

    #[test]
    fn french_puts_0_and_1_in_the_singular_english_only_1() {
        let one = |lang| [0, 1, 2, 5, -1].map(|n| is_one(lang, n));
        assert_eq!(one(Fr), [true, true, false, false, true]);
        assert_eq!(one(En), [false, true, false, false, true]);
    }

    #[test]
    fn a_count_picks_the_form_of_its_language() {
        let files = |lang, n: usize| {
            tr_n_in!(
                lang,
                n,
                "{n} fichier commité",
                "{n} fichiers commités",
                "{n} file committed",
                "{n} files committed",
                n = n
            )
        };
        assert_eq!(
            [0, 1, 2].map(|n| files(Fr, n)),
            [
                "0 fichier commité",
                "1 fichier commité",
                "2 fichiers commités"
            ]
        );
        assert_eq!(
            [0, 1, 2].map(|n| files(En, n)),
            ["0 files committed", "1 file committed", "2 files committed"]
        );
        // The count captured from the scope, another argument besides; the count an expression of
        // any integer type, evaluated once.
        let branch = "main";
        let mut calls = 0;
        let mut count = || {
            calls += 1;
            1u64
        };
        assert_eq!(
            tr_n_in!(
                En,
                count(),
                "{n} commit à tirer sur {branch}",
                "{n} commits à tirer sur {branch}",
                "{n} commit to pull on {branch}",
                "{n} commits to pull on {branch}",
                n = 1
            ),
            "1 commit to pull on main"
        );
        assert_eq!(calls, 1);
        let n = 3;
        assert_eq!(
            tr_n_in!(Fr, n, "{n} agent", "{n} agents", "{n} agent", "{n} agents"),
            "3 agents"
        );
    }
}
