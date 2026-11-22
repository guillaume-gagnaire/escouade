//! « Vue <harness> » of a harness whose interface joins the agent's process instead of taking its
//! place (`Engine::attaches`): OpenCode, whose `opencode attach` is one more client of the server
//! the agent's session is on (docs/TODO-1.8.md, « Vue terminal de chaque moteur »). Escouade's own
//! client goes on: the conversation, the state, the permissions and the cost keep coming from its
//! event stream, with no hook nor log to read; a permission answered on one side is seen on the
//! other (the server tells both).
//!
//! The interface gets the server's password in its environment only. Closing it (or leaving the
//! view) stops it alone, never the server, which serves the account's other agents. An interface
//! whose process is gone, or now on another server (started again), is stopped: what it would do
//! there, Escouade would not hear.

use crate::accounts;
use crate::agent::{AgentHandle, AttachRun};
use crate::core::Core;
use crate::engine::terminal::Attach;
use crate::model::new_id;
use crate::pty::TermInfo;
use crate::tui::{Forward, TuiOutput};
use anyhow::{anyhow, bail, Result};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::Runtime;

/// How often an interface is checked against the process it joined.
const WATCH_EVERY: Duration = Duration::from_millis(300);
/// How long an interface stays with no process to join: what a process started again takes.
const NO_PROCESS_GRACE: Duration = Duration::from_secs(3);

/// What becomes of an interface (`watch`).
#[derive(Debug, PartialEq)]
enum Watch {
    Keep,
    Stop,
}

/// What becomes of an interface started as `launched`, the process of its agent now joined by
/// `current` (None: no process, or one with no session), for `without_since` when that is so.
fn watch(launched: &Attach, current: Option<&Attach>, without_since: Option<Duration>) -> Watch {
    match current {
        Some(a) if a == launched => Watch::Keep,
        // Another server (started again, a new address or password): the interface talks to one
        // that is gone.
        Some(_) => Watch::Stop,
        None if without_since.is_some_and(|d| d >= NO_PROCESS_GRACE) => Watch::Stop,
        None => Watch::Keep,
    }
}

impl<R: Runtime> Core<R> {
    /// « Vue <harness> » on or off for an agent whose interface joins its process: on, the window
    /// opens the interface (`open_attached`) and the process starts if it was not running; off,
    /// the interface stops. The process goes on either way, even in the middle of a turn.
    pub(crate) async fn set_attached(
        self: &Arc<Self>,
        h: &AgentHandle,
        id: &str,
        on: bool,
    ) -> Result<()> {
        let term = {
            let lock = self.spawn_lock(id);
            let _guard = lock.lock().await;
            let mut rt = h.lock();
            if rt.meta.attached == on {
                return Ok(());
            }
            if on && rt.meta.ticket_id.is_some() {
                return Err(crate::tui::ticket_refusal());
            }
            rt.meta.attached = on;
            if on {
                None
            } else {
                rt.attach.take().map(|a| a.info.id)
            }
        };
        if let Some(t) = term {
            let pty = self.pty.clone();
            let _ = tokio::task::spawn_blocking(move || pty.kill(&t)).await;
        }
        self.emit_agent(h);
        self.request_save();
        if on {
            self.warm(id);
        }
        Ok(())
    }

    /// The interface joined to the agent's process, in a terminal the window shows (`on_data`).
    /// One that runs already is taken over by the window, sent its screen first. Else the process
    /// is started first if need be (the interface joins its session), then the interface.
    pub(crate) async fn open_attached(
        self: &Arc<Self>,
        h: &AgentHandle,
        id: &str,
        size: (u16, u16),
        on_data: Forward,
    ) -> Result<TermInfo> {
        if self.quitting.load(std::sync::atomic::Ordering::Acquire) {
            bail!(crate::core::quitting_refusal());
        }
        let harness = {
            let rt = h.lock();
            let harness = crate::engine::of(&rt.meta).name();
            if !rt.meta.attached {
                bail!(tr!(
                    "L’agent n’est pas en vue {harness}.",
                    "The agent is not in the {harness} view.",
                    harness = harness
                ));
            }
            if rt.meta.ticket_id.is_some() {
                return Err(crate::tui::ticket_refusal());
            }
            if rt.meta.archived {
                bail!(tr!("L’agent est archivé.", "The agent is archived."));
            }
            harness
        };
        let mut on_data = Some(on_data);
        if let Some(info) = self.take_over_attached(h, size, &mut on_data) {
            return Ok(info);
        }
        // Its process first, with its session: what the interface joins.
        let proc = self.ensure_process(id).await?;
        let attach = proc.attach().ok_or_else(|| {
            anyhow!(tr!(
                "{harness} n’a pas encore ouvert la session de l’agent : réessaie dans un instant.",
                "{harness} has not opened the agent’s session yet: try again in a moment.",
                harness = harness
            ))
        })?;
        let lock = self.spawn_lock(id);
        let _guard = lock.lock().await;
        // Another window started it meanwhile.
        if let Some(info) = self.take_over_attached(h, size, &mut on_data) {
            return Ok(info);
        }
        let settings = self.settings.read().clone();
        let (account, cwd, name) = {
            let rt = h.lock();
            if !rt.meta.attached || rt.meta.archived {
                bail!(tr!(
                    "L’agent a quitté la vue {harness}.",
                    "The agent left the {harness} view.",
                    harness = harness
                ));
            }
            (
                accounts::get(&settings, &rt.meta.account),
                rt.meta.cwd.clone(),
                rt.meta.name.clone(),
            )
        };
        let program = accounts::program(&account, &settings).ok_or_else(|| {
            anyhow!(crate::engine::missing_line(
                account.engine,
                crate::i18n::ui()
            ))
        })?;
        crate::proxysecret::refuse_without_password(&settings)?;
        let mut env = [settings.agent_env(), accounts::launch_env(&account)].concat();
        env.extend(attach.env.iter().cloned());
        crate::tui::no_proxy(&mut env);
        let info = TermInfo {
            id: new_id(),
            // No project's: the window lists it with no project's terminals.
            project_id: String::new(),
            name,
            shell: "opencode".into(),
        };
        // Its arguments hold no secret: they go to the log as they are.
        log::info!(
            "agent {id}: starting {harness}'s interface {} {} in {cwd} (account {})",
            program.describe(),
            attach.args.join(" "),
            account.id
        );
        let output = Arc::new(TuiOutput::new(on_data.take()));
        let on_out = {
            let (pty, term, output) = (self.pty.clone(), info.id.clone(), output.clone());
            move |data: Vec<u8>| {
                // With no window to answer it, the pseudo-console's cursor position query is
                // answered here (as for an interface that takes the process's place).
                if output.push(data) {
                    let _ = pty.write(&term, crate::tui::CURSOR_HOME);
                }
            }
        };
        let (me, agent, term) = (Arc::downgrade(self), id.to_string(), info.id.clone());
        self.pty.spawn_program(
            info.clone(),
            &program,
            &attach.args,
            &cwd,
            size,
            env,
            false,
            on_out,
            move |code| {
                log::info!("agent {agent}: interface {term} ended (code {code:?})");
                if let Some(c) = me.upgrade() {
                    c.attached_ended(&agent, &term);
                }
            },
        )?;
        h.lock().attach = Some(AttachRun {
            info: info.clone(),
            output,
            launched: attach,
        });
        self.emit_agent(h);
        let (c, agent, term) = (self.clone(), id.to_string(), info.id.clone());
        tauri::async_runtime::spawn(async move { c.watch_attached(agent, term).await });
        Ok(info)
    }

    /// The interface that runs already, taken over by the window of `on_data` (taken from it).
    fn take_over_attached(
        &self,
        h: &AgentHandle,
        size: (u16, u16),
        on_data: &mut Option<Forward>,
    ) -> Option<TermInfo> {
        let (info, output) = {
            let rt = h.lock();
            let run = rt.attach.as_ref()?;
            if !self.pty.exists(&run.info.id) {
                return None;
            }
            (run.info.clone(), run.output.clone())
        };
        output.attach(on_data.take()?);
        // It draws its screen again at the window's size.
        let _ = self.pty.resize(&info.id, size.0, size.1);
        Some(info)
    }

    /// The interface `term` of the agent `id` ended: the agent no longer has it (it stays in the
    /// view, whose « Relancer » starts another).
    fn attached_ended(&self, id: &str, term: &str) {
        let Ok(h) = self.agent(id) else { return };
        let ended = {
            let mut rt = h.lock();
            let mine = rt.attach.as_ref().is_some_and(|a| a.info.id == term);
            if mine {
                rt.attach = None;
            }
            mine
        };
        if ended {
            self.emit_agent(&h);
        }
    }

    /// Follows the interface `term` of the agent `id` until it ends: stopped once the agent is
    /// gone, or its process (`watch`).
    async fn watch_attached(self: Arc<Self>, id: String, term: String) {
        let mut without: Option<Instant> = None;
        loop {
            tokio::time::sleep(WATCH_EVERY).await;
            let verdict = match self.agent(&id) {
                Err(_) => Watch::Stop,
                Ok(h) => {
                    let rt = h.lock();
                    let Some(run) = rt.attach.as_ref().filter(|a| a.info.id == term) else {
                        return;
                    };
                    let current = rt.proc.as_ref().and_then(|p| p.attach());
                    if current.is_some() {
                        without = None;
                    } else {
                        without.get_or_insert_with(Instant::now);
                    }
                    watch(
                        &run.launched,
                        current.as_ref(),
                        without.map(|t| t.elapsed()),
                    )
                }
            };
            if verdict == Watch::Stop {
                log::info!(
                    "agent {id}: interface {term} stopped, the process it joined is gone or changed"
                );
                let pty = self.pty.clone();
                let _ = tokio::task::spawn_blocking(move || pty.kill(&term)).await;
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn joined(password: &str) -> Attach {
        Attach {
            args: vec!["attach".into(), "http://127.0.0.1:4096".into()],
            env: vec![("OPENCODE_SERVER_PASSWORD".into(), password.into())],
        }
    }

    #[test]
    fn an_interface_stays_while_its_process_is_on_the_same_server() {
        let a = joined("p");
        assert_eq!(watch(&a, Some(&joined("p")), None), Watch::Keep);
    }

    #[test]
    fn an_interface_stops_once_the_server_of_its_process_changed() {
        assert_eq!(watch(&joined("p"), Some(&joined("q")), None), Watch::Stop);
    }

    #[test]
    fn an_interface_without_a_process_waits_for_one_a_few_seconds() {
        let a = joined("p");
        assert_eq!(
            watch(&a, None, Some(Duration::from_millis(100))),
            Watch::Keep
        );
        assert_eq!(watch(&a, None, Some(NO_PROCESS_GRACE)), Watch::Stop);
    }
}
