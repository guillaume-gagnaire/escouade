//! The files an agent reads and writes through the client (`fs/read_text_file`, `fs/write_text_file`), on both targets.
//!
//! Natively, the client judges a path by `paths` and opens it. A module has no disk, and a path of the host (`C:\wt`) means
//! nothing to it: the host judges and opens, by `fs/*` (docs/PLUGINS.md §8.4.6), with the agent's key. It holds to the roots
//! the plugin declares and to the agent's fence, whatever the code here says, so that a module only keeps the refusals
//! apart: what an agent is told must stay what it was told.

use std::path::PathBuf;

/// Why a path is not opened.
#[derive(Debug, PartialEq, Eq)]
pub enum Refused {
    /// Not absolute, as ACP asks, or climbing over its root.
    NotAbsolute,
    /// In a private project.
    Fenced,
    /// Out of the folders the host gives this agent: a module only.
    #[cfg_attr(not(target_family = "wasm"), allow(dead_code))]
    OutOfReach,
}

/// Whether `path` is one of Windows, whatever this target is: a module runs on any system.
fn is_windows(path: &str) -> bool {
    let b = path.as_bytes();
    path.starts_with(r"\\")
        || (b.len() > 2
            && b[0].is_ascii_alphabetic()
            && b[1] == b':'
            && matches!(b[2], b'\\' | b'/'))
}

/// `path` without its `.` and `..`, by its text alone, in the separators of its system; None when it is not absolute or a
/// `..` climbs over its root. `paths::lexical` for a path that is not of the system it runs on.
#[cfg_attr(not(target_family = "wasm"), allow(dead_code))]
fn plain(path: &str) -> Option<String> {
    let windows = is_windows(path);
    if !windows && !path.starts_with('/') {
        return None;
    }
    let (root, rest, sep): (&str, &str, char) = if windows {
        match path.strip_prefix(r"\\") {
            Some(rest) => (r"\\", rest, '\\'),
            None => (&path[..3], &path[3..], '\\'),
        }
    } else {
        ("/", &path[1..], '/')
    };
    let split: &[char] = if windows { &['\\', '/'] } else { &['/'] };
    let mut parts: Vec<&str> = Vec::new();
    for part in rest.split(split) {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            part => parts.push(part),
        }
    }
    let root = if windows {
        root.replace('/', "\\")
    } else {
        root.to_string()
    };
    Some(format!("{root}{}", parts.join(&sep.to_string())))
}

pub use imp::{read, real_of, write};

#[cfg(not(target_family = "wasm"))]
mod imp {
    use super::{PathBuf, Refused};

    /// The real path of the file `path`, the one to open (`paths::resolve`).
    pub fn real_of(_agent: &str, path: &str, fence: &[PathBuf]) -> Result<PathBuf, Refused> {
        let real = crate::paths::resolve(path).ok_or(Refused::NotAbsolute)?;
        if crate::paths::within(&real, fence) {
            return Err(Refused::Fenced);
        }
        Ok(real)
    }

    /// The text of the file `real`, what is not UTF-8 replaced.
    pub fn read(_agent: &str, real: &std::path::Path) -> Result<String, String> {
        std::fs::read(real)
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .map_err(|e| e.to_string())
    }

    /// `content` in the file `real`, the folders it needs made.
    pub fn write(_agent: &str, real: &std::path::Path, content: &str) -> Result<(), String> {
        real.parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|_| std::fs::write(real, content))
            .map_err(|e| e.to_string())
    }
}

#[cfg(target_family = "wasm")]
mod imp {
    use super::{plain, PathBuf, Refused};
    use base64::Engine as _;
    use escouade_plugin_protocol::turn;
    use serde_json::json;

    /// What the host says of a path of the agent's private project (`files.rs` of the app).
    const FENCED: &str = "Projet privé";

    /// See the native `real_of`: the host's own judgement (`fs/canon`), as far as the path exists. The fence is the
    /// host's, not to be held here: `_fence` stays empty of what a module could compare.
    pub fn real_of(agent: &str, path: &str, _fence: &[PathBuf]) -> Result<PathBuf, Refused> {
        let plain = plain(path).ok_or(Refused::NotAbsolute)?;
        match turn::host_call("fs/canon", json!({ "agent": agent, "path": plain })) {
            Ok(v) => Ok(PathBuf::from(v["path"].as_str().unwrap_or(&plain))),
            Err(why) if why.contains(FENCED) => Err(Refused::Fenced),
            Err(_) => Err(Refused::OutOfReach),
        }
    }

    /// See the native `read`.
    pub fn read(agent: &str, real: &std::path::Path) -> Result<String, String> {
        let params = json!({ "agent": agent, "path": real.to_string_lossy() });
        let v = turn::host_call("fs/read", params)?;
        if let Some(text) = v["text"].as_str() {
            return Ok(text.to_string());
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(v["dataBase64"].as_str().unwrap_or_default())
            .map_err(|e| e.to_string())?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    /// See the native `write`: the host makes the folders.
    pub fn write(agent: &str, real: &std::path::Path, content: &str) -> Result<(), String> {
        let params = json!({ "agent": agent, "path": real.to_string_lossy(), "text": content });
        turn::host_call("fs/write", params).map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_of_the_host_is_read_whatever_the_target() {
        assert_eq!(
            plain(r"C:\wt\src\..\a.txt").as_deref(),
            Some(r"C:\wt\a.txt")
        );
        assert_eq!(plain("C:/wt/./a.txt").as_deref(), Some(r"C:\wt\a.txt"));
        assert_eq!(
            plain(r"\\srv\share\a\..\b").as_deref(),
            Some(r"\\srv\share\b")
        );
        assert_eq!(plain("/srv/wt/../a.txt").as_deref(), Some("/srv/a.txt"));
        // A backslash is a letter of a name under Unix.
        assert_eq!(plain(r"/srv/a\..\b").as_deref(), Some(r"/srv/a\..\b"));
    }

    #[test]
    fn a_relative_path_or_one_over_its_root_is_none() {
        for p in [
            "",
            "a.txt",
            "./a.txt",
            r"wt\a.txt",
            "C:a.txt",
            "/..",
            "/a/../..",
        ] {
            assert_eq!(plain(p), None, "{p}");
        }
        assert_eq!(plain(r"C:\..\a"), None);
    }

    #[test]
    fn a_file_is_refused_by_the_fence_and_goes_there_and_back() {
        let dir = std::env::temp_dir().join(format!("acp-files-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("sub").join("a.txt").to_string_lossy().into_owned();
        let real = real_of("a", &path, &[]).unwrap();
        write("a", &real, "écrit").unwrap();
        assert_eq!(read("a", &real).unwrap(), "écrit");
        let fence = crate::paths::fence_of(&[dir.to_string_lossy().into_owned()]);
        assert_eq!(real_of("a", &path, &fence), Err(Refused::Fenced));
        assert_eq!(real_of("a", "a.txt", &[]), Err(Refused::NotAbsolute));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
