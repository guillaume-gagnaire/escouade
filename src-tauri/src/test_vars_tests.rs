//! SEC-13 of the 2.2.0 audit: a variable of the environment by which the tests put a fake service, a shorter delay or a
//! file of theirs in the place of the real one is read by a build of development alone (`cfg!(debug_assertions)`), or
//! by the unit tests alone (`cfg!(test)`). A published app ignores them: whoever sets the environment of a session does
//! not send the account's token to a server of its own (`ESCOUADE_AZURE_DEVOPS_API`) nor makes the app wait otherwise.
//!
//! Read in the sources of the app and of its crates: every `ESCOUADE_*` read by its name, out of the tests, is read
//! under one of these conditions in its function, or is one of `NOT_OF_TESTS`, which are no test's.

use std::path::{Path, PathBuf};

/// The variables of Escouade that are not the tests', and why they stay read in a published app.
const NOT_OF_TESTS: &[(&str, &str)] = &[
    ("ESCOUADE_LANG", "the language the core gives the process of a plugin"),
    (
        "ESCOUADE_TERM_TRACE",
        "a diagnostic: the times and the sizes of the writes to a terminal, nothing of what they write",
    ),
    (
        "ESCOUADE_ORT_DYLIB",
        "where ONNX Runtime is for a build that loads it (`ort-dynamic`)",
    ),
    (
        "ESCOUADE_JAVY",
        "the compiler of the plugins in JS that `escouade-plugin` is given (the store's image, the scripts)",
    ),
    (
        "ESCOUADE_CONNECTOR_CONFIG",
        "where `escouade-connector` reads its connector.toml (ESC-599), as its `--config` says it",
    ),
];

/// The files compiled for the tests alone (their `mod` is under `#[cfg(test)]`), by the end of their path.
const TESTS_ALONE: &[&str] = &[
    "src-tauri/src/fluxbench.rs",
    "src-tauri/src/engine/bench/",
    "crates/escouade-team-crypto/src/vectors.rs",
];

/// The sources of the app and of its crates, without their tests.
fn sources() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut dirs = vec![root.join("src-tauri").join("src")];
    for e in std::fs::read_dir(root.join("crates")).unwrap().flatten() {
        dirs.push(e.path().join("src"));
    }
    let mut files = Vec::new();
    while let Some(dir) = dirs.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.flatten() {
            let path = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            if path.is_dir() {
                if name != "tests" {
                    dirs.push(path);
                }
            } else if TESTS_ALONE
                .iter()
                .any(|t| path.to_string_lossy().replace('\\', "/").contains(t))
            {
                continue;
            } else if name.ends_with(".rs")
                && !name.ends_with("_tests.rs")
                && name != "tests.rs"
                && name != "build.rs"
            {
                files.push(path);
            }
        }
    }
    files
}

/// The lines of `text` before its module of tests (`#[cfg(test)]` then `mod …`), the rest being the tests'.
fn out_of_tests(text: &str) -> Vec<&str> {
    let lines: Vec<&str> = text.lines().collect();
    let end = lines
        .windows(2)
        .position(|w| w[0].trim() == "#[cfg(test)]" && w[1].trim_start().starts_with("mod "))
        .unwrap_or(lines.len());
    lines[..end].to_vec()
}

/// The `ESCOUADE_*` variables a line reads by their name.
fn read_by(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    for call in ["env::var(\"", "env::var_os(\"", "debug_env(\""] {
        let mut rest = line;
        while let Some(i) = rest.find(call) {
            rest = &rest[i + call.len()..];
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == '_')
                .collect();
            if name.starts_with("ESCOUADE_") {
                out.push(format!("{call}{name}"));
            }
        }
    }
    out
}

/// Whether the read on line `at` of `lines` is under a condition of a build of development or of the unit tests: in
/// the attributes of its function or its body up to the read; or, for a build of development, in the expression that
/// reads it (`.filter(|_| cfg!(debug_assertions))`). A `cfg!(test)` after the read is no guard of it.
fn gated(lines: &[&str], at: usize) -> bool {
    let start = (0..=at)
        .rev()
        .find(|&i| {
            let l = lines[i].trim_start();
            [
                "fn ",
                "pub fn ",
                "pub(crate) fn ",
                "pub(super) fn ",
                "async fn ",
                "pub async fn ",
                "pub(crate) async fn ",
            ]
            .iter()
            .any(|p| l.starts_with(p))
        })
        .unwrap_or(0);
    let from = start.saturating_sub(3);
    let to = (at + 6).min(lines.len());
    lines[from..=at].iter().any(|l| {
        l.contains("debug_assertions") || l.contains("cfg!(test)") || l.contains("cfg(test)")
    }) || lines[at + 1..to]
        .iter()
        .any(|l| l.contains("debug_assertions"))
}

#[test]
fn the_variables_of_the_tests_are_read_by_a_build_of_development_alone() {
    let mut ungated = Vec::new();
    let mut seen = 0;
    for file in sources() {
        let text = std::fs::read_to_string(&file).unwrap();
        let lines = out_of_tests(&text);
        for (at, line) in lines.iter().enumerate() {
            for read in read_by(line) {
                seen += 1;
                let name = read.rsplit('"').next().unwrap_or_default();
                if read.starts_with("debug_env(")
                    || NOT_OF_TESTS.iter().any(|(n, _)| *n == name)
                    || gated(&lines, at)
                {
                    continue;
                }
                ungated.push(format!("{}:{}: {name}", file.display(), at + 1));
            }
        }
    }
    assert!(seen > 20, "{seen} reads found: the sources were not read");
    assert!(
        ungated.is_empty(),
        "read in a published app too:\n{}",
        ungated.join("\n")
    );
}
