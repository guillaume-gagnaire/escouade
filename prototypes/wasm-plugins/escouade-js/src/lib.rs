//! Escouade's Javy plugin (ESC-319, prototype): the QuickJS of Javy 9.1, with the event loop, TextEncoder and
//! `Javy.IO`, plus one export, `escouade-turn`.
//!
//! Javy's own exports evaluate the plugin's module again at each call (and QuickJS never frees a module): a plugin
//! that lives as long as Escouade would grow without end. `escouade-turn` instead calls `globalThis.__esc.pump()`,
//! which the prelude set when the module was evaluated, once, by `_start`; then runs the jobs it queued (promises)
//! until there are none. The plugin's state stays from a turn to the next, as in its process before.
use javy_plugin_api::javy::quickjs::{Context, Function, Object};
use javy_plugin_api::javy::Runtime;
use javy_plugin_api::{import_namespace, Config};
use std::cell::RefCell;

import_namespace!("escouade-js-v1");

thread_local! {
    static CONTEXT: RefCell<Option<Context>> = const { RefCell::new(None) };
}

fn config() -> Config {
    let mut config = Config::default();
    config.text_encoding(true).javy_stream_io(true);
    config.event_loop(true);
    config
}

fn modify_runtime(runtime: Runtime) -> Runtime {
    CONTEXT.with_borrow_mut(|c| *c = Some(runtime.context().clone()));
    runtime
}

#[unsafe(export_name = "initialize-runtime")]
fn initialize_runtime() {
    javy_plugin_api::initialize_runtime(config, modify_runtime).unwrap();
}

/// A turn: the lines Escouade wrote go to the plugin, then every job they queued runs. A JS error ends the instance
/// (Escouade makes a new one), as a process that crashes.
#[unsafe(export_name = "escouade-turn")]
fn turn() {
    let ok = CONTEXT.with_borrow(|c| {
        let Some(context) = c.as_ref() else {
            return Err("not initialized".to_string());
        };
        context.with(|ctx| {
            let esc: Object = ctx.globals().get("__esc").map_err(|e| e.to_string())?;
            let pump: Function = esc.get("pump").map_err(|e| e.to_string())?;
            if let Err(e) = pump.call::<_, ()>(()) {
                let caught = ctx.catch();
                return Err(format!("{e}: {caught:?}"));
            }
            while ctx.execute_pending_job() {}
            Ok(())
        })
    });
    if let Err(e) = ok {
        eprintln!("{e}");
        std::process::abort();
    }
}
