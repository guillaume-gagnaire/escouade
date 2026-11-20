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

/// Escouade's modes, from the one that allows the least to the one that allows the most: the list of the policies
/// (`permissions.modes`, `locks::MODES`) is this one, and a plugin's declaration is checked against an equal one
/// (`escouade_plugins::described::MODES`, a test holds them equal).
pub const ORDER: &[&str] = escouade_policy::list::MODES;

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
/// or when that mode is Bypass and Bypass is not `allowed`, with why and what to do. Every path of the app judges by
/// the policies too (`judge_in`, `Core::judge_mode`): this one, without them, is the rule alone, as the tests read it.
#[cfg(test)]
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

/// Why an agent asked to be in `asked`, that `engine` runs in `mode`, does not go in it: the policies of the device or
/// of its project's team do not allow that mode (`why`, their refusal, TS-11). The mode its harness runs it in is the
/// one judged (ESC-497); when it is not the one asked, the refusal says so.
pub fn denied(lang: Lang, engine: &dyn Engine, asked: &str, mode: &str, why: &str) -> String {
    let name = engine.name();
    if mode == asked {
        return why.to_string();
    }
    if offered(engine).len() <= 1 {
        tr_in!(
            lang,
            "{name} ne tourne qu’en « {mode} » : {why} Choisis un autre harness.",
            "{name} only runs in “{mode}”: {why} Choose another harness.",
            name = name,
            mode = label(lang, mode),
            why = why
        )
    } else {
        tr_in!(
            lang,
            "{name} n’a pas le mode « {asked} » et tournerait en « {mode} » : {why} Choisis un autre mode ou un autre harness.",
            "{name} has no “{asked}” mode and would run in “{mode}”: {why} Choose another mode or another harness.",
            name = name,
            mode = label(lang, mode),
            asked = label(lang, asked),
            why = why
        )
    }
}

/// `mode`, the one `engine` runs an agent asked to be in `asked` in, judged by the policies `locks` (the device's, or
/// its project's team's: `Core::locks_for`), refused with `denied`.
pub fn policies(
    engine: &dyn Engine,
    asked: &str,
    mode: &str,
    locks: &crate::locks::Locks,
    lang: Lang,
) -> Result<()> {
    match locks.check_mode(mode) {
        Ok(()) => Ok(()),
        Err(e) => bail!(denied(lang, engine, asked, mode, &e.to_string())),
    }
}

/// `check`, and the policies before it (`policies`): what every path judges the mode its harness runs an agent in by.
pub fn check_in(
    engine: &dyn Engine,
    asked: &str,
    mode: &str,
    allowed: bool,
    locks: &crate::locks::Locks,
    lang: Lang,
) -> Result<()> {
    policies(engine, asked, mode, locks, lang)?;
    check(engine, mode, allowed, lang)
}

/// `judge`, by the policies too (`check_in`): the mode `engine` runs an agent asked to be in `asked` in, refused when
/// none of its modes allows as little, when the policies do not allow it, or when it is Bypass and Bypass is not
/// `allowed`.
pub fn judge_in(
    engine: &dyn Engine,
    asked: &str,
    allowed: bool,
    locks: &crate::locks::Locks,
    lang: Lang,
) -> Result<String> {
    let Some(mode) = runs_in(asked, &offered(engine)) else {
        bail!(none_as_careful(lang, engine.name(), asked));
    };
    check_in(engine, asked, &mode, allowed, locks, lang)?;
    Ok(mode)
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

/// Why the agent of `meta`, as it runs now (its harness, the mode it runs in), may not run on: its harness or that mode
/// not allowed by `locks` (its project's, `Core::locks_for`), or that mode Bypass while « Autoriser le mode Bypass »
/// (`bypass_setting`, the app's) or the policies do not allow it (ESC-497). None when it may run on.
pub fn refusal_of(
    meta: &crate::model::AgentMeta,
    locks: &crate::locks::Locks,
    bypass_setting: bool,
    lang: Lang,
) -> Option<String> {
    if let Err(e) = locks.check_engine(meta.engine.as_str()) {
        return Some(e.to_string());
    }
    let engine = crate::engine::of(meta);
    let allowed = bypass_setting && locks.check_mode(BYPASS).is_ok();
    check_in(engine, &meta.mode, &meta.mode, allowed, locks, lang)
        .err()
        .map(|e| e.to_string())
}

/// What the conversation of an agent stopped because `why` says.
pub fn stopped(lang: Lang, why: &str) -> String {
    tr_in!(
        lang,
        "{why} L’agent s’arrête.",
        "{why} The agent stops.",
        why = why
    )
}

impl Core {
    /// Why the agent `id`, as it runs now, may not run on (`refusal_of`): what the hot changes of the policies and of
    /// « Autoriser le mode Bypass » look at, and what a start does not take a live process over with.
    pub(crate) fn run_refusal(&self, id: &str) -> Option<String> {
        let h = self.agent(id).ok()?;
        let project_id = h.lock().meta.project_id.clone();
        // Taken apart: never under the agent's lock.
        let locks = self.locks_for(&project_id);
        let bypass = self.bypass_allowed();
        let meta = h.lock().meta.clone();
        refusal_of(&meta, &locks, bypass, crate::i18n::ui())
    }

    /// What runs of the agent `id` — its process, its interface (the terminal view, the one kept up for Remote Control)
    /// — stopped, its conversation saying `text`: it may not run on (ESC-497). An agent at work is left to end its turn
    /// (as « Un agent dans un mode devenu interdit finit son tour » says), and judged again then (`settle_locked_mode`
    /// at `turn_ended`). True when something stopped.
    pub(crate) fn stop_out_of_rule(self: &Arc<Self>, id: &str, text: &str) -> bool {
        let Ok(h) = self.agent(id) else { return false };
        let (proc, term) = {
            let mut rt = h.lock();
            if rt.meta.status.is_active() {
                return false;
            }
            let proc = rt.detach();
            let term = rt
                .tui
                .as_ref()
                .map(|run| (run.term_id.clone(), crate::tui::quit_key(&rt.meta)));
            if term.is_some() {
                rt.gen += 1;
                rt.stop_tui(&mut crate::agent::Effects::default());
            }
            (proc, term)
        };
        if proc.is_none() && term.is_none() {
            return false;
        }
        log::info!("agent {id}: stopped, it may not run on: {text}");
        if let Some(p) = proc {
            p.close_input();
        }
        if let Some((term, key)) = term {
            self.hooks.revoke(id);
            self.quit_soon(id, &term, key);
        }
        self.emit_agent(&h);
        self.request_save();
        let text = text.to_string();
        let _ = self.with_agent(id, |rt, fx| {
            rt.notice("warn", text, fx);
            Ok(())
        });
        true
    }

    /// What every start of the agent `id` goes through, process or interface: its harness allowed by the policies of
    /// its project (TS-11), and the mode its harness runs it in judged (`settle_mode`). Refused, what still runs of it
    /// (a process started ahead, an interface kept up) stops, said with the same reason (ESC-497).
    pub(crate) fn judge_start(self: &Arc<Self>, id: &str) -> Result<()> {
        let (engine, project_id) = {
            let h = self.agent(id)?;
            let rt = h.lock();
            (rt.meta.engine, rt.meta.project_id.clone())
        };
        let judged = self
            .locks_for(&project_id)
            .check_engine(engine.as_str())
            .and_then(|()| self.settle_mode(id));
        if let Err(e) = &judged {
            self.stop_out_of_rule(id, &stopped(crate::i18n::ui(), &e.to_string()));
        }
        judged
    }

    /// The mode `engine` runs an agent of the project `project_id` asked to be in `asked` in, judged by « Autoriser le
    /// mode Bypass » and by the policies of the device and of the project's team (`judge_in`): what the Kanban and the
    /// MCP server judge before anything is made.
    pub(crate) fn judge_mode(
        &self,
        project_id: &str,
        engine: &dyn Engine,
        asked: &str,
    ) -> Result<String> {
        judge_in(
            engine,
            asked,
            self.bypass_allowed_in(project_id),
            &self.locks_for(project_id),
            crate::i18n::ui(),
        )
    }

    /// The agent `id` in the mode its harness runs it in (`runs_in`), judged: what every start goes through, and a
    /// change of harness. When its harness runs it in another mode than its own, that one becomes its mode (shown on
    /// it) and its conversation says so; refused, with why and what to do, when no mode of its harness allows as
    /// little, when it is a discussion its harness has no mode for, when the policies of the device or of its
    /// project's team do not allow that mode (ESC-497), or when that mode is Bypass, not allowed.
    pub(crate) fn settle_mode(self: &Arc<Self>, id: &str) -> Result<()> {
        let lang = crate::i18n::ui();
        // The policies of its project, taken apart: never under the agent's lock.
        let project_id = self.agent(id)?.lock().meta.project_id.clone();
        let locks = self.locks_for(&project_id);
        let allowed = self.bypass_allowed_in(&project_id);
        // Its harness runs it in its own mode: nothing changes, so the window is told nothing and nothing is saved (a
        // warm-up that comes after the window read the agent sends it nothing more, ESC-521).
        let same = {
            let h = self.agent(id)?;
            let rt = h.lock();
            let engine = crate::engine::of(&rt.meta);
            runs_in(&rt.meta.mode, &offered(engine))
                .filter(|m| *m == rt.meta.mode)
                .map(|m| (engine, m))
        };
        if let Some((engine, mode)) = same {
            return check_in(engine, &mode, &mode, allowed, &locks, lang);
        }
        // Its mode shown as its harness runs it, refused or not.
        let (engine, asked, mode) = self.with_agent(id, |rt, fx| {
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
            Ok((engine, asked, mode))
        })?;
        check_in(engine, &asked, &mode, allowed, &locks, lang)
    }
}

#[cfg(test)]
#[path = "modes_tests.rs"]
pub(crate) mod tests;

#[cfg(test)]
#[path = "modes_policies_tests.rs"]
mod policies_tests;
