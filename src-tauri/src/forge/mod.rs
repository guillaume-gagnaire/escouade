//! Code hosts (GitHub, GitLab, Bitbucket Cloud, Gitea and Forgejo, Azure DevOps): their contract, their accounts and
//! their clients are in `escouade-forge` (docs/CRATES.md), re-exported here where they were. What reads the core stays:
//! the forms and the health of the hosts (`connect`), and the tests that start a `core`.

pub use escouade_forge::forge::*;

pub mod connect;
#[cfg(test)]
mod tests;

// What `connect` and the tests take of the module (`use super::*`), as when the module was here.
use anyhow::{anyhow, Result};
use serde::Serialize;
use serde_json::Value;
use std::sync::Arc;
