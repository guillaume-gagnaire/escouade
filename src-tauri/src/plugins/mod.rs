//! The installed plugins (docs/PLUGINS-DESIGN.md §3.2): their host is `escouade-plugins` (the registry, the manifests, the
//! runtime of their processes and modules, the store, the seed, the bench), re-exported here where it was (docs/CRATES.md).
//! What links them to the core stays: the harnesses in process (`harness`, `shared`), the facts handed to them (`events`),
//! the accounts they serve and the adapters of the sources and code hosts (`accounts`, `adapters`, `hosted`), the terminal
//! views (`view`), the workflows they bring (`reader`), what the sort of the seed reads of the configuration (`sort`).

pub use escouade_plugins::*;

// The kinds of accounts, which plugin serves an account, its name in the keychain (INT-6).
pub mod accounts;
// The adapters of the sources and the code hosts in process (INT-5).
pub mod adapters;
// The plugins of the first hand, and the code hosts they serve to the tests.
pub mod builtin;
// The facts handed to the plugins, their webhooks and their commands (PLG-5).
pub mod events;
/// The adapter of a harness in process (INT-9).
pub mod harness;
// A source on a code host's instances, one account each, served by its plugin (GitLab Issues, INT-8.2).
pub mod hosted;
// The reader of workflows a plugin brings (ESC-200).
pub mod reader;
pub mod shared;
pub mod sort;
// The terminal views through the modules of their harnesses (ESC-352).
pub mod view;
