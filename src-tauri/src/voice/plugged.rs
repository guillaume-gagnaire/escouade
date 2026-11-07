//! The voice services of the tests: the real plugins, modules that `scripts/plugins-wasm.sh` built, on one runtime for the
//! whole test program, each reaching the fake services of the tests by their addresses. `service::of` answers from here, so
//! that a test of the dictation, of a stream or of a reading plays what the app plays.

use super::online::Provider;
use super::plugin_host::ProcessVoiceService;
use super::service::VoiceService;
use crate::integrations::secrets::MemoryStore;
use crate::paths::DataDir;
use crate::plugins::runtime::{Limits, Runtime};
use crate::plugins::wasm::Net;
use crate::plugins::Plugins;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

const VERSION: &str = "2.2.0";

fn app_dir() -> std::path::PathBuf {
    let exe = std::env::current_exe().unwrap();
    exe.parent().unwrap().parent().unwrap().to_path_buf()
}

/// A runtime whose plugins reach any port of this machine: a route with no port, which only a test has.
pub(crate) fn runtime(name: &str) -> Arc<Runtime> {
    let dir = crate::engine::bench::bench_dir(&format!("int13-{name}"));
    let data = DataDir::new(dir.join("data"));
    let plugins = Plugins::load_in(data.clone(), VERSION, app_dir());
    let rt = Runtime::new(
        plugins,
        Arc::new(MemoryStore::default()),
        &data,
        VERSION,
        Limits {
            initialize: Duration::from_secs(20),
            ..Limits::default()
        },
    );
    rt.set_net(Net {
        routes: HashMap::from([("127.0.0.1".to_string(), "http://127.0.0.1".to_string())]),
        ..Net::default()
    });
    rt
}

/// The service `p` of `rt`, as its plugin serves it.
pub(crate) async fn through(rt: &Arc<Runtime>, p: Provider) -> Arc<ProcessVoiceService> {
    ProcessVoiceService::connect(rt.clone(), &format!("dev.escouade.{}", p.id()))
        .await
        .unwrap_or_else(|e| panic!("{}: {e}", p.id()))
}

/// The service `p`, its plugin started once for this test program.
pub(crate) fn plugged(p: Provider) -> Arc<dyn VoiceService> {
    static RUNTIME: OnceLock<Arc<Runtime>> = OnceLock::new();
    let rt = RUNTIME.get_or_init(|| runtime("plugged")).clone();
    // On a thread and a runtime of its own: a test may be on one that it would block.
    std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(through(&rt, p))
    })
    .join()
    .expect("the plugin started")
}
