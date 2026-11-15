use super::*;
use crate::list::{def, POLICIES};
use std::collections::BTreeSet;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn the_files_of_the_repository_are_what_the_list_generates() {
    // The test of non-divergence: a key added to the list, or a text changed, without generating again fails here.
    let mut stale = Vec::new();
    for (path, content) in files() {
        let on_disk = std::fs::read_to_string(root().join(path))
            .unwrap_or_default()
            .replace("\r\n", "\n");
        if on_disk != content {
            stale.push(path);
        }
    }
    assert!(
        stale.is_empty(),
        "generated again by `cargo run -p escouade-policy --example generate`: {stale:?}"
    );
}

/// An element of an XML document: its name, its attributes and its text.
type Element = (String, Vec<(String, String)>, String);

/// The elements of an XML document, each with its attributes; panics when it is not well formed.
fn elements(text: &str) -> Vec<Element> {
    use quick_xml::events::Event;
    let mut reader = quick_xml::Reader::from_str(text);
    let mut out = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    let element = |e: &quick_xml::events::BytesStart| {
        let name = e.name().as_ref().to_owned();
        let attrs = e
            .attributes()
            .map(|a| {
                let a = a.expect("an attribute");
                (
                    a.key.as_ref().to_owned(),
                    a.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                        .unwrap()
                        .into_owned(),
                )
            })
            .collect();
        (name, attrs, String::new())
    };
    loop {
        match reader.read_event().expect("well-formed XML") {
            Event::Start(e) => {
                out.push(element(&e));
                stack.push(out.len() - 1);
            }
            Event::Empty(e) => out.push(element(&e)),
            Event::Text(t) => {
                if let Some(&i) = stack.last() {
                    out[i].2.push_str(&t.xml10_content());
                }
            }
            Event::End(_) => {
                stack.pop();
            }
            Event::Eof => break,
            _ => {}
        }
    }
    out
}

fn attr<'a>(attrs: &'a [(String, String)], name: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.as_str())
}

#[test]
fn the_admx_has_one_policy_per_key_written_where_the_reader_looks() {
    let admx = admx();
    let els = elements(&admx);
    let policies: Vec<_> = els.iter().filter(|(n, _, _)| n == "policy").collect();
    assert_eq!(policies.len(), POLICIES.len());
    for (d, (_, attrs, _)) in POLICIES.iter().zip(&policies) {
        assert_eq!(attr(attrs, "name"), Some(d.id().as_str()));
        assert_eq!(attr(attrs, "class"), Some("Both"));
        assert_eq!(attr(attrs, "key"), Some(r"Software\Policies\Escouade"));
    }
    // A Boolean writes a REG_DWORD named by its key; a list, a subkey named by its key with the values 1, 2…
    let telemetry = policies
        .iter()
        .find(|(_, a, _)| attr(a, "name") == Some("telemetry_enabled"))
        .unwrap();
    assert_eq!(attr(&telemetry.1, "valueName"), Some("telemetry.enabled"));
    let lists: Vec<_> = els.iter().filter(|(n, _, _)| n == "list").collect();
    assert!(lists.iter().any(|(_, a, _)| attr(a, "key")
        == Some(r"Software\Policies\Escouade\permissions.modes")
        && attr(a, "valuePrefix") == Some("")));
    // An enum's values are the list's choices.
    let channel: Vec<&str> = els
        .iter()
        .filter(|(n, _, _)| n == "string")
        .map(|(_, _, t)| t.as_str())
        .collect();
    assert!(channel.contains(&"stable") && channel.contains(&"beta"));
}

#[test]
fn each_adml_holds_every_string_and_presentation_the_admx_names_and_nothing_more() {
    let admx = admx();
    let named = |prefix: &str| -> BTreeSet<String> {
        admx.match_indices(prefix)
            .map(|(i, _)| {
                let rest = &admx[i + prefix.len()..];
                rest[..rest.find(')').unwrap()].to_owned()
            })
            .collect()
    };
    let strings = named("$(string.");
    let presentations = named("$(presentation.");
    for lang in [Lang::Fr, Lang::En] {
        let adml = adml(lang);
        let els = elements(&adml);
        let ids = |tag: &str| -> BTreeSet<String> {
            els.iter()
                .filter(|(n, _, _)| n == tag)
                .filter_map(|(_, a, _)| attr(a, "id").map(str::to_owned))
                .collect()
        };
        let have = ids("string");
        for s in &strings {
            assert!(have.contains(s), "{lang:?}: string {s} missing");
        }
        assert_eq!(ids("presentation"), presentations, "{lang:?}");
        // Every control of a presentation refers to an element of the ADMX.
        for (n, a, _) in &els {
            if let Some(r) = attr(a, "refId") {
                assert!(admx.contains(&format!(r#"id="{r}""#)), "{lang:?}: {n} {r}");
            }
        }
    }
    // The French one is in French, with its non-breaking spaces inside the guillemets.
    let fr = adml(Lang::Fr);
    assert!(fr.contains("Interdire le mode Bypass"));
    assert!(fr.contains("«\u{a0}Autoriser le mode Bypass\u{a0}»"));
    assert!(adml(Lang::En).contains("Deny the Bypass mode"));
}

#[test]
fn the_example_profile_is_a_property_list_with_every_key_at_its_example() {
    let profile: plist::Value =
        plist::from_bytes(mobileconfig().as_bytes()).expect("a property list");
    let top = profile.as_dictionary().unwrap();
    assert_eq!(
        top.get("PayloadType").and_then(|v| v.as_string()),
        Some("Configuration")
    );
    let payload = top["PayloadContent"].as_array().unwrap()[0]
        .as_dictionary()
        .unwrap();
    assert_eq!(
        payload.get("PayloadType").and_then(|v| v.as_string()),
        Some(crate::APP_ID)
    );
    for d in POLICIES {
        let v: serde_json::Value = serde_json::to_value(&payload[d.key]).unwrap();
        // What macOS gives the reader is what the list says it may be.
        assert_eq!(
            crate::list::convert(d, &v),
            Ok(d.example_value()),
            "{}",
            d.key
        );
    }
}

#[test]
fn the_manifest_names_every_key_with_its_type() {
    let manifest: plist::Value =
        plist::from_bytes(profile_manifest().as_bytes()).expect("a property list");
    let top = manifest.as_dictionary().unwrap();
    assert_eq!(
        top.get("pfm_domain").and_then(|v| v.as_string()),
        Some(crate::APP_ID)
    );
    assert!(top.get("pfm_last_modified").unwrap().as_date().is_some());
    let subkeys = top["pfm_subkeys"].as_array().unwrap();
    for d in POLICIES {
        let k = subkeys
            .iter()
            .filter_map(|s| s.as_dictionary())
            .find(|s| s.get("pfm_name").and_then(|n| n.as_string()) == Some(d.key))
            .unwrap_or_else(|| panic!("{} in the manifest", d.key));
        assert_eq!(
            k.get("pfm_type").and_then(|t| t.as_string()),
            Some(pfm_type(d.kind())),
            "{}",
            d.key
        );
    }
}

#[test]
fn the_schemas_of_jamf_and_of_etc_take_the_examples_and_refuse_what_only_widens() {
    for text in [linux_schema(), jamf_schema()] {
        let schema: serde_json::Value = serde_json::from_str(&text).unwrap();
        let props = schema["properties"].as_object().unwrap();
        assert_eq!(props.len(), POLICIES.len());
        for d in POLICIES {
            let p = &props[d.key];
            let t = match d.kind() {
                crate::list::Type::Boolean => "boolean",
                crate::list::Type::List => "array",
                crate::list::Type::String => "string",
                crate::list::Type::Number => "integer",
                crate::list::Type::Json => "object",
            };
            assert_eq!(p["type"], t, "{}", d.key);
        }
        // A flag that restricts takes its restricted value alone.
        assert_eq!(
            props["telemetry.enabled"]["enum"],
            serde_json::json!([false])
        );
        assert_eq!(
            props["permissions.modes"]["items"]["enum"][0],
            serde_json::json!("plan")
        );
    }
    let example: serde_json::Value = serde_json::from_str(&linux_example()).unwrap();
    assert_eq!(example["team.onlyThisServer"], serde_json::json!(true));
    assert_eq!(
        def("proxy.url").unwrap().example_value(),
        example["proxy.url"]
    );
}

#[test]
fn the_page_of_the_site_tells_each_key_in_both_languages_the_same_way() {
    let (fr, en) = (docs_page(Lang::Fr), docs_page(Lang::En));
    for d in POLICIES {
        let code = format!("`{}`", d.key);
        assert!(fr.contains(&code) && en.contains(&code), "{}", d.key);
    }
    let headings = |s: &str| s.lines().filter(|l| l.starts_with('#')).count();
    assert_eq!(headings(&fr), headings(&en));
    assert!(fr.starts_with("# Stratégies du poste\n\n"));
    assert!(en.starts_with("# Device policies\n\n"));
    // The site's rules: no straight quote in the prose of the French page outside the code, guillemets with their spaces.
    assert!(!fr.contains("« ") && !fr.contains(" »"));
    assert!(fr.ends_with(".\n") && !fr.ends_with("\n\n"));
}
