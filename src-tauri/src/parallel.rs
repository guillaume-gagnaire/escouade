//! How many tickets of a Kanban run at once, in the core (ESC-620): the one bound of `escouade_model::parallel` (16 on
//! a desktop, 64 on a server), lowered by the ceiling the managed device or the team imposes (`projects.maxParallel`,
//! the locks of the project). The board (`board::to_start`, `ticketprs::to_fix`), the MCP (`start_ticket`,
//! `set_board`, `get_board`), the window (`board_set`, `board_capacity`) and the remote API (its served commands) all
//! go by [`Core::parallel_max`].

use std::sync::atomic::Ordering;

use anyhow::{bail, Result};
use escouade_model::parallel::{self, Machine};
use serde::Serialize;

use crate::core::Core;
use crate::locks::LockedBy;
use crate::model::{BoardSettings, Project};

/// A ceiling imposed, and who imposes it.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ceiling {
    pub value: u32,
    pub by: LockedBy,
}

/// What the window shows under « En parallèle »: the bound, the ceiling and where it comes from, and what this machine
/// holds (the server's, for a server's space: its core answers). The warning beyond `warn_above` is the window's to say.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Capacity {
    /// The core is a server's (`escouade-server`).
    pub server: bool,
    /// The machine's bound: 16 or 64.
    pub limit: u32,
    /// The bound with the ceiling: the most the setting takes.
    pub max: u32,
    pub ceiling: Option<Ceiling>,
    /// Beyond it, the warning.
    pub warn_above: u32,
    pub machine: Machine,
    /// About how many agents that compile and run tests this machine holds.
    pub fits: u32,
    /// The room an agent takes, measured (MB).
    pub agent_mb: u64,
}

impl Core {
    /// This core is a server's (`escouade-server`; a test).
    pub(crate) fn mark_server(&self) {
        self.server.store(true, Ordering::Relaxed);
    }

    pub(crate) fn is_server(&self) -> bool {
        self.server.load(Ordering::Relaxed)
    }

    /// The ceiling imposed on the Kanban of `project_id` (its team's locks for a team's project, the device's).
    pub(crate) fn parallel_ceiling(&self, project_id: &str) -> Option<Ceiling> {
        Self::ceiling_of(&self.locks_for(project_id))
    }

    fn ceiling_of(locks: &crate::locks::Locks) -> Option<Ceiling> {
        let (value, by) = locks.max_parallel()?;
        Some(Ceiling {
            value,
            by: by.clone(),
        })
    }

    /// The most tickets at once on the Kanban of `project_id`.
    pub(crate) fn parallel_max(&self, project_id: &str) -> u32 {
        parallel::max(
            self.is_server(),
            self.parallel_ceiling(project_id).map(|c| c.value),
        )
    }

    /// The board's settings as it starts by: its « En parallèle » within the bound.
    pub(crate) fn board_within(&self, p: &Project) -> BoardSettings {
        let mut board = p.board.clone();
        board.max_parallel = board
            .max_parallel
            .clamp(parallel::MIN, self.parallel_max(&p.id));
        board
    }

    /// `n` for the Kanban of `project_id`, refused beyond its bound and said why (`set_board`, `board_set`).
    pub(crate) fn check_parallel(&self, project_id: &str, n: u32) -> Result<()> {
        let max = self.parallel_max(project_id);
        if (parallel::MIN..=max).contains(&n) {
            return Ok(());
        }
        // The ceiling said when it is what holds, not the machine's bound.
        let limit = parallel::limit(self.is_server());
        match self
            .parallel_ceiling(project_id)
            .filter(|c| c.value < limit)
        {
            Some(c) if n > max => bail!(tr!(
                "maxParallel vaut au plus {max}, le plafond imposé {by} (reçu {n}).",
                "maxParallel is {max} at most, the ceiling imposed {by} (got {n}).",
                by = c.by.by()
            )),
            _ => bail!(tr!(
                "maxParallel va de 1 à {max} sur {machine} (reçu {n}).",
                "maxParallel goes from 1 to {max} on {machine} (got {n}).",
                machine = if self.is_server() {
                    tr!("un serveur", "a server")
                } else {
                    tr!("un poste", "a desktop")
                }
            )),
        }
    }

    /// « En parallèle » of the Kanban of `project_id`, as the window shows it; of a project to come (the assistant
    /// « Nouveau projet ») with none, under the device's ceiling alone.
    pub fn board_capacity(&self, project_id: Option<&str>) -> Result<Capacity> {
        let server = self.is_server();
        let ceiling = match project_id {
            Some(id) => {
                self.project(id)?;
                self.parallel_ceiling(id)
            }
            None => Self::ceiling_of(&self.locks.read()),
        };
        let machine = Machine::this();
        Ok(Capacity {
            server,
            limit: parallel::limit(server),
            max: parallel::max(server, ceiling.as_ref().map(|c| c.value)),
            ceiling,
            warn_above: parallel::WARN_ABOVE,
            fits: machine.fits(),
            machine,
            agent_mb: parallel::AGENT_MB,
        })
    }
}
