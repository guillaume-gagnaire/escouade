//! No runtime: what the other binaries weigh without wasmtime.
use sha2::Digest;
fn main() {
    let path = std::env::args().nth(1).expect("a file");
    let bytes = std::fs::read(&path).unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or_default();
    println!("{v} {:x}", sha2::Sha256::digest(&bytes));
    if std::env::args().nth(2).is_some() {
        let _ = ureq::get("http://127.0.0.1:1/").call();
    }
}
