//! The core holds to the protocol's schema (`schemas/plugin-protocol-1.json`, PLG-9, ESC-181), the
//! one source the SDK's TypeScript is generated from: its facts, its delays, its error codes, its
//! version, and the messages the core and its bench send (`initialize`, each fact, a decision). A
//! change on one side without the other fails here, or in the SDK's test of its generated types.

use crate::plugins::bench::{decision_request, fact, fake::Session, FACTS};
use crate::plugins::manifest;
use crate::plugins::runtime::{
    timeout_of, INVALID_PARAMS, KEYCHAIN, METHOD_NOT_FOUND, NOT_YOURS, PROTOCOL,
};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn schema() -> Value {
    let text = std::fs::read_to_string(root().join("schemas/plugin-protocol-1.json")).unwrap();
    serde_json::from_str(&text).unwrap()
}

/// What `value` breaks of `schema` (the subset the protocol's schema uses), by path.
fn check(value: &Value, schema: &Value, defs: &Value, path: &str, out: &mut Vec<String>) {
    if let Some(r) = schema["$ref"].as_str() {
        let name = r.trim_start_matches("#/definitions/");
        return check(value, &defs[name], defs, path, out);
    }
    if let Some(choices) = schema["enum"].as_array() {
        if !choices.contains(value) {
            out.push(format!("{path}: {value} is not one of {choices:?}"));
        }
    }
    let types: Vec<&str> = match &schema["type"] {
        Value::String(t) => vec![t.as_str()],
        Value::Array(ts) => ts.iter().filter_map(Value::as_str).collect(),
        _ => vec![],
    };
    let is = |t: &str| match t {
        "string" => value.is_string(),
        "integer" => value.is_i64() || value.is_u64(),
        "number" => value.is_number(),
        "boolean" => value.is_boolean(),
        "null" => value.is_null(),
        "array" => value.is_array(),
        "object" => value.is_object(),
        _ => true,
    };
    if !types.is_empty() && !types.iter().any(|t| is(t)) {
        out.push(format!("{path}: {value} is not {types:?}"));
        return;
    }
    if let Some(obj) = value.as_object() {
        for r in schema["required"].as_array().into_iter().flatten() {
            let r = r.as_str().unwrap_or_default();
            if !obj.contains_key(r) {
                out.push(format!("{path}.{r}: required"));
            }
        }
        if let Some(props) = schema["properties"].as_object() {
            for (k, v) in obj {
                match props.get(k) {
                    Some(p) => check(v, p, defs, &format!("{path}.{k}"), out),
                    None if schema["additionalProperties"] != json!(true) => {
                        out.push(format!("{path}.{k}: not in the schema"))
                    }
                    None => {}
                }
            }
        }
    }
}

pub(crate) fn holds(value: &Value, method: &str, part: &str) {
    let s = schema();
    let mut problems = Vec::new();
    check(
        value,
        &s["x-methods"][method][part],
        &s["definitions"],
        &format!("{method}.{part}"),
        &mut problems,
    );
    assert!(
        problems.is_empty(),
        "{method} breaks the protocol's schema: {problems:#?}"
    );
}

#[test]
fn the_facts_are_those_of_the_schema() {
    let s = schema();
    let named: Vec<&str> = s["definitions"]["FactName"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert_eq!(named, FACTS.to_vec());
}

#[test]
fn the_version_delays_and_errors_are_those_of_the_schema() {
    let s = schema();
    assert_eq!(s["x-protocolVersion"], json!(PROTOCOL));
    assert_eq!(PROTOCOL, manifest::API);
    for (method, m) in s["x-methods"].as_object().unwrap() {
        if m["direction"] != "toPlugin" || m["notification"] == json!(true) {
            continue;
        }
        let ms = m["timeoutMs"]
            .as_u64()
            .or(s["x-defaultTimeoutMs"].as_u64())
            .unwrap();
        assert_eq!(timeout_of(method).as_millis() as u64, ms, "{method}");
    }
    assert_eq!(
        timeout_of("anything/else").as_millis() as u64,
        s["x-defaultTimeoutMs"].as_u64().unwrap()
    );
    let errors = &s["x-errors"];
    assert_eq!(errors["methodNotFound"]["code"], json!(METHOD_NOT_FOUND));
    assert_eq!(errors["invalidParams"]["code"], json!(INVALID_PARAMS));
    assert_eq!(errors["notYours"]["code"], json!(NOT_YOURS));
    assert_eq!(errors["keychain"]["code"], json!(KEYCHAIN));
}

#[test]
fn what_the_core_sends_holds_the_schema() {
    let m = manifest::read(
        r#"{"id":"dev.test.schema","name":"Schéma","version":"1.0.0","escouade":{"api":"1","min":"2.1.0"}}"#,
        false,
    )
    .manifest
    .unwrap();
    holds(
        &Session::hello(&m, "2.1.0", PROTOCOL),
        "initialize",
        "params",
    );
    for name in FACTS {
        holds(&fact(name), "events/fact", "params");
    }
    holds(&decision_request(), "permissions/decide", "params");
    holds(&json!(null), "shutdown", "params");
}

#[test]
fn the_check_finds_what_breaks_the_schema() {
    let s = schema();
    let mut problems = Vec::new();
    check(
        &json!({ "name": "ticket.nope", "version": "1" }),
        &s["x-methods"]["events/fact"]["params"],
        &s["definitions"],
        "fact",
        &mut problems,
    );
    // A name it does not know, a version of another type, and `seq`, `at` and `data` missing.
    assert_eq!(problems.len(), 5, "{problems:#?}");
    for part in [
        "fact.name",
        "fact.version",
        "fact.seq",
        "fact.at",
        "fact.data",
    ] {
        assert!(
            problems.iter().any(|p| p.starts_with(part)),
            "{part}: {problems:#?}"
        );
    }
}

/// The site serves the schema at its `$id` (escouade.dev/schemas/plugin-protocol-1.json).
#[test]
fn the_site_publishes_the_same_schema() {
    let site =
        std::fs::read_to_string(root().join("website/public/schemas/plugin-protocol-1.json"))
            .unwrap_or_default();
    assert_eq!(
        serde_json::from_str::<Value>(&site).ok(),
        Some(schema()),
        "copy schemas/plugin-protocol-1.json to website/public/schemas/"
    );
}
