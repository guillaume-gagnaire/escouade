//! External ticket systems (Jira, Trello, GitHub Issues, Linear, ClickUp, GitLab Issues, Azure Boards): their accounts,
//! their secrets and the registry of the sources are in `escouade-forge` (docs/CRATES.md), re-exported here where they
//! were. What reads the core stays: the sync of their states with the Kanban's columns (`sync`) and the texts it writes
//! (`text`).

pub use escouade_forge::integrations::*;
// An account of a source, not the harnesses' (`model::Account`), as when it was defined here.
pub use escouade_forge::integrations::Account;

pub mod sync;
pub mod text;

// What `sync` and the tests take of the module (`use super::*`), as when the module was here.
use crate::model::*;
use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
