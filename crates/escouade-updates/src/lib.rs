//! What the app learns when a request of the updater fails, and how it says it (`net`; docs/CRATES.md, § 12), and the
//! fake server its tests answer from (`fake_server`). The app re-exports both where they were (`updates::net`,
//! `updates::fake_server`); the updater itself (`updates.rs`) and the tests of its flow stay there.

#[macro_use]
extern crate escouade_plugin_cli;

/// A text in the interface's language: `tr!("français", "English", args…)`, as the app's.
macro_rules! tr {
    ($($t:tt)+) => {
        tr_in!(escouade_base::i18n::ui(), $($t)+)
    };
}

pub mod net;

// The fake server of the updates: the app's tests of the updater's flow answer from it too.
#[cfg(any(test, feature = "testing"))]
#[cfg_attr(not(test), allow(dead_code))]
pub mod fake_server;

#[cfg(test)]
mod net_tests;

/// The languages, as the app's `i18n` gives them.
mod i18n {
    pub use escouade_base::i18n::{ui, Lang};
}

use escouade_model::model;
use escouade_plugins::tls;

/// The fake services over https, as the app's tests have them.
#[cfg(test)]
use escouade_forge::integrations;

/// Each test process's temporary folder of its own, as the app's tests have it.
#[cfg(test)]
#[path = "../../../src-tauri/src/test_tmp.rs"]
mod test_tmp;
