//! The kit of the Rust plugins of Escouade's own hand (docs/PLUGINS.md §4.3, §4.5): what every one of them shares, so
//! that a plugin is its client and nothing more.
//!
//! - [`serve`]: the loop of a plugin in process, asynchronous: the calls are answered as they end, several at once,
//!   and the plugin asks Escouade too (`secrets/get`, `log`) while it answers.
//! - [`hosts`]: the service of the code hosts (`forge/*`) and of the Kanban's sources (`tickets/*`), and of their
//!   account (`account/me`): the account read from the call, its token asked by `secrets/get`, one client kept per
//!   account and token (a host's conditional reads and limits live in it), the typed errors as codes and the account's
//!   budget of calls in each answer (`{ value, rateLimit }`). A plugin gives its kinds ([`hosts::ForgeKind`],
//!   [`hosts::SourceKind`]); the rest is here.
//! - [`forge`]: the contract of a code host's client, which the core's adapters implement too.
//! - [`errors`], [`http`], [`text`], [`i18n`]: an HTTP error and what it says, the calls, the criteria of a ticket's
//!   description, the language of the interface.

pub mod errors;
pub mod forge;
pub mod hosts;
pub mod http;
pub mod i18n;
pub mod serve;
pub mod text;

pub use escouade_plugin_protocol as protocol;
