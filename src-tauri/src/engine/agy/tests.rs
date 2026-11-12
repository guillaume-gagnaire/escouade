use super::*;

#[test]
fn escouades_server_is_in_its_own_mcp_config_and_taken_out_after() {
    let dir = crate::paths::test_dir("esc270-agy-grant");
    // As its plugin's launch writes it (`grant_doc`), a server of the user's beside it.
    std::fs::create_dir_all(mcp_file(&dir).parent().unwrap()).unwrap();
    std::fs::write(
        mcp_file(&dir),
        r#"{"mcpServers":{"escouade":{"serverUrl":"http://127.0.0.1:1/mcp","headers":{"Authorization":"Bearer s3cr3t"}}},"other":1}"#,
    )
    .unwrap();
    let doc: Value =
        serde_json::from_str(&std::fs::read_to_string(mcp_file(&dir)).unwrap()).unwrap();
    let e = &doc["mcpServers"]["escouade"];
    assert_eq!(e["serverUrl"], "http://127.0.0.1:1/mcp");
    assert_eq!(e["headers"]["Authorization"], "Bearer s3cr3t");
    remove_grant(&dir);
    let after = std::fs::read_to_string(mcp_file(&dir)).unwrap();
    assert!(!after.contains("s3cr3t") && !after.contains("escouade"));
    assert!(after.contains("\"other\": 1"), "{after}");
}

#[test]
fn a_message_takes_text_files_and_refuses_an_image() {
    let file = |name: &str, media: &str, data: &str| Attachment {
        name: name.into(),
        media_type: media.into(),
        data: data.into(),
    };
    let m = message("Regarde", &[file("notes.txt", "text/plain", "bonjour")]).unwrap();
    assert_eq!(m["text"], "notes.txt:\n\nbonjour\n\nRegarde");
    assert!(message("x", &[file("a.png", "image/png", "iVBORw0KGgo=")]).is_err());
}
