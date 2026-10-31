//! « Changer de compte à » (ESC-148): an agent whose account has a short quota window at the
//! threshold, or a long one at the threshold whose pace is not good (PR #20, `pace`), goes on, at the
//! end of its turn, on another usable account of its harness (its session with it, as « Reprendre
//! sur <compte> » moves it), never in the middle of a turn, nor while a question waits for the user
//! or something runs in the background in its process.

use crate::accounts;
use crate::agent::{AgentAlert, NotifyKind};
use crate::board;
use crate::core::Core;
use crate::i18n::{self, Lang};
use crate::model::*;
use crate::pace::{self, MeterVerdict};
use crate::usage::window_name;
use std::sync::Arc;
use tauri::Runtime;

/// Why an agent past the threshold has no other account to go on on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NoOther {
    /// None of its harness serving its project is usable (under the threshold, signed in, held
    /// back by nothing).
    Nobody,
    /// Its project is private and confined (ESC-141): the usable ones are accounts it shares.
    Private,
    /// Its ticket is set on its account (« Compte », ESC-153): it stays there.
    Pinned,
}

/// The meter as a sentence says what of it is used: « sa fenêtre de 5 h », « its weekly window »,
/// « ses crédits du mois ».
pub(crate) fn meter_phrase(lang: Lang, meter: &QuotaMeter) -> String {
    match meter.minutes {
        Some(m) => {
            let window = window_name(lang, m);
            tr_in!(lang, "sa fenêtre {window}", "its {window} window")
        }
        None => tr_in!(lang, "ses crédits du mois", "its month’s credits"),
    }
}

/// What follows the meter's use in a sentence when it is the pace that puts it behind: where it
/// would end at its reset at the pace so far (nothing when the use alone says it).
fn pace_note(lang: Lang, meter: &MeterVerdict) -> String {
    match meter.projected.filter(|_| meter.by_pace()) {
        Some(p) => {
            let pct = pace::written(p);
            tr_in!(
                lang,
                " (au rythme actuel, {pct} % à sa remise à zéro)",
                " (at the current pace, {pct}% at its reset)"
            )
        }
        None => String::new(),
    }
}

/// What the conversation says when the agent goes on on `to`, leaving `from` at `meter`.
pub(crate) fn switched_line(lang: Lang, to: &str, from: &str, meter: &MeterVerdict) -> String {
    let (pct, what) = (meter.meter.pct.round(), meter_phrase(lang, &meter.meter));
    let pace = pace_note(lang, meter);
    tr_in!(
        lang,
        "L’agent passe sur le compte {to} : {from} est à {pct} % de {what}{pace}.",
        "The agent goes on on the {to} account: {from} is at {pct}% of {what}{pace}."
    )
}

/// What the notification says of the agent that went on on `to`.
pub(crate) fn switched_toast(
    lang: Lang,
    agent: &str,
    to: &str,
    from: &str,
    meter: &MeterVerdict,
) -> String {
    let (pct, what) = (meter.meter.pct.round(), meter_phrase(lang, &meter.meter));
    let pace = pace_note(lang, meter);
    tr_in!(
        lang,
        "{agent} passe sur {to} : {from} à {pct} % de {what}{pace}.",
        "{agent} goes on on {to}: {from} at {pct}% of {what}{pace}."
    )
}

/// What the conversation says when the agent stays on `from`, at the threshold at `meter`, with no
/// account to go on on (`why`); `overage`: the account answers as extra usage, billed apart.
pub(crate) fn stays_line(
    lang: Lang,
    from: &str,
    harness: &str,
    meter: &MeterVerdict,
    why: NoOther,
    overage: bool,
) -> String {
    let (pct, what) = (meter.meter.pct.round(), meter_phrase(lang, &meter.meter));
    let pace = pace_note(lang, meter);
    let line = match why {
        NoOther::Nobody => tr_in!(
            lang,
            "{from} est à {pct} % de {what}{pace}, mais aucun autre compte {harness} ne peut prendre l’agent (sous le seuil ou à bon rythme, connecté, servant ce projet) : il y reste.",
            "{from} is at {pct}% of {what}{pace}, but no other {harness} account can take the agent (under the threshold or on pace, signed in, serving this project): it stays there."
        ),
        NoOther::Private => tr_in!(
            lang,
            "{from} est à {pct} % de {what}{pace}, mais ce projet privé ne passe pas tout seul sur un compte qu’il partage : l’agent y reste.",
            "{from} is at {pct}% of {what}{pace}, but this private project doesn’t go on its own to an account it shares: the agent stays there."
        ),
        NoOther::Pinned => tr_in!(
            lang,
            "{from} est à {pct} % de {what}{pace}, mais le ticket de l’agent est fixé sur ce compte : il y reste.",
            "{from} is at {pct}% of {what}{pace}, but the agent’s ticket is set on this account: it stays there."
        ),
    };
    if !overage {
        return line;
    }
    let billed = tr_in!(
        lang,
        "Ce compte est en usage supplémentaire, facturé à part.",
        "This account is in extra usage, billed separately."
    );
    format!("{line} {billed}")
}

/// While it lives, the agent is being looked at for a switch: another look waits for none.
struct Switching<'a, R: Runtime>(&'a Core<R>, String);

impl<R: Runtime> Drop for Switching<'_, R> {
    fn drop(&mut self) {
        self.0.switching.lock().remove(&self.1);
    }
}

impl<R: Runtime> Core<R> {
    /// `switch_if_over`, in the background: a reading of the quota of the agent's account just came.
    pub(crate) fn switch_soon(self: &Arc<Self>, id: &str) {
        let (c, id) = (self.clone(), id.to_string());
        tauri::async_runtime::spawn(async move {
            c.switch_if_over(&id).await;
        });
    }

    /// The agent goes on on another account of its harness when its own makes it leave at « Changer
    /// de compte à » (a short window at the threshold, a long one at the threshold whose pace is not
    /// good: `pace::judge`) and it is free: its turn over, no question or permission waiting for the user,
    /// nothing running in the background in its process, no reset of a usage limit waited for (the
    /// move at the limit is `switch_on_limit`'s). The account it goes to is under the threshold
    /// itself (`other_account`, `board::pick_threshold`: one an agent would not leave), so that it
    /// never goes back and forth: it leaves that one only once that one passes it too, and the first
    /// is a choice again once reset. With none, it stays, and its conversation says why, once for that account. True when
    /// it moved.
    pub(crate) async fn switch_if_over(self: &Arc<Self>, id: &str) -> bool {
        let settings = self.settings.read().clone();
        let threshold = board::switch_threshold(&settings);
        // « À la limite »: nothing moves before it.
        if threshold >= 100 {
            return false;
        }
        if !self.switching.lock().insert(id.to_string()) {
            return false;
        }
        let _looking = Switching(self, id.to_string());
        let Ok(h) = self.agent(id) else { return false };
        let account = {
            let rt = h.lock();
            // A harness whose sessions go from an account to another (Claude Code alone today).
            let moves = crate::engine::by_id(rt.meta.engine)
                .caps()
                .resume_other_account;
            let free = moves
                && !rt.meta.archived
                // The Claude Code view's process is its interface: not moved under the user.
                && !rt.meta.terminal
                && !rt.meta.status.is_active()
                && rt.meta.resume_at.is_none()
                && rt.meta.session_id.is_some()
                && rt.waiting_views().next().is_none()
                && !rt
                    .meta
                    .plan
                    .as_ref()
                    .is_some_and(|p| p.holds_process(crate::model::now_ms()));
            if !free {
                return false;
            }
            accounts::get(&settings, &rt.meta.account)
        };
        let usage = self.usage.lock().accounts.clone();
        let leaving = accounts::verdict(&usage, &account.id, threshold, self.pause_now()).leaving;
        let Some(meter) = leaving else {
            self.switch_told.lock().remove(id);
            return false;
        };
        let lang = i18n::ui();
        let from = accounts::name_in(lang, &account);
        let target = match self.other_account(id, board::pick_threshold(&settings)) {
            Ok(target) => target,
            Err(why) => {
                // Said once while the agent stays on that account.
                let told = self
                    .switch_told
                    .lock()
                    .insert(id.to_string(), account.id.clone());
                if told.as_deref() != Some(account.id.as_str()) {
                    let overage = usage.iter().any(|u| u.id == account.id && u.overage);
                    let harness = crate::engine::by_id(account.engine).name();
                    let text = stays_line(lang, &from, harness, &meter, why, overage);
                    log::info!(
                        "agent {id}: stays on the account {} past the threshold",
                        account.id
                    );
                    let _ = self.with_agent(id, |rt, fx| {
                        rt.notice("warn", text, fx);
                        Ok(())
                    });
                }
                return false;
            }
        };
        if let Err(e) = self.move_session(id, &target).await {
            // Its turn begun meanwhile, its process gone: the next end of a turn looks again.
            log::warn!("agent {id}: no move to the account {target} past the threshold: {e:#}");
            return false;
        }
        self.switch_told.lock().remove(id);
        let to = accounts::name_in(lang, &accounts::get(&self.settings.read(), &target));
        log::info!(
            "agent {id}: from the account {} to {target}, past the threshold",
            account.id
        );
        let line = switched_line(lang, &to, &from, &meter);
        // A separator in its conversation, as a handover's (2.0 design, choice 24).
        let (name, project) = match self.with_agent(id, |rt, fx| {
            rt.notice("switch", line, fx);
            Ok((rt.meta.name.clone(), rt.meta.project_id.clone()))
        }) {
            Ok(x) => x,
            Err(_) => return true,
        };
        self.hub.emit(UiEvent::Toast {
            text: switched_toast(lang, &name, &to, &from, &meter),
        });
        let alert = AgentAlert {
            kind: NotifyKind::Done,
            body: switched_line(lang, &to, &from, &meter),
        };
        self.notify_agent(alert, &project, id, &name);
        // Its process started again on the new account: the card shows the account it runs on.
        self.warm(id);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Lang::{En, Fr};
    use crate::pace::Standing;

    fn meter(minutes: Option<u32>, pct: f64) -> QuotaMeter {
        QuotaMeter {
            minutes,
            pct,
            resets_at: Some(2_000_000),
        }
    }

    /// A meter that makes an agent leave because its use reached the threshold.
    fn out(minutes: Option<u32>, pct: f64) -> MeterVerdict {
        MeterVerdict {
            meter: meter(minutes, pct),
            standing: Standing::Out,
            leave: true,
            projected: None,
        }
    }

    /// A long window past the threshold whose pace would not last to its reset.
    fn behind(minutes: Option<u32>, pct: f64, projected: f64) -> MeterVerdict {
        MeterVerdict {
            meter: meter(minutes, pct),
            standing: Standing::Behind,
            leave: true,
            projected: Some(projected),
        }
    }

    #[test]
    fn the_switch_is_said_with_the_window_that_made_the_agent_leave() {
        let five = out(Some(300), 95.4);
        assert_eq!(
            switched_line(Fr, "Pro", "Principal", &five),
            "L’agent passe sur le compte Pro : Principal est à 95 % de sa fenêtre de 5 h."
        );
        assert_eq!(
            switched_line(En, "Pro", "Main", &out(Some(WEEK), 96.0)),
            "The agent goes on on the Pro account: Main is at 96% of its weekly window."
        );
        assert_eq!(
            switched_toast(Fr, "Revue", "Pro", "Principal", &five),
            "Revue passe sur Pro : Principal à 95 % de sa fenêtre de 5 h."
        );
        assert_eq!(
            switched_line(Fr, "B", "A", &out(None, 90.0)),
            "L’agent passe sur le compte B : A est à 90 % de ses crédits du mois."
        );
        // The one used the most of those that make it leave.
        let usage = [AccountUsage {
            id: "a".into(),
            meters: vec![
                QuotaMeter {
                    resets_at: Some(10_000_000),
                    ..meter(Some(300), 91.0)
                },
                QuotaMeter {
                    resets_at: Some(10_000_000),
                    ..meter(Some(300), 97.0)
                },
            ],
            ..Default::default()
        }];
        let leaving = |id, threshold| accounts::verdict(&usage, id, threshold, 0).leaving;
        assert_eq!(leaving("a", 90).map(|m| m.meter.pct), Some(97.0));
        assert_eq!(leaving("a", 98), None);
        assert_eq!(leaving("b", 10), None);
    }

    #[test]
    fn the_pace_is_said_when_it_is_what_puts_the_window_behind() {
        let week = behind(Some(WEEK), 96.0, 168.4);
        assert_eq!(
            switched_line(Fr, "Pro", "Principal", &week),
            "L’agent passe sur le compte Pro : Principal est à 96 % de sa fenêtre hebdomadaire (au rythme actuel, 168 % à sa remise à zéro)."
        );
        assert_eq!(
            switched_line(En, "Pro", "Main", &week),
            "The agent goes on on the Pro account: Main is at 96% of its weekly window (at the current pace, 168% at its reset)."
        );
        assert_eq!(
            switched_toast(Fr, "Revue", "Pro", "Principal", &behind(None, 91.0, 1500.0)),
            "Revue passe sur Pro : Principal à 91 % de ses crédits du mois (au rythme actuel, 999+ % à sa remise à zéro)."
        );
        // A week at its limit, or with no projection, is said by its use alone.
        let mut limit = week;
        limit.standing = Standing::Out;
        assert!(!switched_line(Fr, "Pro", "Principal", &limit).contains("rythme"));
    }

    #[test]
    fn staying_says_why_and_that_extra_usage_is_billed_apart() {
        let five = out(Some(300), 96.0);
        assert_eq!(
            stays_line(Fr, "Principal", "Claude Code", &five, NoOther::Nobody, false),
            "Principal est à 96 % de sa fenêtre de 5 h, mais aucun autre compte Claude Code ne peut prendre l’agent (sous le seuil ou à bon rythme, connecté, servant ce projet) : il y reste."
        );
        assert_eq!(
            stays_line(En, "Main", "Claude Code", &five, NoOther::Private, true),
            "Main is at 96% of its 5-hour window, but this private project doesn’t go on its own to an account it shares: the agent stays there. This account is in extra usage, billed separately."
        );
        assert_eq!(
            stays_line(
                En,
                "Main",
                "Claude Code",
                &behind(Some(WEEK), 96.0, 140.0),
                NoOther::Nobody,
                false
            ),
            "Main is at 96% of its weekly window (at the current pace, 140% at its reset), but no other Claude Code account can take the agent (under the threshold or on pace, signed in, serving this project): it stays there."
        );
    }
}
