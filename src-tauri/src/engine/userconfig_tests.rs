//! The server's entry in a user config Escouade edits itself (`userconfig`), as Codex declares it.

use super::userconfig::UserConfig;
use super::{Engine, CODEX};
use crate::mcp::install::{Entry, Standing};
use std::path::{Path, PathBuf};

fn codex() -> &'static UserConfig {
    CODEX.user_config().expect("Codex declares its config.toml")
}

fn entry(token: &str) -> Entry {
    Entry {
        url: "http://127.0.0.1:4100/mcp".into(),
        token: token.into(),
    }
}

/// A folder of its own for the test, empty.
fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("escouade-userconfig-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap()
}

/// What `text` becomes with the entry added, and that it is exactly `text` again once removed.
fn round_trip(name: &str, text: &str) -> String {
    round_trip_to(name, text, text)
}

/// `round_trip`, `text` becoming `after` once the entry is removed.
fn round_trip_to(name: &str, text: &str, after: &str) -> String {
    let file = dir(name).join("config.toml");
    std::fs::write(&file, text).unwrap();
    codex().add(&file, &entry("secret")).unwrap();
    let added = read(&file);
    assert!(
        matches!(codex().declared(&file), Standing::Declared(e) if e == entry("secret")),
        "{added}"
    );
    codex().remove(&file).unwrap();
    assert_eq!(read(&file), after, "removed, the file is as it was");
    assert!(matches!(codex().declared(&file), Standing::Absent));
    added
}

#[test]
fn the_entry_is_written_beside_the_users_own_comments_and_tables_and_leaves_them_as_they_were() {
    let text = "# mon modèle\nmodel = \"gpt-5\" # à côté\n\n[features]\njs_repl = false\n\n# mes serveurs\n[mcp_servers.node_repl]\ncommand = 'C:\\node.exe'\nargs = []\n";
    let added = round_trip("comments", text);
    assert!(
        added.starts_with(text),
        "the user's lines first, untouched:\n{added}"
    );
    assert!(
        added.contains("[mcp_servers.escouade]\nurl = \"http://127.0.0.1:4100/mcp\"\nhttp_headers = { Authorization = \"Bearer secret\" }"),
        "{added}"
    );
}

#[test]
fn a_file_with_crlf_or_empty_is_the_same_once_the_entry_leaves_and_one_with_no_last_newline_gains_it(
) {
    // Its last line ends where the entry began: Codex writes one anyway.
    round_trip_to("no-newline", "model = \"gpt-5\"", "model = \"gpt-5\"\n");
    round_trip(
        "crlf",
        "# un\r\nmodel = \"gpt-5\"\r\n\r\n[features]\r\njs_repl = false\r\n",
    );
    round_trip("empty", "");
    // A table of the servers with a line of its own stays, emptied.
    round_trip(
        "explicit",
        "[mcp_servers]\n\n[mcp_servers.autre]\nurl = \"http://x\"\n",
    );
}

#[test]
fn an_entry_of_that_name_is_read_whatever_its_form_and_one_that_is_not_ours_has_no_url_or_token() {
    let file = dir("forms").join("config.toml");
    let declared = |text: &str| {
        std::fs::write(&file, text).unwrap();
        match codex().declared(&file) {
            Standing::Declared(e) => Some(e),
            _ => None,
        }
    };
    let wanted = Some(entry("t"));
    assert_eq!(
        declared("[mcp_servers.escouade]\nurl = \"http://127.0.0.1:4100/mcp\"\nhttp_headers = { Authorization = \"Bearer t\" }\n"),
        wanted
    );
    assert_eq!(
        declared("[mcp_servers]\nescouade = { url = \"http://127.0.0.1:4100/mcp\", http_headers = { Authorization = \"Bearer t\" } }\n"),
        wanted
    );
    assert_eq!(
        declared("[mcp_servers.escouade]\ncommand = \"escouade\"\n"),
        Some(Entry {
            url: String::new(),
            token: String::new()
        })
    );
    assert_eq!(
        declared("[mcp_servers.escouade]\nurl = \"http://127.0.0.1:4100/mcp\"\nbearer_token_env_var = \"T\"\n"),
        Some(Entry { url: "http://127.0.0.1:4100/mcp".into(), token: String::new() })
    );
    assert_eq!(declared("[mcp_servers.autre]\nurl = \"http://x\"\n"), None);
    assert!(matches!(codex().declared(&file), Standing::Absent));
    std::fs::write(&file, "model = = 1").unwrap();
    assert!(matches!(codex().declared(&file), Standing::Unreadable));
    assert!(matches!(
        codex().declared(&file.with_file_name("none.toml")),
        Standing::NoFile
    ));
}

#[test]
fn an_entry_there_is_replaced_in_place_and_an_inline_table_of_servers_is_refused_untouched() {
    let file = dir("replace").join("config.toml");
    let text = "[mcp_servers.escouade]\n# le mien\ncommand = \"vieux\"\n\n[features]\nx = 1\n";
    std::fs::write(&file, text).unwrap();
    codex().add(&file, &entry("neuf")).unwrap();
    let added = read(&file);
    assert!(!added.contains("vieux"), "{added}");
    assert!(matches!(codex().declared(&file), Standing::Declared(e) if e == entry("neuf")));
    assert!(added.contains("[features]\nx = 1\n"), "{added}");

    let inline = "mcp_servers = { autre = { url = \"http://x\" } }\n";
    std::fs::write(&file, inline).unwrap();
    let e = codex().add(&file, &entry("t")).unwrap_err();
    assert!(format!("{e:#}").contains("mcp_servers"), "{e:#}");
    assert_eq!(read(&file), inline);
}

#[test]
fn a_file_that_is_no_toml_is_left_alone_and_said_so() {
    let file = dir("invalid").join("config.toml");
    std::fs::write(&file, "model = = 1\n").unwrap();
    let e = codex().add(&file, &entry("t")).unwrap_err();
    assert!(format!("{e:#}").contains("TOML"), "{e:#}");
    assert!(codex().remove(&file).is_err());
    assert_eq!(read(&file), "model = = 1\n");
}

#[test]
fn with_no_file_the_folder_and_the_file_are_made_and_removing_from_none_writes_nothing() {
    let root = dir("none");
    let file = root.join("compte").join("config.toml");
    codex().remove(&file).unwrap();
    assert!(!file.exists());
    codex().add(&file, &entry("t")).unwrap();
    assert!(matches!(codex().declared(&file), Standing::Declared(e) if e == entry("t")));
    codex().remove(&file).unwrap();
    assert_eq!(read(&file), "");
    // No entry: the file is not written again.
    std::fs::write(&file, "x = 1").unwrap();
    codex().remove(&file).unwrap();
    assert_eq!(read(&file), "x = 1");
}

#[test]
fn the_entry_to_paste_by_hand_is_the_one_written() {
    assert_eq!(
        CODEX.manual_declaration(true, "codex", None, &entry("t"), true),
        "[mcp_servers.escouade]\nurl = \"http://127.0.0.1:4100/mcp\"\nhttp_headers = { Authorization = \"Bearer t\" }"
    );
    assert_eq!(
        crate::mcp::install::mask_bearer(&codex().snippet(&entry("secret"))),
        "[mcp_servers.escouade]\nurl = \"http://127.0.0.1:4100/mcp\"\nhttp_headers = { Authorization = \"Bearer ••••••••\" }"
    );
}

#[cfg(unix)]
#[test]
fn a_config_that_is_a_link_stays_one_and_the_file_it_leads_to_is_the_one_written() {
    let root = dir("link");
    let principal = root.join("principal.toml");
    let text = "# le mien\nmodel = \"gpt-5\"\n";
    std::fs::write(&principal, text).unwrap();
    let account = root.join("compte").join("config.toml");
    std::fs::create_dir_all(account.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(&principal, &account).unwrap();
    let is_link = |p: &Path| {
        std::fs::symlink_metadata(p)
            .unwrap()
            .file_type()
            .is_symlink()
    };

    codex().add(&account, &entry("t")).unwrap();
    assert!(is_link(&account), "still a link");
    assert!(matches!(codex().declared(&principal), Standing::Declared(e) if e == entry("t")));
    assert_eq!(
        super::userconfig::same_file_key(&account),
        super::userconfig::same_file_key(&principal)
    );
    codex().remove(&account).unwrap();
    assert!(is_link(&account), "still a link");
    assert_eq!(read(&principal), text);
}

// ----- OpenCode and Copilot CLI: JSON with comments (ESC-119) -----

fn opencode() -> &'static UserConfig {
    super::OPENCODE
        .user_config()
        .expect("OpenCode declares its opencode.json")
}

fn copilot() -> &'static UserConfig {
    super::COPILOT
        .user_config()
        .expect("Copilot declares its mcp-config.json")
}

/// `round_trip` for a JSON user config: added, read back, and the file byte for byte once removed.
fn json_round_trip(user: &UserConfig, name: &str, file: &str, text: &str) -> String {
    let file = dir(name).join(file);
    std::fs::write(&file, text).unwrap();
    user.add(&file, &entry("secret")).unwrap();
    let added = read(&file);
    assert!(
        matches!(user.declared(&file), Standing::Declared(e) if e == entry("secret")),
        "{added}"
    );
    // What OpenCode and Copilot read: valid JSON with comments.
    escouade_engines::jsonc::parse(&added).unwrap();
    user.remove(&file).unwrap();
    assert_eq!(read(&file), text, "removed, the file is as it was");
    assert!(matches!(user.declared(&file), Standing::Absent));
    added
}

#[test]
fn opencodes_entry_goes_beside_the_users_servers_comments_and_order_kept() {
    let text = "{\n  // mon modèle\n  \"$schema\": \"https://opencode.ai/config.json\",\n  \"model\": \"ollama/qwen3\", /* à côté */\n  \"mcp\": {\n    \"autre\": { \"type\": \"local\", \"command\": [\"node\"] }, // le mien\n  },\n  \"provider\": {},\n}\n";
    let added = json_round_trip(opencode(), "oc-comments", "opencode.jsonc", text);
    assert!(
        added.contains("\"autre\": { \"type\": \"local\", \"command\": [\"node\"] },\n    \"escouade\": {\n      \"type\": \"remote\",\n      \"url\": \"http://127.0.0.1:4100/mcp\",\n      \"headers\": { \"Authorization\": \"Bearer secret\" }\n    }, // le mien\n  },"),
        "{added}"
    );
    assert!(
        added.starts_with("{\n  // mon modèle\n  \"$schema\""),
        "{added}"
    );
}

#[test]
fn with_no_object_of_the_servers_it_is_made_at_the_end_and_goes_with_the_entry() {
    // Tabs, CRLF, no newline at the end, a compact object.
    for (name, text) in [
        ("oc-tabs", "{\r\n\t\"model\": \"x\"\r\n}"),
        ("oc-compact", "{\"model\":\"x\",\"share\":\"disabled\"}"),
        ("oc-empty-object", "{}\n"),
        ("oc-comment-inside", "{ // rien\n}\n"),
        ("oc-trailing", "{\n  \"model\": \"x\",\n}\n"),
    ] {
        let added = json_round_trip(opencode(), name, "opencode.json", text);
        assert!(added.contains("\"mcp\": {"), "{name}: {added}");
    }
    let added = json_round_trip(
        opencode(),
        "oc-crlf",
        "opencode.json",
        "{\r\n\t\"model\": \"x\"\r\n}\r\n",
    );
    assert!(
        added.contains("\t\"model\": \"x\",\r\n\t\"mcp\": {\r\n\t\t\"escouade\": {\r\n\t\t\t\"type\": \"remote\""),
        "{added:?}"
    );
    assert!(!added.replace("\r\n", "").contains('\n'), "{added:?}");
}

#[test]
fn copilots_entry_is_the_one_copilot_mcp_add_writes_in_mcp_servers() {
    // As `copilot mcp add` leaves the file, with one server.
    let text = "{\n  \"mcpServers\": {\n    \"essai\": {\n      \"tools\": [\n        \"*\"\n      ],\n      \"type\": \"http\",\n      \"url\": \"http://127.0.0.1:1/mcp\"\n    }\n  }\n}";
    let added = json_round_trip(copilot(), "cp-one", "mcp-config.json", text);
    assert!(
        added.contains("    },\n    \"escouade\": {\n      \"type\": \"http\",\n      \"url\": \"http://127.0.0.1:4100/mcp\",\n      \"headers\": { \"Authorization\": \"Bearer secret\" }\n    }\n  }\n}"),
        "{added}"
    );
    // An empty object of the servers, as `copilot mcp remove` leaves it: emptied again.
    json_round_trip(
        copilot(),
        "cp-empty",
        "mcp-config.json",
        "{\n  \"mcpServers\": {}\n}",
    );
    // The entry first of two, put there by hand: the other one stays as it was.
    let file = dir("cp-first").join("mcp-config.json");
    std::fs::write(
        &file,
        "{\"mcpServers\": {\"escouade\": {\"url\": \"x\"}, \"b\": {}}}",
    )
    .unwrap();
    copilot().remove(&file).unwrap();
    assert_eq!(read(&file), "{\"mcpServers\": {\"b\": {}}}");
}

#[test]
fn with_no_file_one_is_made_and_goes_once_the_entry_leaves_and_an_entry_there_is_replaced_in_place()
{
    let file = dir("cp-none").join("compte").join("mcp-config.json");
    copilot().remove(&file).unwrap();
    copilot().add(&file, &entry("t")).unwrap();
    assert_eq!(
        read(&file),
        "{\n  \"mcpServers\": {\n    \"escouade\": {\n      \"type\": \"http\",\n      \"url\": \"http://127.0.0.1:4100/mcp\",\n      \"headers\": { \"Authorization\": \"Bearer t\" }\n    }\n  }\n}\n"
    );
    copilot().remove(&file).unwrap();
    assert!(!file.exists(), "the file Escouade made is gone");

    let text = "{\n  \"mcp\": {\n    \"escouade\": { \"type\": \"local\" }, // vieux\n    \"b\": {}\n  }\n}\n";
    std::fs::write(&file, text).unwrap();
    opencode().add(&file, &entry("neuf")).unwrap();
    let added = read(&file);
    assert!(matches!(opencode().declared(&file), Standing::Declared(e) if e == entry("neuf")));
    assert!(
        added.contains("    }, // vieux\n    \"b\": {}\n"),
        "{added}"
    );
}

#[test]
fn a_json_that_is_not_one_or_whose_servers_are_no_object_is_left_alone_and_said_so() {
    let file = dir("cp-invalid").join("mcp-config.json");
    for bad in ["{\"mcpServers\": ", "[1]", "{\"mcpServers\": []}"] {
        std::fs::write(&file, bad).unwrap();
        let e = copilot().add(&file, &entry("t")).unwrap_err();
        assert!(
            format!("{e:#}").contains("JSON") || format!("{e:#}").contains("mcpServers"),
            "{e:#}"
        );
        assert_eq!(read(&file), bad);
    }
    std::fs::write(&file, "{\"mcpServers\": ").unwrap();
    assert!(matches!(copilot().declared(&file), Standing::Unreadable));
    assert!(copilot().remove(&file).is_err());
}

#[test]
fn an_entry_of_that_name_is_read_from_json_and_one_not_ours_has_no_url_or_token() {
    let file = dir("cp-forms").join("mcp-config.json");
    let declared = |text: &str| {
        std::fs::write(&file, text).unwrap();
        match copilot().declared(&file) {
            Standing::Declared(e) => Some(e),
            _ => None,
        }
    };
    assert_eq!(
        declared("{/* c */ \"mcpServers\": {\"escouade\": {\"headers\": {\"Authorization\": \"Bearer t\"}, \"url\": \"http://127.0.0.1:4100/mcp\",},},}"),
        Some(entry("t"))
    );
    assert_eq!(
        declared("{\"mcpServers\": {\"escouade\": {\"command\": \"x\"}}}"),
        Some(Entry {
            url: String::new(),
            token: String::new()
        })
    );
    assert_eq!(declared("{\"mcpServers\": {}}"), None);
}

#[test]
fn the_json_to_paste_by_hand_is_the_file_add_makes_and_the_file_is_the_one_there() {
    assert_eq!(
        crate::mcp::install::mask_bearer(&opencode().snippet(&entry("secret"))),
        "{\n  \"mcp\": {\n    \"escouade\": {\n      \"type\": \"remote\",\n      \"url\": \"http://127.0.0.1:4100/mcp\",\n      \"headers\": { \"Authorization\": \"Bearer ••••••••\" }\n    }\n  }\n}"
    );
    let d = dir("oc-files");
    assert_eq!(opencode().file_in(&d), d.join("opencode.json"));
    std::fs::write(d.join("opencode.jsonc"), "{}").unwrap();
    assert_eq!(opencode().file_in(&d), d.join("opencode.jsonc"));
    std::fs::write(d.join("opencode.json"), "{}").unwrap();
    assert_eq!(opencode().file_in(&d), d.join("opencode.json"));
}
