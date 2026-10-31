//! The demonstration plugin (`dev.escouade.minimal`): the least a plugin of the app is, which proves the way from the
//! registry to a process. It does the handshake and answers `minimal/echo` with what it is given and `minimal/hello`
//! with the version of Escouade it was told.

use escouade_plugin_protocol::{serve, Refusal};
use serde_json::json;

pub fn run() -> i32 {
    let version = std::env::var("ESCOUADE_VERSION").unwrap_or_default();
    let stdin = std::io::stdin();
    let result = serve(
        stdin.lock(),
        std::io::stdout().lock(),
        json!({}),
        |method, params| match method {
            "minimal/echo" => Ok(params),
            "minimal/hello" => Ok(json!({ "escouade": version })),
            other => Err(Refusal::not_found(other)),
        },
    );
    match result {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("escouade-plugins minimal: {e}");
            1
        }
    }
}
