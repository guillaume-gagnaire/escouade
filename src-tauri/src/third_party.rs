//! What a plugin of a third party may do without a review (audit SEC-01, ESC-589).
//!
//! - « N'installer que des versions vérifiées » holds for a folder and a git repository too: one is accepted, the setting
//!   on, only when the store has the same id and the same version verified, of the same files (`held_to_store`). A
//!   folder in development stays another gesture, said as such in the window, and the setting does not hold it.
//! - The setting turns on once, at the update, for an installation that has no plugin of a third party installed
//!   (`settle_verified_only`); one that has one keeps its choice.
//! - A harness a plugin of a third party describes asks the user at its first launch, its whole command said
//!   (`plugins::consent`): the core keeps what was agreed in the plugin's choices, by the fingerprint of the line
//!   (`agree_to_harness`), and hands it to the places that run its program (`sync_consents`). « Harness des plugins
//!   tiers : Refuser » runs none.

use crate::core::Core;
use crate::engine::{self, Program};
use crate::i18n::Lang;
use crate::model::{EngineId, ThirdPartyHarnesses};
use crate::plugins::consent::{self, Held, LaunchLine};
use anyhow::{anyhow, bail, Result};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use tauri::Runtime;

/// How many lines of a harness an agreement keeps (accounts whose program is found elsewhere each have their own).
const KEPT: usize = 4;

/// A harness of a third party whose command does not run now, as the window reads it (`harness_launches`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LaunchHeld {
    /// Its command waits for the user's agreement.
    Asks,
    /// « Harness des plugins tiers : Refuser ».
    Refused,
}

/// The command of a harness of a third party, as the window of the agreement shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchAsk {
    #[serde(flatten)]
    pub line: LaunchLine,
    /// The name of the plugin that describes it, in the interface's language.
    pub plugin_name: String,
    /// Why it does not run now; None once agreed to.
    pub held: Option<LaunchHeld>,
}

impl<R: Runtime> Core<R> {
    /// « N'installer que des versions vérifiées » for what an inspection of a folder or a git repository set aside: kept
    /// as it is with the setting off; on, kept only when the store's version of the same id and the same version is
    /// verified and of the same files, else refused with why and the gesture that installs it anyway.
    pub async fn held_to_store(
        &self,
        preview: crate::plugins::Preview,
        lang: Lang,
    ) -> Result<crate::plugins::Preview, crate::plugins::Refusal> {
        let settings = self.settings.read().clone();
        if !settings.plugins_verified_only {
            return Ok(preview);
        }
        let fetched = match crate::plugins::store::client(&settings) {
            Ok(http) => {
                self.store
                    .fetch(&http, &preview.id, Some(&preview.version), true, lang)
                    .await
            }
            Err(e) => Err(e.to_string()),
        };
        self.plugins.hold_to_store(preview, fetched, lang)
    }

    /// Once, at the first start of a version that has it (ESC-589): « N'installer que des versions vérifiées » turned on
    /// for an installation that has no plugin of a third party installed (an id that is not `dev.escouade.*`); one that
    /// has one keeps its choice. A new installation has it on and settled already.
    pub(crate) fn settle_verified_only(&self) {
        if self.settings.read().plugins_verified_settled {
            return;
        }
        let third = self
            .plugins
            .list(crate::i18n::ui())
            .into_iter()
            .any(|p| !crate::plugins::builtin::first_hand(&p.id));
        {
            let mut s = self.settings.write();
            s.plugins_verified_settled = true;
            if !third && !s.plugins_verified_only {
                s.plugins_verified_only = true;
                log::info!(
                    "settings: “install verified versions only” turned on, no plugin of a third party being installed"
                );
            }
        }
        self.write_settings_file();
    }

    /// What was agreed to, handed to the places that run a harness's program (`consent::set`): the lines kept in each
    /// plugin's choices, by the engine of its harness, and « Refuser ».
    pub fn sync_consents(&self) {
        let refuse_all = self.settings.read().third_party_harnesses == ThirdPartyHarnesses::Refuse;
        let mut approved: HashMap<EngineId, Vec<String>> = HashMap::new();
        for p in self.plugins.list(crate::i18n::Lang::En) {
            if crate::plugins::builtin::first_hand(&p.id) {
                continue;
            }
            for (harness, lines) in &p.choices.harnesses_approved {
                approved.insert(
                    crate::plugins::harness::described_id(&p.id, harness),
                    lines.clone(),
                );
            }
        }
        consent::set(refuse_all, approved);
    }

    /// The program an agent of `engine` would run: its account's when `agent` is given, else the harness's own.
    fn program_for(&self, engine: EngineId, agent: Option<&str>) -> Result<Program> {
        let settings = self.settings.read().clone();
        if let Some(id) = agent {
            let h = self.agent(id)?;
            let account = self.agent_account(&h)?;
            return crate::accounts::program(&account, &settings)
                .ok_or_else(|| anyhow!(engine::missing_line(engine, crate::i18n::ui())));
        }
        let harness = engine::by_id(engine);
        harness
            .locate(harness.settings_path(&settings), &settings.wsl_distro)
            .ok_or_else(|| anyhow!(engine::missing_line(engine, crate::i18n::ui())))
    }

    /// The whole command of `engine` (for the agent `agent`, else the harness's own program), when a plugin of a third
    /// party describes it: what the window of the agreement shows, and whether it runs now. None for the others.
    pub fn launch_ask(&self, engine: EngineId, agent: Option<&str>) -> Result<Option<LaunchAsk>> {
        let harness = engine::by_id(engine);
        let program = self.program_for(engine, agent)?;
        let Some(line) = harness.launch_line(&program) else {
            return Ok(None);
        };
        let plugin_name = self
            .plugins
            .get(&line.plugin, crate::i18n::ui())
            .map(|p| p.name)
            .unwrap_or_else(|| line.plugin.clone());
        let held = consent::check(&line).err().map(|h| match h {
            Held::Asks => LaunchHeld::Asks,
            Held::Refused => LaunchHeld::Refused,
        });
        Ok(Some(LaunchAsk {
            line,
            plugin_name,
            held,
        }))
    }

    /// The harnesses of a third party whose command does not run now, as the window reads them: their agents say so and
    /// offer « Voir la commande… ». Each program its agents may run is judged, its accounts' and its own: one that
    /// waits is enough, « Refuser » first.
    pub fn harness_launches(&self) -> BTreeMap<EngineId, LaunchHeld> {
        let settings = self.settings.read().clone();
        crate::plugins::harness::described_of_third_parties()
            .into_iter()
            .filter_map(|e| {
                let harness = engine::by_id(e);
                let programs = settings
                    .accounts
                    .iter()
                    .filter(|a| a.engine == e)
                    .filter_map(|a| crate::accounts::program(a, &settings))
                    .chain(harness.locate(harness.settings_path(&settings), &settings.wsl_distro));
                let held: Vec<Held> = programs
                    .filter_map(|p| harness.launch_line(&p))
                    .filter_map(|line| consent::check(&line).err())
                    .collect();
                if held.contains(&Held::Refused) {
                    Some((e, LaunchHeld::Refused))
                } else if held.contains(&Held::Asks) {
                    Some((e, LaunchHeld::Asks))
                } else {
                    None
                }
            })
            .collect()
    }

    /// « Autoriser » (the user's gesture, in the window or by the client that launches): the line of `engine` whose
    /// fingerprint the user read is kept as agreed, after it is made again here: a line changed meanwhile is refused, to
    /// be read again. « Refuser » (`allow` false) keeps nothing, and the agent's conversation says it does not start.
    pub fn agree_to_harness(
        self: &Arc<Self>,
        engine: EngineId,
        agent: Option<&str>,
        fingerprint: &str,
        allow: bool,
    ) -> Result<()> {
        let ask = self
            .launch_ask(engine, agent)?
            .ok_or_else(|| anyhow!("{engine}: no command of a third party to agree to"))?;
        let line = ask.line;
        if !allow {
            log::info!(
                "{engine}: its command refused by the user ({})",
                line.program
            );
            if let Some(id) = agent {
                let text = tr!(
                    "Tu as refusé de lancer {name} : l’agent ne démarre pas. « Voir la commande… » te la redemande.",
                    "You refused to launch {name}: the agent doesn’t start. “See the command…” asks you again.",
                    name = line.harness
                );
                let _ = self.with_agent(id, |rt, fx| {
                    rt.notice("warn", text, fx);
                    Ok(())
                });
            }
            return Ok(());
        }
        if self.settings.read().third_party_harnesses == ThirdPartyHarnesses::Refuse {
            bail!(consent::refusal(&line, &Held::Refused));
        }
        if line.fingerprint != fingerprint {
            bail!(tr!(
                "La commande de {name} a changé depuis que tu l’as lue : relis-la.",
                "The command of {name} changed since you read it: read it again.",
                name = line.harness
            ));
        }
        let harness_id = engine
            .as_str()
            .rsplit_once(':')
            .map(|(_, h)| h.to_string())
            .unwrap_or_else(|| engine.as_str().to_string());
        let kept = self
            .plugins
            .choose(&line.plugin, |c| {
                let lines = c.harnesses_approved.entry(harness_id).or_default();
                lines.retain(|f| f != &line.fingerprint);
                lines.insert(0, line.fingerprint.clone());
                lines.truncate(KEPT);
            })
            .map_err(|e| anyhow!(e))?;
        if !kept {
            bail!("{}: not installed", line.plugin);
        }
        log::info!(
            "{engine}: its command agreed to by the user: {} ({})",
            line.program,
            &line.fingerprint[..12]
        );
        self.sync_consents();
        if let Some(id) = agent {
            self.warm(id);
        }
        Ok(())
    }

    /// Before a harness's program runs for an agent (its start, « Se connecter… »): a harness of a third party whose
    /// command is not agreed to, or refused by the setting, does not run, said.
    pub(crate) fn check_harness_consent(
        &self,
        harness: &dyn engine::Engine,
        program: &Program,
    ) -> Result<()> {
        match harness.launch_line(program) {
            Some(line) => consent::check(&line).map_err(|h| anyhow!(consent::refusal(&line, &h))),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
#[path = "third_party_tests.rs"]
mod tests;
