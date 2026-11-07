//! A terminal view read through its harness's plugin (`view/translate`, ESC-352, docs/PLUGINS.md §8): the core reads the file
//! the harness's own interface writes and its hooks (`tui`), the plugin says what its lines mean, and what it gives is applied
//! as the items of a process are (`agent/items.rs`). What only the core knows goes with each line (`ViewParams`), or as a note
//! (`{"escouade": …}`: the end of a turn, a hook that says what the file does not).

use super::*;
use crate::plugins::view;
use escouade_plugin_protocol::engine::{Item, ViewKnown, ViewParams};

/// How long the lines of a view go by unread once its plugin was told not to read them: a module that does not answer takes
/// its whole turn at each try, under the agent's lock.
const VIEW_PAUSE: std::time::Duration = std::time::Duration::from_secs(30);

/// Where the view of an agent stands in its plugin (not saved).
#[derive(Default)]
pub(crate) struct ViewState {
    /// The view, as its plugin knows it: another conversation is another key.
    key: Option<String>,
    /// The instance it started on.
    generation: u64,
    /// The turn of its harness under way, as the plugin told it (`Item::ViewTurn`).
    pub(super) turn: Option<String>,
    /// The last turn its plugin told, over or not: a hook told late is not a new turn.
    pub(super) last_turn: Option<String>,
    /// Why the plugin did not read it, said once.
    told: bool,
    /// Its lines go by unread until then.
    pub(super) paused: Option<std::time::Instant>,
    /// The plugin of a test, in place of its harness's.
    #[cfg(test)]
    pub(super) plugin: Option<&'static str>,
}

impl AgentRt {
    /// Another conversation (or another interface): what its plugin kept of the last one goes with its key.
    pub(super) fn view_reset(&mut self) {
        self.view.turn = None;
        self.view.last_turn = None;
        self.view.told = false;
        self.view.paused = None;
        self.tui_asked = None;
        self.view_gone = self.view.key.take().or(self.view_gone.take());
    }

    /// What the conversation holds, for a view that starts on it.
    fn view_known(&mut self) -> Vec<ViewKnown> {
        self.conv
            .items()
            .iter()
            .filter_map(|i| {
                let tool = i["kind"] == "tool";
                let text = |key: &str| i[key].as_str().map(str::to_string);
                Some(ViewKnown {
                    id: i["id"].as_str()?.to_string(),
                    name: text("name").filter(|_| tool),
                    status: text("status").filter(|_| tool),
                })
            })
            .collect()
    }

    /// The plugin that reads the view.
    fn view_plugin(&self) -> Option<&'static str> {
        #[cfg(test)]
        if self.view.plugin.is_some() {
            return self.view.plugin;
        }
        view::plugin_of(self.meta.engine)
    }

    /// `lines` read by the plugin of the agent's harness, what it says applied, `shown` to the window when they are the file's.
    /// Whether it read them. A plugin that does not answer starts anew once; if it still does not, the conversation says so,
    /// once, and the lines go by unread for `VIEW_PAUSE`, then for a single try at a time.
    fn view_lines(&mut self, lines: Vec<String>, shown: bool, fx: &mut Effects) -> bool {
        let Some(plugin) = self.view_plugin() else {
            return true;
        };
        let now = std::time::Instant::now();
        if self.view.paused.is_some_and(|until| now < until) {
            return false;
        }
        let tries = if self.view.told { 1 } else { 2 };
        let mut why = String::new();
        for _ in 0..tries {
            let generation = match view::generation(plugin) {
                Ok(g) => g,
                Err(e) => {
                    why = e;
                    break;
                }
            };
            let fresh = self.view.key.is_none() || self.view.generation != generation;
            if fresh {
                self.view.key = Some(format!(
                    "{}-{}",
                    self.meta.id,
                    uuid::Uuid::new_v4().simple()
                ));
                self.view.generation = generation;
            }
            let params = ViewParams {
                agent: self.view.key.clone().unwrap_or_default(),
                harness: self.meta.engine.as_str().to_string(),
                fresh,
                known: if fresh { self.view_known() } else { Vec::new() },
                gone: self.view_gone.take().filter(|_| fresh),
                cwd: self.meta.cwd.clone(),
                mode: self.meta.mode.clone(),
                model: self.meta.model.clone(),
                lang: i18n::ui().tag().to_string(),
                main_model: self.main_model.clone(),
                session: self.meta.session_id.clone(),
                active: self.meta.status.is_active(),
                interrupted: self.interrupted,
                copy: self.meta.fork_of.is_some(),
                discussion: self.meta.discussion.is_some(),
                prompt: self.tui_prompt.clone(),
                lines: lines.clone(),
            };
            match view::translate(plugin, &params) {
                Ok(items) => {
                    self.view.told = false;
                    self.view.paused = None;
                    for item in items {
                        let said = !matches!(
                            item,
                            Item::Spoke
                                | Item::Wrote
                                | Item::ViewTurn { .. }
                                | Item::Bridged { .. }
                                | Item::Linked
                        );
                        let ends = matches!(item, Item::Turn { .. });
                        self.apply_item(item, fx);
                        fx.agent_changed |= said && shown;
                        if ends {
                            self.tui_prompt = None;
                        }
                    }
                    return true;
                }
                Err(e) => {
                    log::warn!("{plugin}: a view's lines left unread: {e}");
                    why = e;
                }
            }
        }
        self.view.paused = Some(std::time::Instant::now() + VIEW_PAUSE);
        if !std::mem::replace(&mut self.view.told, true) {
            let name = crate::engine::of(&self.meta).name();
            let text = tr!(
                "Le plugin de {name} n’a pas lu ce que sa vue a écrit ({why}).",
                "The plugin of {name} didn’t read what its view wrote ({why}).",
                name = name,
                why = why
            );
            self.notice("warn", text, fx);
        }
        false
    }

    /// One line of the file its interface writes.
    pub(crate) fn view_line(&mut self, e: &Value, fx: &mut Effects) {
        self.view_lines(vec![e.to_string()], true, fx);
    }

    /// What only the core knows, for the plugin (`ViewParams`). Whether it read it.
    pub(super) fn view_note(&mut self, note: Value, fx: &mut Effects) -> bool {
        self.view_lines(vec![note.to_string()], false, fx)
    }

    /// The end of the running turn, for the plugin (`note`). One that cannot read it does not keep the agent running for ever:
    /// the turn ends here, with what the core knows of it.
    pub(super) fn view_end(&mut self, note: Value, fx: &mut Effects) {
        let error = note["error"].as_str().map(str::to_string);
        if !self.view_note(note, fx) {
            self.apply_item(
                Item::Turn {
                    interrupted: false,
                    limited: None,
                    usage: Default::default(),
                    error,
                    entry: None,
                },
                fx,
            );
        }
    }

    /// A message of its harness, as a process's: a recorded stream in a test.
    #[cfg(test)]
    pub(crate) fn view_frame(&mut self, f: &Value, fx: &mut Effects) {
        self.view_note(json!({ "escouade": "frame", "frame": f }), fx);
    }

    /// What the user typed in the interface (or Escouade pasted there): their message, a turn starts. `prompt` is the id of
    /// its prompt for an interface that writes a message twice.
    pub(super) fn tui_typed(
        &mut self,
        id: Option<String>,
        text: String,
        images: u32,
        prompt: Option<String>,
        fx: &mut Effects,
    ) {
        let twice = prompt.is_some();
        if let Some(prompt) = prompt {
            let prompt = (!prompt.is_empty()).then_some(prompt);
            if prompt.is_some()
                && prompt == self.tui_prompt_id
                && self.tui_prompt.as_deref() == Some(text.as_str())
            {
                return;
            }
            self.tui_prompt_id = prompt;
        }
        let id = id.unwrap_or_else(new_id);
        if self.conv.contains(&id) {
            return;
        }
        let item = json!({
            "kind": "user", "id": id, "text": text, "images": images, "ts": now_ms(),
            "queued": false, "origin": "terminal",
        });
        self.meta.resume_at = None;
        self.append(item, fx);
        if self.tui_prompt.is_none() {
            self.turn_text.clear();
            self.final_text.clear();
            self.interrupted = false;
            self.tui_error = None;
            if self.meta.plan.as_ref().is_some_and(PlanState::is_finished) {
                self.reset_plan(fx);
            }
        }
        if twice || !self.meta.status.is_active() {
            self.set_status(AgentStatus::Running, fx);
        }
        self.meta.prompts += 1;
        self.meta.last_activity = now_ms();
        self.tui_prompt = Some(text);
        fx.agent_changed = true;
        fx.save = true;
    }

    /// A tool of a view whose run is not told: shown as `status`, its status changed when it comes again.
    fn view_row(
        &mut self,
        id: Option<String>,
        name: String,
        input: Value,
        status: String,
        fx: &mut Effects,
    ) {
        if let Some(id) = id.as_deref().filter(|id| self.conv.contains(id)) {
            let id = id.to_string();
            return self.patch(&id, json!({ "status": status }), fx);
        }
        if !self.meta.status.is_active() {
            self.set_status(AgentStatus::Running, fx);
        }
        self.final_text.clear();
        let activity = tool_activity(&name, &input, &self.meta.cwd);
        self.set_activity(activity, fx);
        let item = json!({
            "kind": "tool", "id": id.unwrap_or_else(new_id), "name": name,
            "input": item_input(&name, &input), "status": status, "parent": null, "ts": now_ms(),
        });
        self.append(item, fx);
    }

    /// An item only a view gives: true when `item` is one, applied.
    pub(super) fn apply_view_item(&mut self, item: Item, fx: &mut Effects) -> Option<Item> {
        match item {
            Item::Typed {
                id,
                text,
                images,
                prompt,
            } => self.tui_typed(id, text, images, prompt, fx),
            Item::Spoke => {
                if let Some(run) = self.tui.as_mut() {
                    run.spoke = true;
                }
            }
            Item::Wrote => self.unwritten_session = None,
            Item::Waits { body } => self.tui_wait(body, fx),
            Item::WaitsOn { tool, input } => {
                let body = notify::permission_body(&tool, &input, &self.meta.cwd);
                self.tui_wait(body, fx);
            }
            Item::Asks { tool, input } => {
                self.tui_asked = Some(notify::permission_body(&tool, &input, &self.meta.cwd));
            }
            Item::Runs => {
                if !self.meta.status.is_active() {
                    self.set_status(AgentStatus::Running, fx);
                }
            }
            Item::Goes => {
                self.tui_asked = None;
                if self.meta.status == AgentStatus::Waiting {
                    self.set_status(AgentStatus::Running, fx);
                }
            }
            Item::ViewTurn { id } => {
                if id.is_some() {
                    self.view.last_turn.clone_from(&id);
                }
                self.view.turn = id;
            }
            Item::ViewEnd {
                interrupted,
                duration,
                error,
            } => {
                if interrupted {
                    self.interrupted = true;
                }
                if error.is_some() {
                    self.tui_error = error;
                }
                self.end_tui_turn(duration, fx);
            }
            Item::ViewUsage { id, model, usage } => {
                self.main_model = model.clone();
                self.tui_usage.insert(id, (model, Usage::from_api(&usage)));
                self.context_tokens = [
                    &usage["input_tokens"],
                    &usage["cache_read_input_tokens"],
                    &usage["cache_creation_input_tokens"],
                ]
                .iter()
                .filter_map(|v| v.as_u64())
                .sum();
            }
            Item::ViewError { error } => self.tui_error = Some(error),
            Item::Row {
                id,
                name,
                input,
                status,
            } => self.view_row(id, name, input, status, fx),
            Item::Bridged { url, said } => self.on_bridge_status(url.as_deref(), &said, fx),
            Item::Linked => self.interface_linked(fx),
            other => return Some(other),
        }
        None
    }
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
