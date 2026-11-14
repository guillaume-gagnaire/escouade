//! The installed plugins (docs/PLUGINS-DESIGN.md §3.2): their host is `escouade-plugins` (the registry, the manifests, the
//! runtime of their processes and modules, the store, the seed, the bench), re-exported here where it was (docs/CRATES.md).
//! The accounts they serve and the adapters of the sources and code hosts (`accounts`, `adapters`, `hosted`) are in
//! `escouade-forge`. What links them to the core stays: the harnesses in process (`harness`, `shared`), the facts handed
//! to them (`events`), the terminal views (`view`), the workflows they bring (`reader`), what the sort of the seed reads
//! of the configuration (`sort`).

pub use escouade_plugins::*;

// The kinds of accounts, the adapters of the sources and the code hosts in process, the plugins of the first hand and the
// code hosts they serve to the tests, a source on a code host's instances: in `escouade-forge`, with the hosts and the sources.
pub use escouade_forge::plugins::{accounts, adapters, builtin, hosted};

// The facts handed to the plugins, their webhooks and their commands (PLG-5).
pub mod events;
/// The adapter of a harness in process (INT-9).
pub mod harness;
// The reader of workflows a plugin brings (ESC-200).
pub mod reader;
pub mod shared;
pub mod sort;
// The terminal views through the modules of their harnesses (ESC-352).
pub mod view;
// The attack tests of the security audit of the 2.2.0 (2026-11, ESC-418): kept with its report, all of them ignored
// (`--run-ignored ignored-only -E 'test(audit_2_2_0)'`).
#[cfg(test)]
#[path = "../../../docs/security/audit-2026-11/poc/whitebox/audit_2_2_0.rs"]
mod audit_2_2_0;
