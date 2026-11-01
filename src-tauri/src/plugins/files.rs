//! `files/read` (ESC-321, WASM-2): a file of the plugin's own folder, as text, for the SDK's `p.file(path)`, served
//! the same to a plugin in WASM and to a process (schemas/plugin-protocol-1.json).
//!
//! Only inside the folder: a relative path, `/` between its parts, no `..`, nothing hidden (`.env`, `.git`), not the
//! manifest; and, links followed, still inside the folder. 256 KiB at most, UTF-8.

use serde_json::Value;
use std::path::Path;

/// A path the plugin may not read: outside its folder, hidden, its manifest (as `net/fetch`'s refusals).
pub const REFUSED: i64 = -32003;
/// A file too big, or not text.
pub const FAILED: i64 = -32004;
/// No such file in its folder.
pub const NOT_FOUND: i64 = -32005;
/// The most a file may hold.
pub const FILE_BYTES: u64 = 256 << 10;

/// `{ path }` → the file's text, or why not.
pub fn read(folder: &Path, params: &Value) -> Result<Value, (i64, String)> {
    let Some(path) = params.get("path").and_then(Value::as_str) else {
        return Err((
            super::runtime::INVALID_PARAMS,
            "files/read takes a path".into(),
        ));
    };
    let parts = parts_of(path).ok_or_else(|| {
        (
            REFUSED,
            format!("{path:?} is not a file of the plugin's folder"),
        )
    })?;
    if parts.len() == 1 && parts[0] == super::manifest::FILE {
        return Err((
            REFUSED,
            format!("{path:?}: the manifest is not read by its plugin"),
        ));
    }
    let not_found = || {
        (
            NOT_FOUND,
            format!("no file {path:?} in the plugin's folder"),
        )
    };
    let root = std::fs::canonicalize(folder).map_err(|_| not_found())?;
    let target = parts.iter().fold(root.clone(), |p, s| p.join(s));
    let real = std::fs::canonicalize(&target).map_err(|_| not_found())?;
    // A link that leads out of the folder is outside it.
    if !real.starts_with(&root) {
        return Err((
            REFUSED,
            format!("{path:?} leads out of the plugin's folder"),
        ));
    }
    let meta = std::fs::metadata(&real).map_err(|_| not_found())?;
    if !meta.is_file() {
        return Err(not_found());
    }
    if meta.len() > FILE_BYTES {
        return Err((FAILED, format!("{path:?} is over 256 KiB")));
    }
    let bytes = std::fs::read(&real).map_err(|e| (FAILED, format!("{path:?}: {e}")))?;
    String::from_utf8(bytes)
        .map(Value::String)
        .map_err(|_| (FAILED, format!("{path:?} is not text (UTF-8)")))
}

/// Its parts, when it names a file of the folder: relative, `/` (or `\`) between them, none empty, `.`, `..` or
/// hidden, no drive nor colon.
fn parts_of(path: &str) -> Option<Vec<&str>> {
    if path.is_empty() || path.starts_with(['/', '\\']) || path.contains(['\0', ':']) {
        return None;
    }
    let parts: Vec<&str> = path.split(['/', '\\']).collect();
    parts
        .iter()
        .all(|p| !p.is_empty() && !p.starts_with('.'))
        .then_some(parts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn folder() -> std::path::PathBuf {
        let base =
            std::env::temp_dir().join(format!("plugin-files-{}", uuid::Uuid::new_v4().simple()));
        let f = base.join("plugin");
        std::fs::create_dir_all(f.join("data")).unwrap();
        std::fs::create_dir_all(f.join(".git")).unwrap();
        std::fs::write(f.join("rules.json"), r#"{"a":1}"#).unwrap();
        std::fs::write(f.join("data/words.txt"), "un\ndeux\n").unwrap();
        std::fs::write(f.join(".env"), "TOKEN=x").unwrap();
        std::fs::write(f.join(".git/config"), "x").unwrap();
        std::fs::write(f.join("escouade-plugin.json"), "{}").unwrap();
        std::fs::write(f.join("big.txt"), "x".repeat(FILE_BYTES as usize + 1)).unwrap();
        std::fs::write(f.join("bin.dat"), [0xff, 0xfe, 0x00]).unwrap();
        std::fs::write(base.join("secret.txt"), "outside").unwrap();
        f
    }

    fn code(f: &Path, path: &str) -> i64 {
        read(f, &json!({ "path": path })).unwrap_err().0
    }

    #[test]
    fn reads_a_file_of_its_folder_and_of_a_subfolder() {
        let f = folder();
        assert_eq!(
            read(&f, &json!({ "path": "rules.json" })).unwrap(),
            json!(r#"{"a":1}"#)
        );
        assert_eq!(
            read(&f, &json!({ "path": "data/words.txt" })).unwrap(),
            json!("un\ndeux\n")
        );
        assert_eq!(
            read(&f, &json!({ "path": "data\\words.txt" })).unwrap(),
            json!("un\ndeux\n")
        );
    }

    #[test]
    fn refuses_any_path_that_leaves_its_folder_or_is_hidden() {
        let f = folder();
        for p in [
            "../secret.txt",
            "data/../../secret.txt",
            "./rules.json",
            "/etc/passwd",
            "\\Windows\\win.ini",
            "C:/Windows/win.ini",
            "C:rules.json",
            ".env",
            ".git/config",
            "data//words.txt",
            "",
            "escouade-plugin.json",
        ] {
            assert_eq!(code(&f, p), REFUSED, "{p}");
        }
        let abs = f.join("rules.json").to_string_lossy().to_string();
        assert_eq!(code(&f, &abs), REFUSED, "{abs}");
        assert_eq!(
            read(&f, &json!({})).unwrap_err().0,
            super::super::runtime::INVALID_PARAMS
        );
    }

    #[test]
    fn says_a_file_missing_too_big_or_not_text() {
        let f = folder();
        assert_eq!(code(&f, "nope.json"), NOT_FOUND);
        assert_eq!(code(&f, "data"), NOT_FOUND);
        assert_eq!(code(&f, "big.txt"), FAILED);
        assert_eq!(code(&f, "bin.dat"), FAILED);
    }

    #[cfg(unix)]
    #[test]
    fn refuses_a_link_that_leads_out_of_its_folder() {
        let f = folder();
        std::os::unix::fs::symlink(f.join("../secret.txt"), f.join("out.txt")).unwrap();
        std::os::unix::fs::symlink(f.join("rules.json"), f.join("in.json")).unwrap();
        assert_eq!(code(&f, "out.txt"), REFUSED);
        assert_eq!(
            read(&f, &json!({ "path": "in.json" })).unwrap(),
            json!(r#"{"a":1}"#)
        );
    }
}
