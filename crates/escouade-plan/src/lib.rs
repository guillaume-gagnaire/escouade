//! What an agent is getting through (`plan`: its tasks, subagents, workflows and background tasks, worked out from its
//! stream) and the plan files it follows (`planfiles`: superpowers' plans and ledgers, read from its folder). Cut out of
//! the app's crate for it to compile faster (docs/CRATES.md): the app names them `crate::plan` and `crate::planfiles`.

pub mod plan;
#[cfg(test)]
mod plan_later_tests;
#[cfg(test)]
mod plan_tests;
pub mod planfiles;
#[cfg(test)]
mod planfiles_tests;
