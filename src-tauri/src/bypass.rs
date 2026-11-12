//! « Autoriser le mode Bypass » (audit SEC-09): the mode where a harness asks nothing (`bypassPermissions`) is open to the
//! agents only once the user allowed it in Réglages › Agents, a setting off by default. Without it, no agent starts nor
//! goes into it, and Claude Code is not given `--allow-dangerously-skip-permissions`, which makes it reachable in a
//! session. An installation that already used the mode keeps it: the setting is found on at its first start with it
//! (`settle_bypass`), and the modes of its agents and tickets stay as they are.

use crate::core::Core;
use anyhow::{bail, Result};
use tauri::Runtime;

/// The mode of no question.
pub const BYPASS: &str = "bypassPermissions";

/// The arguments that open the mode of no question to a harness's process: removed from a launch when the setting is
/// off, whoever wrote them (also as `--x=y`). Claude Code's two, Codex's, Copilot's `--allow-all` family, Cursor's
/// `--force` (« Run Everything ») and the `--yolo` of Gemini and of the harnesses that follow it (Antigravity).
pub const BYPASS_ARGS: [&str; 9] = [
    "--allow-dangerously-skip-permissions",
    "--dangerously-skip-permissions",
    "--dangerously-bypass-approvals-and-sandbox",
    "--allow-all",
    "--allow-all-tools",
    "--allow-all-paths",
    "--allow-all-urls",
    "--yolo",
    "--force",
];

/// The options that open it by their value (`--option value`, `--option=value`): Gemini's `yolo`, Vibe's
/// `auto-approve`, Claude Code's `bypassPermissions`, a `yolo` mode.
pub const BYPASS_OPTIONS: [(&str, &str); 5] = [
    ("--approval-mode", "yolo"),
    ("--agent", "auto-approve"),
    ("--permission-mode", BYPASS),
    ("--mode", "yolo"),
    ("--mode", BYPASS),
];

/// `args` without what opens the mode of no question, unless `allowed`.
pub fn hold(args: &mut Vec<String>, allowed: bool) {
    if allowed {
        return;
    }
    let opens = |a: &str| {
        let (name, _) = a.split_once('=').unwrap_or((a, ""));
        BYPASS_ARGS.contains(&name)
            || BYPASS_OPTIONS
                .iter()
                .any(|(n, v)| a.strip_prefix(*n).and_then(|r| r.strip_prefix('=')) == Some(*v))
    };
    let mut kept = Vec::with_capacity(args.len());
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        let pair = BYPASS_OPTIONS
            .iter()
            .any(|(n, v)| a == *n && args.get(i + 1).is_some_and(|x| x == v));
        if pair {
            i += 2;
            continue;
        }
        if !opens(a) {
            kept.push(args[i].clone());
        }
        i += 1;
    }
    *args = kept;
}

/// Why an agent does not go in `mode`: the mode of no question, not allowed.
pub fn refusal(mode: &str, allowed: bool) -> Option<String> {
    (mode == BYPASS && !allowed).then(|| {
        tr!(
            "Le mode Bypass n’est pas autorisé : active « Autoriser le mode Bypass » dans Réglages › Agents, ou choisis un autre mode.",
            "Bypass mode is not allowed: turn on “Allow Bypass mode” in Settings › Agents, or choose another mode."
        )
    })
}

/// `refusal` as an error.
pub fn check(mode: &str, allowed: bool) -> Result<()> {
    match refusal(mode, allowed) {
        Some(why) => bail!(why),
        None => Ok(()),
    }
}

impl<R: Runtime> Core<R> {
    /// Whether the mode of no question is open to the agents (`Settings::allow_bypass`).
    pub fn bypass_allowed(&self) -> bool {
        self.settings.read().bypass_allowed()
    }

    /// Once, at the first start that knows the setting: on when the installation already uses the mode of no question
    /// (an account's default, a board's, an agent's mode or the one its plan goes back to), off otherwise. Written at
    /// once, so the user's later choice is never guessed again.
    pub(crate) fn settle_bypass(&self) {
        if self.settings.read().allow_bypass.is_some() {
            return;
        }
        let used = self
            .settings
            .read()
            .accounts
            .iter()
            .any(|a| a.default_mode == BYPASS)
            || self.projects.read().iter().any(|p| p.board.mode == BYPASS)
            || self.agents.read().values().any(|h| {
                let rt = h.lock();
                rt.meta.mode == BYPASS || rt.meta.plan_from.as_deref() == Some(BYPASS)
            });
        self.settings.write().allow_bypass = Some(used);
        if used {
            log::info!("settings: Bypass mode allowed, as this installation already uses it");
        }
        self.write_settings_file();
    }
}

#[cfg(test)]
#[path = "bypass_tests.rs"]
mod tests;
