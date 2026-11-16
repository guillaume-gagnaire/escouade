//! The mode an agent's harness really runs it in (ESC-489). An agent's mode is what Escouade asks of its harness; the
//! harness runs in one of the modes it declares (`EngineCaps::modes`, a plugin's manifest for a harness of a plugin),
//! and that one alone is judged (« Autoriser le mode Bypass », SEC-09). The rule, the same on every path (a new agent,
//! a ticket, a start or a resume, a change of mode, of harness or of account):
//!
//! - a mode its harness has is the one it runs in;
//! - a harness that has one mode alone runs in that one, whatever was asked: it is the agent's mode, shown and judged
//!   (a harness whose one mode is Bypass does not start while Bypass is not allowed);
//! - a harness of several modes without the one asked runs in the closest of its own that never allows more, said in
//!   the conversation; when every one of them allows more, the agent does not start, and says why;
//! - a harness that declares no mode Escouade knows (a manifest of anyone's) is held for Bypass: nothing tells that it
//!   ever asks before it acts (`declared`).

use crate::bypass::BYPASS;
use crate::core::Core;
use crate::engine::registry::ModeCap;
use crate::engine::Engine;
use crate::i18n::Lang;
use anyhow::{bail, Result};
use std::sync::Arc;
use tauri::Runtime;

/// Escouade's modes, from the one that allows the least to the one that allows the most.
pub const ORDER: [&str; 5] = ["plan", "default", "acceptEdits", "auto", BYPASS];

/// Where `mode` stands in `ORDER`; None for a mode Escouade does not know.
pub fn rank(mode: &str) -> Option<usize> {
    ORDER.iter().position(|m| *m == mode)
}

/// The mode an agent in `current` runs in on a harness that offers `offered`: the same when it is there, else the
/// closest one that never allows more. None when every mode offered allows more. A mode Escouade does not know is taken
/// as « Demander » (`default`).
pub fn closest(current: &str, offered: &[&str]) -> Option<String> {
    if offered.contains(&current) {
        return Some(current.to_string());
    }
    let at = rank(current).unwrap_or(1);
    offered
        .iter()
        .filter_map(|m| rank(m).map(|r| (r, *m)))
        .filter(|(r, _)| *r <= at)
        .max_by_key(|(r, _)| *r)
        .map(|(_, m)| m.to_string())
}

/// The mode a harness that offers `offered` runs an agent asked to be in `asked` in (the rule above). None: none of its
/// modes allows as little, the agent does not start. No mode offered is Bypass.
pub fn runs_in(asked: &str, offered: &[&str]) -> Option<String> {
    match offered {
        [] => Some(BYPASS.to_string()),
        [one] => Some(one.to_string()),
        _ => closest(asked, offered),
    }
}

/// The modes `engine` runs in, in its menu's order.
pub fn offered(engine: &dyn Engine) -> Vec<&'static str> {
    engine.caps().modes.iter().map(|m| m.id).collect()
}

/// What a harness's manifest declares of its modes, as Escouade takes them: those it knows, in their order; a mode of
/// another name is left out (no menu offers it, nothing would judge it). None left: the harness is held for Bypass,
/// with what that means (`undeclared`).
pub fn declared(
    name: &str,
    modes: impl IntoIterator<Item = (&'static str, Option<String>)>,
    lang: Lang,
) -> Vec<ModeCap> {
    let known: Vec<ModeCap> = modes
        .into_iter()
        .filter(|(id, _)| rank(id).is_some())
        .map(|(id, meaning)| ModeCap { id, meaning })
        .collect();
    if !known.is_empty() {
        return known;
    }
    vec![ModeCap {
        id: BYPASS,
        meaning: Some(undeclared(lang, name)),
    }]
}

/// What the only mode of a harness that declares none says.
pub fn undeclared(lang: Lang, name: &str) -> String {
    tr_in!(
        lang,
        "{name} ne déclare aucun mode : rien ne dit qu’il demande avant d’agir, Escouade le tient pour Bypass.",
        "{name} declares no mode: nothing says it asks before it acts, so Escouade holds it as Bypass.",
        name = name
    )
}

/// The name of `mode` as the menu says it.
pub fn label(lang: Lang, mode: &str) -> String {
    match (lang, mode) {
        (_, "auto") => "Auto".into(),
        (Lang::Fr, "default") => "Demander".into(),
        (Lang::En, "default") => "Ask".into(),
        (_, "plan") => "Plan".into(),
        (Lang::Fr, "acceptEdits") => "Édits auto".into(),
        (Lang::En, "acceptEdits") => "Accept edits".into(),
        (_, BYPASS) => "Bypass".into(),
        (_, other) => other.into(),
    }
}

/// Why an agent asked to be in `asked` does not start on `engine` (named `name`): none of its modes allows as little.
pub fn none_as_careful(lang: Lang, name: &str, asked: &str) -> String {
    tr_in!(
        lang,
        "{name} n’a aucun mode qui autorise aussi peu que « {mode} » : choisis un de ses modes, ou un autre harness.",
        "{name} has no mode that allows as little as “{mode}”: choose one of its modes, or another harness.",
        name = name,
        mode = label(lang, asked)
    )
}

/// Why an agent of a harness whose one mode is Bypass (`declares`: it declares none) does not start without
/// « Autoriser le mode Bypass ».
pub fn only_bypass(lang: Lang, name: &str, declares: bool) -> String {
    if declares {
        tr_in!(
            lang,
            "{name} ne tourne qu’en Bypass (il ne demande jamais avant d’agir) : active « Autoriser le mode Bypass » dans Réglages › Agents, ou choisis un autre harness.",
            "{name} only runs in Bypass (it never asks before it acts): turn on “Allow Bypass mode” in Settings › Agents, or choose another harness.",
            name = name
        )
    } else {
        tr_in!(
            lang,
            "{name} ne déclare aucun mode : Escouade le tient pour Bypass. Active « Autoriser le mode Bypass » dans Réglages › Agents, ou choisis un autre harness.",
            "{name} declares no mode, so Escouade holds it as Bypass. Turn on “Allow Bypass mode” in Settings › Agents, or choose another harness.",
            name = name
        )
    }
}

/// What the conversation says of an agent asked to be in `asked` that its harness runs in `mode`.
pub fn settled(lang: Lang, name: &str, asked: &str, mode: &str, alone: bool) -> String {
    if alone {
        tr_in!(
            lang,
            "{name} n’a que le mode « {mode} » : l’agent tourne dans ce mode, pas en « {asked} ».",
            "{name} only has the “{mode}” mode: the agent runs in it, not in “{asked}”.",
            name = name,
            mode = label(lang, mode),
            asked = label(lang, asked)
        )
    } else {
        tr_in!(
            lang,
            "{name} n’a pas le mode « {asked} » : l’agent tourne dans le plus proche qui n’autorise pas plus, « {mode} ».",
            "{name} has no “{asked}” mode: the agent runs in the closest one that allows no more, “{mode}”.",
            name = name,
            mode = label(lang, mode),
            asked = label(lang, asked)
        )
    }
}

/// The mode `engine` runs an agent asked to be in `asked` in, judged: refused when none of its modes allows as little,
/// or when that mode is Bypass and Bypass is not `allowed`, with why and what to do.
pub fn judge(engine: &dyn Engine, asked: &str, allowed: bool, lang: Lang) -> Result<String> {
    let Some(mode) = runs_in(asked, &offered(engine)) else {
        bail!(none_as_careful(lang, engine.name(), asked));
    };
    check(engine, &mode, allowed, lang)?;
    Ok(mode)
}

/// `mode`, one of `engine`'s, refused when it is Bypass and Bypass is not `allowed`: for a harness that has no other,
/// the refusal says so, and that another harness is the way out.
pub fn check(engine: &dyn Engine, mode: &str, allowed: bool, lang: Lang) -> Result<()> {
    if mode == BYPASS && !allowed && offered(engine).len() <= 1 {
        bail!(only_bypass(lang, engine.name(), engine.declares_modes()));
    }
    crate::bypass::check(mode, allowed)
}

/// Why an agent of `name` is not put in `mode`, a mode its harness does not have.
pub fn missing(lang: Lang, name: &str, mode: &str) -> String {
    tr_in!(
        lang,
        "{name} n’a pas le mode « {mode} » : choisis un de ses modes.",
        "{name} has no “{mode}” mode: choose one of its modes.",
        name = name,
        mode = label(lang, mode)
    )
}

/// Why a discussion of the Docs tab (ESC-261) does not run on a harness that has neither « Demander » nor « Plan ».
pub fn no_discussion(lang: Lang, name: &str) -> String {
    tr_in!(
        lang,
        "{name} n’a ni « Demander » ni « Plan » : une discussion, qui ne modifie jamais le code, ne tourne pas dessus.",
        "{name} has neither “Ask” nor “Plan”: a discussion, which never changes the code, does not run on it.",
        name = name
    )
}

impl<R: Runtime> Core<R> {
    /// The agent `id` in the mode its harness runs it in (`runs_in`), judged: what every start goes through, and a
    /// change of harness. When its harness runs it in another mode than its own, that one becomes its mode (shown on
    /// it) and its conversation says so; refused, with why and what to do, when no mode of its harness allows as
    /// little, when it is a discussion its harness has no mode for, or when that mode is Bypass, not allowed.
    pub(crate) fn settle_mode(self: &Arc<Self>, id: &str) -> Result<()> {
        let lang = crate::i18n::ui();
        let allowed = self.bypass_allowed();
        // Its mode shown as its harness runs it, refused or not.
        let (engine, mode) = self.with_agent(id, |rt, fx| {
            let engine = crate::engine::of(&rt.meta);
            let offered = offered(engine);
            let asked = rt.meta.mode.clone();
            let Some(mode) = runs_in(&asked, &offered) else {
                bail!(none_as_careful(lang, engine.name(), &asked));
            };
            if mode != asked {
                if rt.meta.discussion.is_some()
                    && !crate::discussion::MODES.contains(&mode.as_str())
                {
                    bail!(no_discussion(lang, engine.name()));
                }
                log::info!(
                    "agent {id}: {} runs it in {mode}, not in {asked}",
                    engine.name()
                );
                rt.meta.set_mode(&mode);
                let text = settled(lang, engine.name(), &asked, &mode, offered.len() == 1);
                rt.notice("info", text, fx);
            }
            Ok((engine, mode))
        })?;
        check(engine, &mode, allowed, lang)
    }
}

#[cfg(test)]
#[path = "modes_tests.rs"]
pub(crate) mod tests;
