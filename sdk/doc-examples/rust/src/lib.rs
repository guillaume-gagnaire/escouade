//! The examples of the documentation, in Rust. Each `example/*` method runs one of them, for `escouade-plugin dev`.

mod files;
mod net;
mod procs;
mod settings;
mod socket;
mod timers;

use escouade_plugin_kit::forge::BoxFuture;
use escouade_plugin_kit::serve::{self, Cx, Plugin, Refusal};
use serde_json::{json, Value};

struct Examples;

impl Plugin for Examples {
    fn call<'a>(
        &'a self,
        cx: &'a Cx,
        method: &'a str,
        params: Value,
    ) -> BoxFuture<'a, Result<Value, Refusal>> {
        Box::pin(async move {
            match method {
                // What the host sends by itself comes here, like any call.
                "net/chunk" => net::chunk(cx, &params),
                "net/end" => net::end(cx, &params),
                "ws/message" => socket::message(cx, &params),
                "ws/closed" => socket::closed(cx, &params),
                "proc/out" => procs::out(cx, &params),
                "proc/exit" => procs::exit(cx, &params),
                "events/fact" => {
                    settings::journal(cx, &params);
                    return Ok(Value::Null);
                }
                "example/fetch" => return net::issue(cx, &text(&params)).await,
                "example/stream" => return net::follow(cx, &text(&params)).await,
                "example/socket" => return socket::open(cx).await,
                "example/close" => return socket::close(cx, &params).await,
                "example/later" => return Ok(timers::later(cx).await),
                "example/patient" => return Ok(json!(timers::patient(cx).await?)),
                "example/keep" => return files::keep(cx, &text(&params)).await,
                "example/notes" => return files::notes(cx).await,
                "example/words" => return files::words(cx).await,
                "example/status" => return procs::status(cx, &text(&params)).await,
                "example/input" => return procs::feed(cx, &params).await,
                "example/kill" => return procs::stop(cx, &params).await,
                "example/settings" => return settings::channel(cx, &params).await,
                "example/secret" => return Ok(json!(settings::token(cx).await?)),
                other => return Err(Refusal::not_found(other)),
            }
            Ok(Value::Null)
        })
    }
}

/// The one string a method of the examples takes.
fn text(params: &Value) -> String {
    params.as_str().unwrap_or_default().to_string()
}

pub fn run() -> i32 {
    serve::run("doc-examples", Examples)
}

escouade_plugin_kit::protocol::module!(run);
