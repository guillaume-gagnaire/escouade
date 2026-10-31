//! The harness an agent names that this version does not have (INT-3): a later version's, a
//! plugin's gone or turned off. Its agents keep their data and stay in the list, marked « harness
//! absent » (docs/PLUGINS.md §4.3); they do not start, and none is created on it (`written`). It
//! answers what the core asks of any harness, and nothing of it runs.

use super::registry::{
    AccountStore, CostKind, EngineCaps, ForkPoint, Install, MidTurn, ModelDecl, QuotaKind, Tested,
};
use super::{
    mcp, BoxFuture, Engine, Escouade, Message, OneShot, Process, Program, Room, Spawn, StartLost,
};
use crate::agent::{AgentRt, Effects};
use crate::core::Attachment;
use crate::i18n::Lang;
use crate::mcp::install::{Entry, Standing, Target};
use crate::model::EngineId;
use anyhow::{anyhow, Result};
use parking_lot::Mutex;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

/// The harness `id` names, absent here.
pub struct Absent {
    id: EngineId,
}

/// The absent harness of `id`, made once and kept (an id's text is kept as long, `EngineId::new`).
pub fn of(id: EngineId) -> &'static Absent {
    static MADE: Mutex<Option<HashMap<EngineId, &'static Absent>>> = Mutex::new(None);
    let mut made = MADE.lock();
    let kept: &'static Absent = made
        .get_or_insert_with(HashMap::new)
        .entry(id)
        .or_insert_with(|| Box::leak(Box::new(Absent { id })));
    kept
}

/// Why an agent on `id` does not start, in `lang`.
pub fn line(id: EngineId, lang: Lang) -> String {
    tr_in!(
        lang,
        "Harness absent : « {id} » n'est pas dans cette version d'Escouade, ou son plugin est désactivé. L'agent et sa conversation sont gardés.",
        "Harness missing: “{id}” is not in this version of Escouade, or its plugin is turned off. The agent and its conversation are kept."
    )
}

impl Engine for Absent {
    fn id(&self) -> EngineId {
        self.id
    }
    /// Its id, as the data says it: no other name is known of it.
    fn name(&self) -> &'static str {
        self.id.as_str()
    }
    fn speaker(&self) -> &'static str {
        self.id.as_str()
    }
    fn instructions_room(&self) -> Option<Room> {
        None
    }
    fn subagent_relay(&self, _: Lang, _: &str, _: &str, _: &str) -> Option<String> {
        None
    }
    fn locate(&self, _: &str, _: &str) -> Option<Program> {
        None
    }
    fn tested(&self) -> Tested {
        Tested {
            oldest: "0",
            newest: "0",
        }
    }
    fn oldest_reason(&self, lang: Lang) -> String {
        line(self.id, lang)
    }
    fn update_command(&self) -> &'static str {
        ""
    }
    fn version_args(&self) -> &'static [&'static str] {
        &[]
    }
    /// It can do nothing: the interface shows none of its functions.
    fn caps(&self) -> EngineCaps {
        EngineCaps {
            modes: Vec::new(),
            effort: false,
            images: false,
            pdf: false,
            fork: ForkPoint::None,
            resume_other_account: false,
            mid_turn: MidTurn::Refused,
            slash_commands: false,
            quota: QuotaKind::None,
            quota_windows: Vec::new(),
            cost: CostKind::None,
            terminal_view: false,
            remote_control: false,
            background_tasks: false,
            todo_tools: false,
            default_refusal: false,
            sandbox: false,
            usage_page: None,
            accounts_by_key: false,
            read_only: false,
        }
    }
    fn accounts(&self) -> AccountStore {
        AccountStore::ConfigDir { var: "" }
    }
    fn terminal(&self) -> Option<&'static dyn super::TerminalView> {
        None
    }
    /// No folder of its is known: nothing to refuse of it.
    fn session_home(&self) -> Option<crate::fence::HomeKind> {
        None
    }
    fn install(&self) -> Install {
        Install {
            windows: "",
            unix: "",
        }
    }
    fn models(&self) -> Vec<ModelDecl> {
        Vec::new()
    }
    fn efforts(&self) -> &'static [&'static str] {
        &[]
    }
    fn supports_effort(&self, _: &str) -> bool {
        false
    }
    fn supports_auto(&self, _: &str) -> bool {
        false
    }
    fn spawn(&self, _: Spawn, _: super::OnFrame, _: super::OnExit) -> Result<Process> {
        Err(anyhow!(line(self.id, crate::i18n::ui())))
    }
    fn message(&self, _: &str, _: &[Attachment]) -> Result<Message> {
        Err(anyhow!(line(self.id, crate::i18n::ui())))
    }
    /// It has no process: no frame comes.
    fn translate(&self, _: &mut AgentRt, _: &Value, _: &mut Effects) {}
    fn start_lost(&self, _: &str) -> Option<StartLost> {
        None
    }
    fn one_shot<'a>(&'a self, _: OneShot<'a>) -> BoxFuture<'a, Result<String>> {
        let why = line(self.id, crate::i18n::ui());
        Box::pin(async move { Err(anyhow!(why)) })
    }
    fn writer(&self, _: super::Weight) -> &'static str {
        self.id.as_str()
    }
    fn grant_file(&self, _: &str, _: &str) -> Option<Vec<u8>> {
        None
    }
    fn refusal(&self) -> Option<Value> {
        None
    }
    fn escouade_launch(&self, _: &Escouade) -> mcp::Launch {
        mcp::Launch {
            args: Vec::new(),
            env: Vec::new(),
        }
    }
    fn declared(&self, _: &Path) -> Standing {
        Standing::NoFile
    }
    fn add_declaration<'a>(
        &'a self,
        _: &'a Target,
        _: &'a Entry,
        _: &'a [&'a str],
    ) -> BoxFuture<'a, Result<()>> {
        let why = line(self.id, crate::i18n::ui());
        Box::pin(async move { Err(anyhow!(why)) })
    }
    fn remove_declaration<'a>(
        &'a self,
        _: &'a Target,
        _: &'a [&'a str],
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(async { Ok(()) })
    }
    fn manual_declaration(&self, _: bool, _: &str, _: Option<&str>, _: &Entry, _: bool) -> String {
        String::new()
    }
}
