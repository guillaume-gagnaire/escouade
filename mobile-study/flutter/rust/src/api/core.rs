//! ESC-445 : le Rust commun (mobile-study/shared/core) vu de Dart, par flutter_rust_bridge.
use escouade_mobile_core as c;

#[flutter_rust_bridge::frb(sync)]
pub fn core_version() -> String {
    c::core_version()
}

#[flutter_rust_bridge::frb(sync)]
pub fn seal(key: String, plain: String) -> Result<String, String> {
    c::seal(key, plain).map_err(|e| e.to_string())
}

#[flutter_rust_bridge::frb(sync)]
pub fn open(key: String, sealed: String) -> Result<String, String> {
    c::open(key, sealed).map_err(|e| e.to_string())
}

#[flutter_rust_bridge::frb(sync)]
pub fn bench_in_rust(n: u32, size: u32) -> u64 {
    c::bench_roundtrips(n, size)
}

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}
