//! OpenCode's events are translated by its plugin (`dev.escouade.opencode`, INT-10.2), which the core reads as items
//! (`agent/items.rs`). What stays here is what the translations still in the core share of it.

pub(super) use escouade_plugin_opencode::{absolute, patch_file};

#[cfg(test)]
#[path = "opencode_tests.rs"]
mod tests;
