//! The data of Escouade, cut out of the app's crate for it to compile faster (docs/CRATES.md): the settings, the
//! projects, the tickets and the agents as they are saved and told to the window (`model`), and what they name (the
//! decisions and pages they cite, a discussion, a branch's CI, a review, a cache's heat, the models a harness lists).
//! Beside them, what every part of the app stands on without the core: the data folder and the paths (`paths`), the
//! processes (`proc`, `job`) and WSL (`wsl`). The app re-exports each where it was (`crate::model::…`, `crate::paths`,
//! `crate::docs::Ref`…): no caller changed. What names the app's own types (`UiEvent`, the quotas' snapshot) stays in
//! the app.

#[macro_use]
extern crate escouade_plugin_cli;

pub mod accounts;
pub mod agentgroups;
pub mod attachfiles;
pub mod branchci;
pub mod discussion;
pub mod docs;
pub mod docsplan;
pub mod fence;
pub mod heat;
pub mod job;
pub mod model;
pub mod notify;
pub mod paths;
pub mod pricing;
pub mod proc;
pub mod projectfile;
pub mod prreview;
pub mod registry;
pub mod repos;
pub mod tls;
pub mod workflowfix;
pub mod wsl;

/// The data of the code hosts the model names, shared with their plugins.
mod forge {
    pub(crate) use escouade_plugin_protocol::forge::{
        CheckState, MergeOptions, MergeStrategy, PrState, PullRequest, RepoRef,
    };
}

/// The languages, as the app's `i18n` gives them.
mod i18n {
    pub(crate) use escouade_base::i18n::{ui, Lang};
}
