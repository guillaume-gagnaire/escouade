//! The code hosts and the ticket sources (docs/CRATES.md, § 12): the contract of a code host and its accounts (`forge`),
//! the accounts of the sources, their secrets in the keychain and the registry of the sources (`integrations`), the
//! plugins' side of them (`plugins`: the kinds of accounts, the adapters of a plugin that serves a host or a source). The
//! app re-exports each where it was (`forge::*`, `integrations::*`, `plugins::{accounts, adapters, builtin, hosted}`);
//! what reads the core stays there: the forms and the health of the hosts (`forge::connect`), the sync of the sources
//! (`integrations::sync`), the texts the sync writes (`integrations::text`).

#[macro_use]
extern crate escouade_plugin_cli;

/// A text in the interface's language: `tr!("français", "English", args…)`, as the app's.
macro_rules! tr {
    ($($t:tt)+) => {
        tr_in!(escouade_base::i18n::ui(), $($t)+)
    };
}

pub mod forge;
pub mod integrations;

/// The plugins' side of the hosts and the sources, beside the host of the plugins (`escouade-plugins`), as the app's
/// `plugins` has them.
pub mod plugins {
    pub use escouade_plugins::*;

    // The kinds of accounts, which plugin serves an account, its name in the keychain (INT-6).
    pub mod accounts;
    // The adapters of the sources and the code hosts in process (INT-5).
    pub mod adapters;
    // The plugins of the first hand, and the code hosts they serve to the tests.
    pub mod builtin;
    // A source on a code host's instances, one account each, served by its plugin (GitLab Issues, INT-8.2).
    pub mod hosted;
}

/// The languages, as the app's `i18n` gives them.
mod i18n {
    pub use escouade_plugins::i18n::{kit, ui, Lang};
}

/// The data folder and the paths, as the app's `paths` gives them.
mod paths {
    #[cfg(test)]
    pub use escouade_base::testing::test_dir;
    pub use escouade_model::paths::*;
}

use escouade_model::model;

// The tests of the plugins of the ticket sources and of their adapters (those that start no `core`), on the fake plugins of
// the host's tests. Those whose helpers the app's tests of parity take too (a fake board, a fake list) are compiled with the
// feature `testing` as well, their tests left out.
#[cfg(test)]
mod plugins_adapters_tests;
#[cfg(any(test, feature = "testing"))]
#[cfg_attr(not(test), allow(dead_code, unused_imports))]
pub mod plugins_clickup_tests;
#[cfg(test)]
mod plugins_jira_tests;
#[cfg(test)]
mod plugins_linear_tests;
#[cfg(any(test, feature = "testing"))]
#[cfg_attr(not(test), allow(dead_code, unused_imports))]
pub mod plugins_trello_tests;
#[cfg(test)]
use escouade_plugins::plugins_runtime_tests;

/// Each test process's temporary folder of its own, as the app's tests have it.
#[cfg(test)]
#[path = "../../../src-tauri/src/test_tmp.rs"]
mod test_tmp;
