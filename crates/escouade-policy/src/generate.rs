//! The files generated from the single list of the policies ([`crate::list`], TS-12): what an IT department deploys,
//! and the page of the site that tells it how.
//!
//! - Windows: `escouade.admx`, and its translations `fr-FR/escouade.adml` and `en-US/escouade.adml` (Group Policy,
//!   Intune's imported administrative templates).
//! - macOS: the manifest of the managed preferences (`ProfileManifests` format, read by ProfileCreator, iMazing Profile
//!   Editor), the custom schema of Jamf Pro, and an example `.mobileconfig`.
//! - Linux: the JSON schema of `/etc/escouade/policies/*.json`, and an example file.
//! - The site: `website/content/docs/{fr,en}/managed-policies.md`.
//!
//! `cargo run -p escouade-policy --example generate` writes them; a test checks the files of the repository are these
//! ([`files`]), so the list and what is deployed never diverge.

use std::fmt::Write as _;

use serde_json::{json, Map, Value};

use crate::list::{Category, Def, Form, Lang, Sort, Type, POLICIES};
use crate::APP_ID;

/// The registry key of the policies, as the ADMX names it (under HKLM and HKCU).
const REGISTRY_KEY: &str = r"Software\Policies\Escouade";
/// The date the manifest says it was last changed: fixed, so that generating twice gives the same bytes.
const MANIFEST_DATE: &str = "2026-11-15T00:00:00Z";
/// The repository, where the generated files are read.
const REPOSITORY: &str = "https://github.com/guillaume-gagnaire/escouade/blob/main";

/// Every generated file: its path from the repository's root, and its content.
pub fn files() -> Vec<(&'static str, String)> {
    vec![
        ("policies/windows/escouade.admx", admx()),
        ("policies/windows/fr-FR/escouade.adml", adml(Lang::Fr)),
        ("policies/windows/en-US/escouade.adml", adml(Lang::En)),
        (
            "policies/macos/dev.gagnaire.escouade.plist",
            profile_manifest(),
        ),
        ("policies/macos/dev.gagnaire.escouade.json", jamf_schema()),
        (
            "policies/macos/escouade-example.mobileconfig",
            mobileconfig(),
        ),
        (
            "policies/linux/escouade-policies.schema.json",
            linux_schema(),
        ),
        ("policies/linux/example.json", linux_example()),
        (
            "website/content/docs/fr/managed-policies.md",
            docs_page(Lang::Fr),
        ),
        (
            "website/content/docs/en/managed-policies.md",
            docs_page(Lang::En),
        ),
    ]
}

// ---------- shared ----------

fn xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// French guillemets hold a non-breaking space inside (the site's rule).
fn typeset(lang: Lang, s: &str) -> String {
    match lang {
        Lang::Fr => s.replace("« ", "«\u{a0}").replace(" »", "\u{a0}»"),
        Lang::En => s.to_owned(),
    }
}

fn sort_name(lang: Lang, sort: Sort) -> &'static str {
    match sort {
        Sort::Restrict => lang.pick("restreindre", "restrict"),
        Sort::Value => lang.pick("valeur", "value"),
        Sort::Sensitive => lang.pick("sensible", "sensitive"),
        Sort::Executable => lang.pick("exécutable", "executable"),
    }
}

fn type_name(lang: Lang, kind: Type) -> &'static str {
    match kind {
        Type::Boolean => lang.pick("booléen", "Boolean"),
        Type::List => lang.pick("liste", "list"),
        Type::String => lang.pick("chaîne", "string"),
        Type::Number => lang.pick("nombre", "number"),
        Type::Json => "JSON",
    }
}

/// The values a key may be given, as one line: `stable`, `beta`; for a Boolean that restricts, the one imposed.
fn values_line(lang: Lang, d: &Def) -> String {
    let code = |v: &[&str]| {
        v.iter()
            .map(|c| format!("`{c}`"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    match d.form {
        Form::Flag {
            restricted: Some(r),
        } => format!("`{r}` (`{}`)", u8::from(r)),
        Form::Flag { restricted: None } => "`true`, `false`".to_owned(),
        Form::Choice { choices, .. } | Form::Ceiling { choices } => code(choices),
        Form::Bound { universe: Some(u) } => lang.pick("parmi ", "among ").to_owned() + &code(u),
        Form::Number { min, max } => format!("{min}–{max}"),
        _ => String::new(),
    }
}

// ---------- Windows ----------

/// `escouade.admx`: one policy per key, for the machine and the user (`class="Both"`).
pub fn admx() -> String {
    let mut s = String::new();
    s.push_str(
        r#"<?xml version="1.0" encoding="utf-8"?>
<!-- Generated from crates/escouade-policy/src/list.rs (cargo run -p escouade-policy --example generate): do not edit. -->
<policyDefinitions xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" revision="1.0" schemaVersion="1.0" xmlns="http://schemas.microsoft.com/GroupPolicy/2006/07/PolicyDefinitions">
  <policyNamespaces>
    <target prefix="escouade" namespace="Escouade.Policies" />
  </policyNamespaces>
  <resources minRequiredRevision="1.0" />
  <supportedOn>
    <definitions>
      <definition name="SUPPORTED_ESCOUADE_2_3" displayName="$(string.SUPPORTED_ESCOUADE_2_3)" />
    </definitions>
  </supportedOn>
  <categories>
    <category name="escouade" displayName="$(string.escouade)" />
"#,
    );
    for c in Category::ALL {
        let _ = writeln!(
            s,
            r#"    <category name="{id}" displayName="$(string.cat_{id})">
      <parentCategory ref="escouade" />
    </category>"#,
            id = c.id()
        );
    }
    s.push_str("  </categories>\n  <policies>\n");
    for d in POLICIES {
        s.push_str(&admx_policy(d));
    }
    s.push_str("  </policies>\n</policyDefinitions>\n");
    s
}

fn admx_policy(d: &Def) -> String {
    let id = d.id();
    let head = |value_name: bool, presentation: bool| {
        let mut h = format!(
            r#"    <policy name="{id}" class="Both" displayName="$(string.{id})" explainText="$(string.{id}_Explain)""#
        );
        if presentation {
            let _ = write!(h, r#" presentation="$(presentation.{id})""#);
        }
        let _ = write!(h, r#" key="{REGISTRY_KEY}""#);
        if value_name {
            let _ = write!(h, r#" valueName="{}""#, d.key);
        }
        h.push_str(">\n");
        let _ = writeln!(
            h,
            r#"      <parentCategory ref="{}" />
      <supportedOn ref="SUPPORTED_ESCOUADE_2_3" />"#,
            d.category.id()
        );
        h
    };
    let body = match d.form {
        // A Boolean: « Activé » writes the value it imposes; « Désactivé » removes it.
        Form::Flag { restricted } => {
            let mut b = head(true, false);
            match restricted {
                Some(r) => {
                    let _ = writeln!(
                        b,
                        r#"      <enabledValue>
        <decimal value="{}" />
      </enabledValue>"#,
                        u8::from(r)
                    );
                }
                None => b.push_str(
                    r#"      <enabledValue>
        <decimal value="1" />
      </enabledValue>
      <disabledValue>
        <decimal value="0" />
      </disabledValue>
"#,
                ),
            }
            b
        }
        Form::Choice { choices, .. } | Form::Ceiling { choices } => {
            let mut b = head(false, true);
            let _ = writeln!(
                b,
                r#"      <elements>
        <enum id="{id}" valueName="{}" required="true">"#,
                d.key
            );
            for c in choices {
                let _ = writeln!(
                    b,
                    r#"          <item displayName="$(string.{id}_{c})">
            <value>
              <string>{c}</string>
            </value>
          </item>"#
                );
            }
            b.push_str("        </enum>\n      </elements>\n");
            b
        }
        Form::Bound { .. } | Form::Union => {
            let mut b = head(false, true);
            let _ = writeln!(
                b,
                r#"      <elements>
        <list id="{id}" key="{REGISTRY_KEY}\{}" valuePrefix="" />
      </elements>"#,
                d.key
            );
            b
        }
        Form::Text | Form::Json => {
            let mut b = head(false, true);
            let max = if d.form == Form::Json { 4096 } else { 1024 };
            let _ = writeln!(
                b,
                r#"      <elements>
        <text id="{id}" valueName="{}" required="true" maxLength="{max}" />
      </elements>"#,
                d.key
            );
            b
        }
        Form::Number { min, max } => {
            let mut b = head(false, true);
            let _ = writeln!(
                b,
                r#"      <elements>
        <decimal id="{id}" valueName="{}" required="true" minValue="{min}" maxValue="{max}" />
      </elements>"#,
                d.key
            );
            b
        }
    };
    body + "    </policy>\n"
}

/// The explanation of a policy, as gpedit shows it under its name.
fn explain(lang: Lang, d: &Def) -> String {
    let t = d.text(lang);
    let mut e = typeset(lang, t.help);
    let _ = write!(
        e,
        "\n\n{}{}",
        lang.pick("Sur le poste : ", "On the device: "),
        typeset(lang, t.locks)
    );
    let values = values_line(lang, d).replace('`', "");
    let _ = write!(
        e,
        "\n\n{} {} ({}{}{}).",
        lang.pick("Clé :", "Key:"),
        d.key,
        type_name(lang, d.kind()),
        if values.is_empty() { "" } else { ", " },
        values
    );
    let _ = write!(
        e,
        " {} {}.",
        lang.pick("Sorte :", "Sort:"),
        sort_name(lang, d.sort)
    );
    if d.sort == Sort::Restrict && matches!(d.form, Form::Flag { .. }) {
        e.push_str(lang.pick(
            " « Désactivé » retire la stratégie : le réglage redevient libre.",
            " “Disabled” removes the policy: the setting is free again.",
        ));
    }
    typeset(lang, &e)
}

/// `escouade.adml` in a language: the strings and the presentations of the ADMX.
pub fn adml(lang: Lang) -> String {
    let mut strings = String::new();
    let mut add = |id: &str, text: &str| {
        let _ = writeln!(strings, r#"      <string id="{id}">{}</string>"#, xml(text));
    };
    add("escouade", "Escouade");
    add(
        "SUPPORTED_ESCOUADE_2_3",
        lang.pick("Escouade 2.3 et plus", "Escouade 2.3 and later"),
    );
    for c in Category::ALL {
        add(&format!("cat_{}", c.id()), c.name(lang));
    }
    for d in POLICIES {
        let id = d.id();
        add(&id, &typeset(lang, d.text(lang).title));
        add(&format!("{id}_Explain"), &explain(lang, d));
        if let Some(choices) = match d.form {
            Form::Choice { choices, .. } | Form::Ceiling { choices } => Some(choices),
            _ => None,
        } {
            for c in choices {
                add(&format!("{id}_{c}"), c);
            }
        }
    }
    let mut presentations = String::new();
    for d in POLICIES {
        let id = d.id();
        let label = xml(&typeset(lang, d.text(lang).title));
        let control = match d.form {
            Form::Flag { .. } => continue,
            Form::Choice { .. } | Form::Ceiling { .. } => {
                format!(r#"<dropdownList refId="{id}" noSort="true">{label}</dropdownList>"#)
            }
            Form::Bound { .. } | Form::Union => {
                format!(r#"<listBox refId="{id}">{label}</listBox>"#)
            }
            Form::Text | Form::Json => format!(
                r#"<textBox refId="{id}">
          <label>{label}</label>
        </textBox>"#
            ),
            Form::Number { .. } => {
                format!(r#"<decimalTextBox refId="{id}">{label}</decimalTextBox>"#)
            }
        };
        let _ = writeln!(
            presentations,
            r#"      <presentation id="{id}">
        {control}
      </presentation>"#
        );
    }
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<!-- Generated from crates/escouade-policy/src/list.rs (cargo run -p escouade-policy --example generate): do not edit. -->
<policyDefinitionResources xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" revision="1.0" schemaVersion="1.0" xmlns="http://schemas.microsoft.com/GroupPolicy/2006/07/PolicyDefinitions">
  <displayName>Escouade</displayName>
  <description>{}</description>
  <resources>
    <stringTable>
{strings}    </stringTable>
    <presentationTable>
{presentations}    </presentationTable>
  </resources>
</policyDefinitionResources>
"#,
        xml(lang.pick(
            "Les stratégies d’Escouade pour les postes gérés.",
            "Escouade’s policies for managed devices."
        ))
    )
}

// ---------- macOS ----------

/// A JSON value as a property list's element, indented by `depth`.
fn plist_value(v: &Value, depth: usize) -> String {
    let pad = "  ".repeat(depth);
    match v {
        Value::Bool(true) => format!("{pad}<true/>\n"),
        Value::Bool(false) => format!("{pad}<false/>\n"),
        Value::Number(n) => format!("{pad}<integer>{n}</integer>\n"),
        Value::String(s) => format!("{pad}<string>{}</string>\n", xml(s)),
        Value::Array(a) if a.is_empty() => format!("{pad}<array/>\n"),
        Value::Array(a) => {
            let mut s = format!("{pad}<array>\n");
            for item in a {
                s.push_str(&plist_value(item, depth + 1));
            }
            s + &format!("{pad}</array>\n")
        }
        Value::Object(o) => {
            let mut s = format!("{pad}<dict>\n");
            for (k, item) in o {
                let _ = writeln!(s, "{pad}  <key>{}</key>", xml(k));
                s.push_str(&plist_value(item, depth + 1));
            }
            s + &format!("{pad}</dict>\n")
        }
        Value::Null => format!("{pad}<string/>\n"),
    }
}

fn plist(body: &Value) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<!-- Generated from crates/escouade-policy/src/list.rs (cargo run -p escouade-policy --example generate): do not edit. -->
<plist version="1.0">
{}</plist>
"#,
        plist_value(body, 0)
    )
}

fn pfm_type(kind: Type) -> &'static str {
    match kind {
        Type::Boolean => "boolean",
        Type::List => "array",
        Type::String => "string",
        Type::Number => "integer",
        Type::Json => "dictionary",
    }
}

/// The manifest of the managed preferences, in the ProfileManifests format (English: the tools that read it show it as
/// it is).
pub fn profile_manifest() -> String {
    let mut subkeys = vec![
        json!({"pfm_name": "PayloadType", "pfm_type": "string", "pfm_default": APP_ID, "pfm_require": "always", "pfm_title": "Payload Type"}),
        json!({"pfm_name": "PayloadVersion", "pfm_type": "integer", "pfm_default": 1, "pfm_require": "always", "pfm_title": "Payload Version"}),
        json!({"pfm_name": "PayloadIdentifier", "pfm_type": "string", "pfm_default": APP_ID, "pfm_require": "always", "pfm_title": "Payload Identifier"}),
        json!({"pfm_name": "PayloadUUID", "pfm_type": "string", "pfm_format": "^[0-9A-Za-z]{8}-[0-9A-Za-z]{4}-[0-9A-Za-z]{4}-[0-9A-Za-z]{4}-[0-9A-Za-z]{12}$", "pfm_require": "always", "pfm_title": "Payload UUID"}),
        json!({"pfm_name": "PayloadDisplayName", "pfm_type": "string", "pfm_default": "Escouade", "pfm_title": "Payload Display Name"}),
    ];
    for d in POLICIES {
        let mut k = Map::new();
        k.insert("pfm_name".into(), json!(d.key));
        k.insert("pfm_title".into(), json!(d.en.title));
        k.insert(
            "pfm_description".into(),
            json!(format!("{} On the device: {}", d.en.help, d.en.locks)),
        );
        k.insert("pfm_type".into(), json!(pfm_type(d.kind())));
        match d.form {
            Form::Flag {
                restricted: Some(r),
            } => {
                k.insert("pfm_range_list".into(), json!([r]));
            }
            Form::Choice { choices, .. } | Form::Ceiling { choices } => {
                k.insert("pfm_range_list".into(), json!(choices));
            }
            Form::Bound { .. } | Form::Union => {
                let mut item = json!({"pfm_name": "item", "pfm_type": "string"});
                if let Some(u) = d.form.choices() {
                    item["pfm_range_list"] = json!(u);
                }
                k.insert("pfm_subkeys".into(), json!([item]));
            }
            Form::Number { min, max } => {
                k.insert("pfm_range_min".into(), json!(min));
                k.insert("pfm_range_max".into(), json!(max));
            }
            _ => {}
        }
        subkeys.push(Value::Object(k));
    }
    plist(&json!({
        "pfm_app_url": "https://escouade.dev/",
        "pfm_description": "Escouade’s policies for managed devices: what the IT department imposes on the app.",
        "pfm_documentation_url": "https://escouade.dev/en/docs/managed-policies/",
        "pfm_domain": APP_ID,
        "pfm_format_version": 1,
        "pfm_last_modified": MANIFEST_DATE,
        "pfm_platforms": ["macOS"],
        "pfm_subkeys": subkeys,
        "pfm_title": "Escouade",
        "pfm_unique": true,
        "pfm_version": 1,
    }))
    // The date is a date, not a string.
    .replace(
        &format!("<string>{MANIFEST_DATE}</string>"),
        &format!("<date>{MANIFEST_DATE}</date>"),
    )
}

/// A key's JSON schema, for Jamf's custom schema and the one of `/etc`.
fn json_schema_of(d: &Def, lang: Lang) -> Value {
    let t = d.text(lang);
    let mut s = Map::new();
    s.insert("title".into(), json!(typeset(lang, t.title)));
    s.insert(
        "description".into(),
        json!(typeset(lang, &format!("{} {}", t.help, t.locks))),
    );
    match d.form {
        Form::Flag { restricted } => {
            s.insert("type".into(), json!("boolean"));
            if let Some(r) = restricted {
                s.insert("enum".into(), json!([r]));
            }
        }
        Form::Choice { choices, .. } | Form::Ceiling { choices } => {
            s.insert("type".into(), json!("string"));
            s.insert("enum".into(), json!(choices));
        }
        Form::Bound { .. } | Form::Union => {
            s.insert("type".into(), json!("array"));
            let mut items = json!({"type": "string"});
            if let Some(u) = d.form.choices() {
                items["enum"] = json!(u);
            }
            s.insert("items".into(), items);
        }
        Form::Text => {
            s.insert("type".into(), json!("string"));
        }
        Form::Number { min, max } => {
            s.insert("type".into(), json!("integer"));
            s.insert("minimum".into(), json!(min));
            s.insert("maximum".into(), json!(max));
        }
        Form::Json => {
            s.insert("type".into(), json!("object"));
            s.insert(
                "additionalProperties".into(),
                json!({"type": "array", "items": {"type": "string"}}),
            );
        }
    }
    Value::Object(s)
}

fn pretty(v: &Value) -> String {
    serde_json::to_string_pretty(v).expect("JSON") + "\n"
}

/// Jamf Pro's custom schema (Application & Custom Settings › External Applications).
pub fn jamf_schema() -> String {
    let properties: Map<String, Value> = POLICIES
        .iter()
        .map(|d| (d.key.to_owned(), json_schema_of(d, Lang::En)))
        .collect();
    pretty(&json!({
        "title": format!("Escouade ({APP_ID})"),
        "description": "Escouade’s policies for managed devices. Generated from crates/escouade-policy/src/list.rs: do not edit.",
        "properties": properties,
    }))
}

/// The values of the example profile and of the example file of `/etc`: each key's example.
fn examples() -> Map<String, Value> {
    POLICIES
        .iter()
        .map(|d| (d.key.to_owned(), d.example_value()))
        .collect()
}

/// An example configuration profile: every key with its example value, to adapt before installing it.
pub fn mobileconfig() -> String {
    let mut payload = Map::new();
    payload.insert("PayloadDisplayName".into(), json!("Escouade"));
    payload.insert(
        "PayloadIdentifier".into(),
        json!(format!("{APP_ID}.policies.example")),
    );
    payload.insert("PayloadType".into(), json!(APP_ID));
    payload.insert(
        "PayloadUUID".into(),
        json!("6C1D2F8E-3B4A-4E5F-9A0B-7C8D9E0F1A2B"),
    );
    payload.insert("PayloadVersion".into(), json!(1));
    payload.extend(examples());
    plist(&json!({
        "PayloadContent": [Value::Object(payload)],
        "PayloadDescription": "An example of Escouade’s policies: every key with an example value. Keep the keys you mean to impose, then install it.",
        "PayloadDisplayName": "Escouade: example policies",
        "PayloadIdentifier": format!("{APP_ID}.example"),
        "PayloadScope": "System",
        "PayloadType": "Configuration",
        "PayloadUUID": "0F4E8A21-5C7B-4D3E-8F9A-1B2C3D4E5F60",
        "PayloadVersion": 1,
    }))
}

// ---------- Linux ----------

/// The JSON schema of a file of `/etc/escouade/policies`.
pub fn linux_schema() -> String {
    let properties: Map<String, Value> = POLICIES
        .iter()
        .map(|d| (d.key.to_owned(), json_schema_of(d, Lang::En)))
        .collect();
    pretty(&json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://escouade.dev/schemas/policies.json",
        "title": "Escouade policies",
        "description": "A file of /etc/escouade/policies/*.json (root:root, 0644). Generated from crates/escouade-policy/src/list.rs: do not edit.",
        "type": "object",
        "additionalProperties": false,
        "properties": properties,
    }))
}

/// An example file of `/etc/escouade/policies`.
pub fn linux_example() -> String {
    pretty(&Value::Object(examples()))
}

// ---------- the page of the site ----------

/// `website/content/docs/<lang>/managed-policies.md`: what each policy does, how to deploy them, what the device shows.
pub fn docs_page(lang: Lang) -> String {
    let fr = lang == Lang::Fr;
    let mut s = String::new();
    let p = |s: &mut String, text: &str| {
        s.push_str(&typeset(lang, text));
        s.push('\n');
    };
    p(
        &mut s,
        lang.pick("# Stratégies du poste\n", "# Device policies\n"),
    );
    p(&mut s, lang.pick(
        "Un service informatique peut imposer des réglages d’Escouade sur les postes qu’il gère, par ses propres outils : les stratégies de groupe ou Intune sous Windows, un profil de configuration sous macOS, des fichiers de configuration sous Linux.\n",
        "An IT department can impose Escouade’s settings on the devices it manages, with its own tools: Group Policy or Intune on Windows, a configuration profile on macOS, configuration files on Linux.\n",
    ));
    p(&mut s, lang.pick(
        "> [!NOTE]\n> Cette page est générée depuis la liste des stratégies d’Escouade (`crates/escouade-policy/src/list.rs`), comme les fichiers qu’elle cite. Aucun serveur d’équipe n’est nécessaire.\n",
        "> [!NOTE]\n> This page is generated from Escouade’s list of policies (`crates/escouade-policy/src/list.rs`), as are the files it cites. No team server is needed.\n",
    ));
    p(
        &mut s,
        lang.pick("## Ce que fait une stratégie\n", "## What a policy does\n"),
    );
    p(&mut s, lang.pick(
        "- Une stratégie posée vaut pour toute l’app, dans tous les projets. Elle passe devant le réglage du poste, le `.escouade.yml` du dépôt et ce qu’une équipe recommande.\n\
- Le réglage qu’elle verrouille est grisé, avec un cadenas et la ligne « Imposé par la stratégie de ce poste ».\n\
- C’est le cœur d’Escouade qui refuse, pas seulement la fenêtre : une écriture par le serveur MCP ou par le `.escouade.yml` du dépôt est refusée aussi, avec la raison.\n\
- Chaque stratégie a sa sorte :\n  - **restreindre** : elle ne s’impose que dans un sens (les statistiques éteintes, jamais allumées). Une autre valeur est ignorée et dite ;\n  - **valeur** : elle pose une valeur (le mode par défaut, l’adresse du serveur d’équipe) ;\n  - **sensible** : l’adresse d’un proxy, un fichier d’autorités. Posée par le poste géré, elle s’applique ; venue d’une équipe, elle attendra l’accord de chaque membre.\n\
- Une stratégie changée pendant que l’app tourne s’applique en quelques secondes, sans redémarrer. Un agent dans un mode devenu interdit finit son tour, puis passe au mode permis le plus proche, et le dit dans sa conversation. Un agent d’un harness devenu interdit s’arrête à la fin de son tour.\n",
        "- A policy set holds for the whole app, in every project. It goes before the device’s setting, the repository’s `.escouade.yml` and what a team recommends.\n\
- The setting it locks is grayed, with a padlock and the line “Imposed by this device’s policy”.\n\
- Escouade’s core refuses, not only the window: a write through the MCP server or through the repository’s `.escouade.yml` is refused too, with the reason.\n\
- Each policy has its sort:\n  - **restrict**: it is imposed one way only (the statistics off, never on). Another value is ignored and said;\n  - **value**: it sets a value (the default mode, the team server’s address);\n  - **sensitive**: a proxy’s address, a file of authorities. Set by the managed device, it applies; from a team, it will wait for each member’s consent.\n\
- A policy changed while the app runs applies within a few seconds, without a restart. An agent in a mode no longer allowed ends its turn, then goes to the nearest allowed mode, and says so in its conversation. An agent of a harness no longer allowed stops at the end of its turn.\n",
    ));
    p(
        &mut s,
        lang.pick("## Les stratégies\n", "## The policies\n"),
    );
    p(&mut s, lang.pick(
        "Chaque stratégie a une clé, le même nom partout : la valeur du registre, la clé du profil, la clé du fichier JSON. Un booléen se donne `true` ou `false` (`1` ou `0` dans le registre). Une liste est une liste ADMX, un tableau du profil ou du JSON ; une chaîne seule vaut une liste d’un élément. Ce qui est « à venir » est lu et gardé, et s’appliquera avec la fonction qui le porte.\n",
        "Each policy has a key, the same name everywhere: the registry value, the profile’s key, the JSON file’s key. A Boolean is given `true` or `false` (`1` or `0` in the registry). A list is an ADMX list, an array of the profile or the JSON; a lone string counts as a list of one item. What is “to come” is read and kept, and will apply with the feature that carries it.\n",
    ));
    for c in Category::ALL {
        let _ = writeln!(s, "### {}\n", c.name(lang));
        s.push_str(lang.pick(
            "| Stratégie | Clé et type | Ce qu’elle fait | Sur le poste |\n| --- | --- | --- | --- |\n",
            "| Policy | Key and type | What it does | On the device |\n| --- | --- | --- | --- |\n",
        ));
        for d in POLICIES.iter().filter(|d| d.category == c) {
            let t = d.text(lang);
            let values = values_line(lang, d);
            let _ = writeln!(
                s,
                "| {} | `{}` · {}{}{} · {} | {} | {} |",
                typeset(lang, t.title),
                d.key,
                type_name(lang, d.kind()),
                if values.is_empty() { "" } else { " " },
                values,
                sort_name(lang, d.sort),
                typeset(lang, t.help),
                typeset(lang, t.locks),
            );
        }
        s.push('\n');
    }
    p(&mut s, lang.pick("## Déployer\n", "## Deploy\n"));
    p(
        &mut s,
        lang.pick(
            "### Windows : stratégies de groupe et Intune\n",
            "### Windows: Group Policy and Intune\n",
        ),
    );
    let admx_url = format!("{REPOSITORY}/policies/windows");
    if fr {
        p(&mut s, &format!(
            "1. Prends `escouade.admx` et ses traductions `fr-FR/escouade.adml` et `en-US/escouade.adml` dans [le dossier policies/windows du dépôt]({admx_url}).\n\
2. Pose-les dans `C:\\Windows\\PolicyDefinitions` (avec les dossiers `fr-FR` et `en-US`) pour un poste, ou dans le magasin central de ton domaine (`\\\\<domaine>\\SYSVOL\\<domaine>\\Policies\\PolicyDefinitions`).\n\
3. Dans l’éditeur de stratégie de groupe, les stratégies sont sous Modèles d’administration › Escouade, côté ordinateur comme côté utilisateur.\n\
4. Dans Intune : Appareils › Configuration › Importer un ADMX, puis un profil de type Modèles importés.\n\
\n\
Escouade lit `HKLM\\SOFTWARE\\Policies\\Escouade`, puis `HKCU\\SOFTWARE\\Policies\\Escouade` ; pour une même clé, la machine gagne. Sans l’ADMX, un script peut écrire le registre lui-même : la valeur porte le nom de la clé, un booléen est un `REG_DWORD` à `0` ou `1`, une chaîne un `REG_SZ`, un réglage JSON un `REG_SZ` qui commence par une accolade, et une liste une sous-clé du nom de la clé dont les valeurs s’appellent `1`, `2`, `3`…\n"
        ));
    } else {
        p(&mut s, &format!(
            "1. Take `escouade.admx` and its translations `fr-FR/escouade.adml` and `en-US/escouade.adml` from [the policies/windows folder of the repository]({admx_url}).\n\
2. Put them in `C:\\Windows\\PolicyDefinitions` (with the `fr-FR` and `en-US` folders) for one device, or in your domain’s central store (`\\\\<domain>\\SYSVOL\\<domain>\\Policies\\PolicyDefinitions`).\n\
3. In the Group Policy editor, the policies are under Administrative Templates › Escouade, for the computer as for the user.\n\
4. In Intune: Devices › Configuration › Import ADMX, then a profile of the Imported Administrative templates type.\n\
\n\
Escouade reads `HKLM\\SOFTWARE\\Policies\\Escouade`, then `HKCU\\SOFTWARE\\Policies\\Escouade`; for a same key, the machine wins. Without the ADMX, a script can write the registry itself: the value is named after the key, a Boolean is a `REG_DWORD` of `0` or `1`, a string a `REG_SZ`, a JSON setting a `REG_SZ` that opens with a brace, and a list a subkey named after the key whose values are named `1`, `2`, `3`…\n"
        ));
    }
    s.push_str(
        "```bat\n\
reg add \"HKLM\\SOFTWARE\\Policies\\Escouade\" /v telemetry.enabled /t REG_DWORD /d 0 /f\n\
reg add \"HKLM\\SOFTWARE\\Policies\\Escouade\\permissions.modes\" /v 1 /t REG_SZ /d plan /f\n\
reg add \"HKLM\\SOFTWARE\\Policies\\Escouade\\permissions.modes\" /v 2 /t REG_SZ /d default /f\n\
```\n\n",
    );
    p(&mut s, lang.pick(
        "> [!WARNING]\n> Escouade refuse une clé `Policies\\Escouade` que l’utilisateur pourrait écrire sans être administrateur : son ACL a été relâchée. Si l’app tourne en administrateur, elle ne peut plus faire cette différence et lit tout.\n",
        "> [!WARNING]\n> Escouade refuses a `Policies\\Escouade` key the user could write without being an administrator: its ACL was loosened. When the app runs as an administrator, it can no longer tell and reads everything.\n",
    ));
    p(
        &mut s,
        lang.pick(
            "### macOS : profil de configuration\n",
            "### macOS: configuration profile\n",
        ),
    );
    let mac_url = format!("{REPOSITORY}/policies/macos");
    if fr {
        p(&mut s, &format!(
            "Escouade lit le domaine de préférences gérées `{APP_ID}`. Seules les valeurs qu’un profil de configuration force comptent : un `defaults write` du même domaine est ignoré. [Le dossier policies/macos du dépôt]({mac_url}) donne :\n\
\n\
- `dev.gagnaire.escouade.plist` : le manifeste des clés au format ProfileManifests, que lisent ProfileCreator et iMazing Profile Editor ;\n\
- `dev.gagnaire.escouade.json` : le schéma pour Jamf Pro (Réglages d’applications et personnalisés › Applications externes › Schéma personnalisé) ;\n\
- `escouade-example.mobileconfig` : un profil d’exemple avec chaque clé, à réduire à ce que tu veux imposer avant de l’installer ou de l’envoyer par ton MDM (Jamf, Intune, Kandji).\n"
        ));
    } else {
        p(&mut s, &format!(
            "Escouade reads the managed preferences domain `{APP_ID}`. Only the values a configuration profile forces count: a `defaults write` of the same domain is ignored. [The policies/macos folder of the repository]({mac_url}) gives:\n\
\n\
- `dev.gagnaire.escouade.plist`: the manifest of the keys in the ProfileManifests format, which ProfileCreator and iMazing Profile Editor read;\n\
- `dev.gagnaire.escouade.json`: the schema for Jamf Pro (Application & Custom Settings › External Applications › Custom Schema);\n\
- `escouade-example.mobileconfig`: an example profile with every key, to cut down to what you mean to impose before installing it or sending it through your MDM (Jamf, Intune, Kandji).\n"
        ));
    }
    p(
        &mut s,
        lang.pick(
            "### Linux : /etc/escouade/policies\n",
            "### Linux: /etc/escouade/policies\n",
        ),
    );
    let linux_url = format!("{REPOSITORY}/policies/linux");
    if fr {
        p(&mut s, &format!(
            "Escouade lit les fichiers `/etc/escouade/policies/*.json`, par ordre de nom ; pour une même clé, le dernier gagne. Chaque fichier est un objet JSON de clés et de valeurs. Le dossier, ceux au-dessus et chaque fichier doivent être à root, sans écriture pour le groupe ni pour les autres ; un lien symbolique est refusé. Le schéma `escouade-policies.schema.json` et un fichier d’exemple sont dans [le dossier policies/linux du dépôt]({linux_url}).\n"
        ));
    } else {
        p(&mut s, &format!(
            "Escouade reads the files `/etc/escouade/policies/*.json`, in the order of their names; for a same key, the last one wins. Each file is a JSON object of keys and values. The folder, those above it and each file must belong to root, with no write for the group or the others; a symbolic link is refused. The schema `escouade-policies.schema.json` and an example file are in [the policies/linux folder of the repository]({linux_url}).\n"
        ));
    }
    s.push_str(
        "```sh\n\
sudo mkdir -p /etc/escouade/policies\n\
echo '{\"telemetry.enabled\": false, \"permissions.modes\": [\"plan\", \"default\"]}' \\\n  | sudo tee /etc/escouade/policies/10-company.json\n\
sudo chmod 644 /etc/escouade/policies/10-company.json\n\
```\n\n",
    );
    p(&mut s, lang.pick("## L’enrôlement\n", "## Enrollment\n"));
    p(&mut s, lang.pick(
        "Le poste géré peut poser l’adresse du serveur d’équipe de l’entreprise (`team.server`) et l’empreinte de son équipe (`team.fingerprint`) : le membre n’aura plus qu’à accepter son invitation. Avec `team.onlyThisServer`, l’app refuse tout autre serveur d’équipe ; avec `servers.allowed`, tout serveur hors de la liste. Aujourd’hui, l’app garde ces valeurs et tient le refus ; l’écran d’entrée dans l’équipe, qui les préremplira, vient avec le serveur d’équipe.\n",
        "The managed device can set the address of the company’s team server (`team.server`) and the fingerprint of its team (`team.fingerprint`): the member will only have to accept their invitation. With `team.onlyThisServer`, the app refuses any other team server; with `servers.allowed`, any server outside the list. Today, the app keeps these values and holds the refusal; the screen to join the team, which will fill them in, comes with the team server.\n",
    ));
    p(
        &mut s,
        lang.pick("## Ce que voit l’utilisateur\n", "## What the user sees\n"),
    );
    p(&mut s, lang.pick(
        "- Un réglage imposé est grisé, son contrôle inerte, avec un cadenas et « Imposé par la stratégie de ce poste ».\n\
- Dans une liste bornée (les modes, les harness), les choix non permis sont grisés, avec la raison dans leur bulle.\n\
- Une écriture refusée le dit : « Ce réglage est imposé par la stratégie de ce poste (telemetry.enabled) : il ne se change pas sur ce poste. »\n\
- Ce que la lecture n’a pas pu appliquer (une clé inconnue, une valeur mal formée, une clé de registre à l’ACL relâchée) est listé dans Réglages › Général › « Stratégies de ce poste ».\n",
        "- An imposed setting is grayed, its control inert, with a padlock and “Imposed by this device’s policy”.\n\
- In a bounded list (the modes, the harnesses), the choices not allowed are grayed, with the reason in their tooltip.\n\
- A refused write says so: “This setting is imposed by this device’s policy (telemetry.enabled): it cannot be changed on this device.”\n\
- What the reading could not apply (an unknown key, a malformed value, a registry key whose ACL was loosened) is listed in Settings › General › “Policies of this device”.\n",
    ));
    // One trailing newline.
    while s.ends_with("\n\n") {
        s.pop();
    }
    s
}

#[cfg(test)]
#[path = "generate_tests.rs"]
mod tests;
