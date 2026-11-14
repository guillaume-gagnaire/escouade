//! Linux: `/etc/escouade/policies/*.json`, as `/etc/opt/chrome/policies/managed`.
//!
//! - Each file is a JSON object, of keys to values (`{"telemetry.enabled": false}`). The files are read in their
//!   names' order, and the last wins key by key.
//! - Read only from a folder and files that root alone may change: owned by root, writable by neither the group nor
//!   the others, and the same for every folder above it. A file that is not so, or that is a symbolic link, is
//!   refused and said; a folder that is not so is not read at all.
//! - A file that is not a JSON object is left out and said; so is a `null` value.
//!
//! The parsing is apart from the file system ([`from_contents`]), and the folder, its owner and how far up it is
//! checked are given ([`Folder`]), so that the tests need no root.

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::{Policies, Source, Warning};

/// The policies' folder.
pub const POLICY_DIR: &str = "/etc/escouade/policies";

/// A policy file larger than this is no policy file.
pub const MAX_FILE_LEN: u64 = 1 << 20;

/// The policies of files already read, given in the order they apply (the last wins).
pub fn from_contents(files: &[(PathBuf, String)], known: &[&str]) -> Policies {
    let mut policies = Policies::default();
    for (path, text) in files {
        read_file(path, text, known, &mut policies);
    }
    policies
}

fn read_file(path: &Path, text: &str, known: &[&str], policies: &mut Policies) {
    let object = match serde_json::from_str::<Value>(text) {
        Ok(Value::Object(object)) => object,
        Ok(_) => {
            policies.warn(Warning::Unreadable {
                place: path.display().to_string(),
                reason: "a policy file is a JSON object of keys to values".to_owned(),
            });
            return;
        }
        Err(e) => {
            policies.warn(Warning::Unreadable {
                place: path.display().to_string(),
                reason: format!("not valid JSON: {e}"),
            });
            return;
        }
    };
    for (key, value) in object {
        let source = Source::File(path.to_owned());
        if value.is_null() {
            policies.warn(Warning::Malformed {
                key,
                source,
                reason: "null is not a policy's value".to_owned(),
            });
        } else {
            policies.set(&key, value, source, known);
        }
    }
}

/// The files' names to read in a folder, in the order they apply: `*.json`, the hidden ones left aside (an editor's
/// copies), sorted by name.
pub fn policy_names(mut names: Vec<String>) -> Vec<String> {
    names.retain(|name| name.ends_with(".json") && !name.starts_with('.'));
    names.sort();
    names
}

#[cfg(unix)]
pub use system::*;

#[cfg(unix)]
mod system {
    use std::fs;
    use std::io;
    use std::os::unix::fs::MetadataExt;
    use std::path::Path;

    use super::{from_contents, policy_names, MAX_FILE_LEN};
    use crate::{Policies, Warning};

    /// A folder of policy files, whom it must belong to, and up to which folder its parents are checked too.
    #[derive(Clone, Copy, Debug)]
    pub struct Folder<'a> {
        pub dir: &'a Path,
        /// root (0) on a real system; the tests' own user in their temporary folder.
        pub owner: u32,
        /// The highest folder checked above `dir` (`/` on a real system).
        pub checked_up_to: &'a Path,
    }

    /// The policies of this machine: `/etc/escouade/policies`, root's alone.
    #[cfg(not(target_os = "macos"))]
    pub fn read_system(known: &[&str]) -> Policies {
        read_folder(
            Folder {
                dir: Path::new(super::POLICY_DIR),
                owner: 0,
                checked_up_to: Path::new("/"),
            },
            known,
        )
    }

    /// The policies of a folder's files, from its trusted files alone.
    pub fn read_folder(folder: Folder, known: &[&str]) -> Policies {
        let mut warnings = Vec::new();
        let files = trusted_files(folder, &mut warnings);
        let mut policies = from_contents(&files, known);
        for warning in warnings {
            policies.warn(warning);
        }
        policies
    }

    fn trusted_files(
        folder: Folder,
        warnings: &mut Vec<Warning>,
    ) -> Vec<(std::path::PathBuf, String)> {
        let place = |path: &Path| path.display().to_string();
        match fs::symlink_metadata(folder.dir) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Vec::new(),
            Err(e) => {
                warnings.push(Warning::Unreadable {
                    place: place(folder.dir),
                    reason: e.to_string(),
                });
                return Vec::new();
            }
            Ok(meta) => {
                if let Err(reason) = trusted(&meta, folder.owner, true) {
                    warnings.push(Warning::Refused {
                        place: place(folder.dir),
                        reason,
                    });
                    return Vec::new();
                }
            }
        }
        let mut above = folder.dir.parent();
        while let Some(dir) = above {
            match fs::metadata(dir) {
                Ok(meta) => {
                    if let Err(reason) = trusted(&meta, folder.owner, true) {
                        warnings.push(Warning::Refused {
                            place: place(folder.dir),
                            reason: format!("a folder above it, {}: {reason}", dir.display()),
                        });
                        return Vec::new();
                    }
                }
                Err(e) => {
                    warnings.push(Warning::Unreadable {
                        place: place(dir),
                        reason: e.to_string(),
                    });
                    return Vec::new();
                }
            }
            if dir == folder.checked_up_to {
                break;
            }
            above = dir.parent();
        }

        let entries = match fs::read_dir(folder.dir) {
            Ok(entries) => entries,
            Err(e) => {
                warnings.push(Warning::Unreadable {
                    place: place(folder.dir),
                    reason: e.to_string(),
                });
                return Vec::new();
            }
        };
        let names: Vec<String> = entries
            .filter_map(|e| e.ok()?.file_name().into_string().ok())
            .collect();
        let mut files = Vec::new();
        for name in policy_names(names) {
            let path = folder.dir.join(&name);
            // Not followed: a link is refused, whatever it points to.
            let meta = match fs::symlink_metadata(&path) {
                Ok(meta) => meta,
                Err(e) => {
                    warnings.push(Warning::Unreadable {
                        place: place(&path),
                        reason: e.to_string(),
                    });
                    continue;
                }
            };
            if let Err(reason) = trusted(&meta, folder.owner, false) {
                warnings.push(Warning::Refused {
                    place: place(&path),
                    reason,
                });
                continue;
            }
            if meta.len() > MAX_FILE_LEN {
                warnings.push(Warning::Refused {
                    place: place(&path),
                    reason: format!("larger than {MAX_FILE_LEN} bytes"),
                });
                continue;
            }
            match fs::read_to_string(&path) {
                Ok(text) => files.push((path, text)),
                Err(e) => warnings.push(Warning::Unreadable {
                    place: place(&path),
                    reason: e.to_string(),
                }),
            }
        }
        files
    }

    /// Whether a folder (`dir`) or a regular file belongs to `owner` and no one else may write it.
    fn trusted(meta: &fs::Metadata, owner: u32, dir: bool) -> Result<(), String> {
        let kind = meta.file_type();
        if dir && !kind.is_dir() {
            return Err(if kind.is_symlink() {
                "a symbolic link".to_owned()
            } else {
                "not a folder".to_owned()
            });
        }
        if !dir && !kind.is_file() {
            return Err(if kind.is_symlink() {
                "a symbolic link".to_owned()
            } else {
                "not a regular file".to_owned()
            });
        }
        if meta.uid() != owner {
            return Err(format!("owned by user {}, not {owner}", meta.uid()));
        }
        let mode = meta.mode();
        if mode & 0o022 != 0 {
            return Err(format!(
                "writable by its group or by others (mode {:o})",
                mode & 0o7777
            ));
        }
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use std::fs;
        use std::os::unix::fs::{symlink, PermissionsExt};

        use serde_json::json;

        use super::*;
        use crate::{Source, PROVISIONAL_KEYS};

        fn chmod(path: &Path, mode: u32) {
            fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
        }

        /// A policies folder of the tests' own user, checked up to the temporary folder that holds it.
        fn setup() -> (tempfile::TempDir, std::path::PathBuf, u32) {
            let root = tempfile::tempdir().unwrap();
            chmod(root.path(), 0o755);
            let dir = root.path().join("policies");
            fs::create_dir(&dir).unwrap();
            chmod(&dir, 0o755);
            let uid = fs::metadata(&dir).unwrap().uid();
            (root, dir, uid)
        }

        fn write(dir: &Path, name: &str, text: &str, mode: u32) {
            let path = dir.join(name);
            fs::write(&path, text).unwrap();
            chmod(&path, mode);
        }

        fn folder<'a>(root: &'a tempfile::TempDir, dir: &'a Path, owner: u32) -> Folder<'a> {
            Folder {
                dir,
                owner,
                checked_up_to: root.path(),
            }
        }

        #[test]
        fn files_are_read_by_name_and_the_last_wins_key_by_key() {
            let (root, dir, uid) = setup();
            write(
                &dir,
                "20-team.json",
                r#"{"telemetry.enabled": true, "proxy.url": "http://b"}"#,
                0o644,
            );
            write(
                &dir,
                "10-base.json",
                r#"{"telemetry.enabled": false, "voice.dictation": false}"#,
                0o644,
            );
            write(&dir, "notes.txt", "not read", 0o644);
            write(&dir, ".30-swap.json", "{", 0o644);

            let policies = read_folder(folder(&root, &dir, uid), PROVISIONAL_KEYS);

            assert_eq!(
                policies.get("telemetry.enabled").unwrap().value,
                json!(true)
            );
            assert_eq!(
                policies.get("telemetry.enabled").unwrap().source,
                Source::File(dir.join("20-team.json"))
            );
            assert_eq!(
                policies.get("voice.dictation").unwrap().source,
                Source::File(dir.join("10-base.json"))
            );
            assert_eq!(policies.get("proxy.url").unwrap().value, json!("http://b"));
            assert!(policies.warnings().is_empty(), "{:?}", policies.warnings());
        }

        #[test]
        fn a_file_others_may_change_is_refused_and_said() {
            let (root, dir, uid) = setup();
            write(&dir, "10-ok.json", r#"{"telemetry.enabled": false}"#, 0o644);
            write(
                &dir,
                "20-world.json",
                r#"{"voice.dictation": false}"#,
                0o666,
            );
            write(
                &dir,
                "30-group.json",
                r#"{"proxy.url": "http://evil"}"#,
                0o664,
            );
            symlink(dir.join("10-ok.json"), dir.join("40-link.json")).unwrap();

            let policies = read_folder(folder(&root, &dir, uid), PROVISIONAL_KEYS);

            assert_eq!(policies.len(), 1);
            assert!(policies.get("telemetry.enabled").is_some());
            let refused: Vec<&str> = policies
                .warnings()
                .iter()
                .filter_map(|w| match w {
                    Warning::Refused { place, .. } => Some(place.as_str()),
                    _ => None,
                })
                .collect();
            assert_eq!(refused.len(), 3, "{:?}", policies.warnings());
            assert!(refused[0].ends_with("20-world.json"));
            assert!(refused[1].ends_with("30-group.json"));
            assert!(refused[2].ends_with("40-link.json"));
        }

        #[test]
        fn files_not_owned_by_root_are_refused_on_a_real_system() {
            let (root, dir, uid) = setup();
            if uid == 0 {
                return; // run as root, the files are root's
            }
            write(&dir, "10.json", r#"{"telemetry.enabled": false}"#, 0o644);
            let policies = read_folder(folder(&root, &dir, 0), PROVISIONAL_KEYS);
            assert!(policies.is_empty());
            assert!(
                matches!(policies.warnings(), [Warning::Refused { reason, .. }] if reason.contains("owned by user"))
            );
        }

        #[test]
        fn a_folder_others_may_change_is_not_read_at_all() {
            let (root, dir, uid) = setup();
            write(&dir, "10.json", r#"{"telemetry.enabled": false}"#, 0o644);
            chmod(&dir, 0o777);
            let policies = read_folder(folder(&root, &dir, uid), PROVISIONAL_KEYS);
            assert!(policies.is_empty());
            assert!(matches!(policies.warnings(), [Warning::Refused { .. }]));

            // Nor one under a folder others may change: they could put another in its place.
            chmod(&dir, 0o755);
            chmod(root.path(), 0o777);
            let policies = read_folder(folder(&root, &dir, uid), PROVISIONAL_KEYS);
            assert!(policies.is_empty());
            assert!(
                matches!(policies.warnings(), [Warning::Refused { reason, .. }] if reason.contains("above"))
            );
            chmod(root.path(), 0o755);
        }

        #[test]
        fn malformed_files_and_values_are_left_out_and_said() {
            let (root, dir, uid) = setup();
            write(&dir, "10-bad.json", "{ not json", 0o644);
            write(&dir, "20-array.json", "[1, 2]", 0o644);
            write(
                &dir,
                "30-ok.json",
                r#"{"telemetry.enabled": false, "proxy.url": null, "bad key": 1}"#,
                0o644,
            );

            let policies = read_folder(folder(&root, &dir, uid), PROVISIONAL_KEYS);

            assert_eq!(policies.len(), 1);
            assert_eq!(policies.warnings().len(), 4, "{:?}", policies.warnings());
        }

        #[test]
        fn an_absent_folder_gives_nothing_and_says_nothing() {
            let (root, dir, uid) = setup();
            let absent = dir.join("absent");
            let policies = read_folder(folder(&root, &absent, uid), PROVISIONAL_KEYS);
            assert!(policies.is_empty());
            assert!(policies.warnings().is_empty());
        }

        #[test]
        fn root_s_own_etc_is_trusted() {
            // /etc and the folders above it are root's and closed to others on any sane system.
            let policies = read_folder(
                Folder {
                    dir: Path::new("/etc"),
                    owner: 0,
                    checked_up_to: Path::new("/"),
                },
                &[],
            );
            assert!(
                !policies
                    .warnings()
                    .iter()
                    .any(|w| matches!(w, Warning::Refused { place, .. } if place == "/etc")),
                "{:?}",
                policies.warnings()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::PROVISIONAL_KEYS;

    #[test]
    fn the_last_file_wins_key_by_key() {
        let files = [
            (
                PathBuf::from("/p/10.json"),
                r#"{"telemetry.enabled": false, "voice.dictation": false}"#.to_owned(),
            ),
            (
                PathBuf::from("/p/20.json"),
                r#"{"telemetry.enabled": true}"#.to_owned(),
            ),
        ];
        let policies = from_contents(&files, PROVISIONAL_KEYS);
        assert_eq!(
            policies.get("telemetry.enabled").unwrap().value,
            json!(true)
        );
        assert_eq!(
            policies.get("telemetry.enabled").unwrap().source,
            Source::File("/p/20.json".into())
        );
        assert_eq!(
            policies.get("voice.dictation").unwrap().source,
            Source::File("/p/10.json".into())
        );
    }

    #[test]
    fn a_file_that_is_no_object_is_said_and_the_others_still_read() {
        let files = [
            (PathBuf::from("/p/10.json"), "{".to_owned()),
            (PathBuf::from("/p/20.json"), "\"text\"".to_owned()),
            (
                PathBuf::from("/p/30.json"),
                r#"{"future.key": [1], "proxy.url": null}"#.to_owned(),
            ),
        ];
        let policies = from_contents(&files, PROVISIONAL_KEYS);
        assert_eq!(policies.get("future.key").unwrap().value, json!([1]));
        let kinds: Vec<&str> = policies
            .warnings()
            .iter()
            .map(|w| match w {
                Warning::Unreadable { .. } => "unreadable",
                Warning::Malformed { .. } => "malformed",
                Warning::UnknownKey { .. } => "unknown",
                Warning::Refused { .. } => "refused",
            })
            .collect();
        assert_eq!(kinds, ["unreadable", "unreadable", "unknown", "malformed"]);
    }

    #[test]
    fn only_visible_json_files_in_name_order() {
        let names = ["b.json", "a.json", ".a.json", "c.txt", "10.json", "9.json"]
            .map(String::from)
            .to_vec();
        assert_eq!(
            policy_names(names),
            ["10.json", "9.json", "a.json", "b.json"]
        );
    }
}
