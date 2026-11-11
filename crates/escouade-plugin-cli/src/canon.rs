//! `rustc-canon`, the `RUSTC_WRAPPER` of a `run.wasm` compilation (ESC-330, ESC-335, docs/PLUGINS.md §8.5.2): the same
//! rustc on Windows and on Linux gives the same module only if it is told the same things. It is `escouade-plugin`
//! itself: cargo runs that program again, [`ROOT`] set ([`crate::cli::main`] comes here first). Cargo tells rustc three
//! things that depend on the machine:
//!
//! 1. `-C metadata`: its hash takes in the host of `rustc -vV` and the path of a path dependency, with its `\`. It salts
//!    every symbol, and the order of the code follows the symbols. Made here from the crate alone: its name, its version,
//!    its features, the target it is for.
//! 2. the folders: the panic locations hold the path of each source file (`Location::file`). `--remap-path-prefix` takes
//!    away a folder's start, not the `\` that Windows puts after it (`src\de\mod.rs`). So one rule per source file, its
//!    whole path to its canonical path (`/src/...`, `/cargo/...`, `/out/...` for what a build script generated).
//! 3. the source's own path, relative to a folder that depends on the crate: given here in full.
//!
//! **Its SHA-256 is that of this file** ([`sha256`]), which `version.json` notes: every byte here can change a module.
//! That is why the profile of a module ([`PROFILE`]) is here too. A change of this file changes the fixtures' hashes
//! (`goldens.json`), and every published module would have to be compiled again.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Names the plugin's folder, and says that this process is the wrapper of a compilation.
pub const ROOT: &str = "ESCOUADE_CANON_ROOT";

/// The profile of a module, by `CARGO_PROFILE_RELEASE_*`, which pass over the author's `[profile]`.
pub const PROFILE: &[(&str, &str)] = &[
    ("OPT_LEVEL", "3"),
    ("LTO", "fat"),
    ("CODEGEN_UNITS", "1"),
    ("PANIC", "abort"),
    ("STRIP", "true"),
    ("DEBUG", "false"),
    ("DEBUG_ASSERTIONS", "false"),
    ("OVERFLOW_CHECKS", "false"),
    ("INCREMENTAL", "false"),
    ("RPATH", "false"),
];

/// What the store notes of the wrapper: the SHA-256 of this file, in its LF form (a checkout under Windows may have CRLF).
pub fn sha256() -> String {
    crate::wasm::sha256(include_str!("canon.rs").replace("\r\n", "\n").as_bytes())
}

fn fnv(data: &str, seed: u64) -> u64 {
    let mut h = 0xcbf29ce484222325u64 ^ seed;
    for b in data.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// `a\b` as `a/b`, whatever the system.
fn unix(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

/// Every file under `dir` that a compilation may name, in the order of their paths: `.rs` alone, or all of them.
fn sources(dir: &Path, all: bool, out: &mut Vec<PathBuf>) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = read.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            if p.file_name()
                .is_some_and(|n| n != "target" && n != "node_modules" && n != ".git")
            {
                sources(&p, all, out);
            }
        } else if all || p.extension().is_some_and(|e| e == "rs") {
            out.push(p);
        }
    }
}

/// The canonical folder of a crate.
fn canonical(manifest: &Path, root: &Path, cargo: &Path) -> String {
    let registry = cargo.join("registry").join("src");
    if let Ok(rest) = manifest.strip_prefix(&registry) {
        // `<index>-<hash>/serde-1.0.0`: the crate's own name is enough, the lock says where it comes from.
        let name: PathBuf = rest.components().skip(1).collect();
        return format!("/cargo/{}", unix(&name));
    }
    match manifest.strip_prefix(root) {
        Ok(rest) => format!("/src/{}", unix(rest)),
        Err(_) => format!("/other/{:016x}", fnv(&unix(manifest), 0)),
    }
}

/// `args`, as cargo gave them to the wrapper: rustc, then its arguments. Gives its exit code.
pub fn run(args: Vec<String>) -> i32 {
    let Some((rustc, args)) = args.split_first() else {
        return 2;
    };
    let run = |args: &[String]| -> i32 {
        match Command::new(rustc).args(args).status() {
            Ok(status) => status.code().unwrap_or(1),
            Err(e) => {
                eprintln!("{rustc}: {e}");
                2
            }
        }
    };
    let name = args
        .iter()
        .position(|a| a == "--crate-name")
        .and_then(|i| args.get(i + 1))
        .cloned();
    let (Some(name), Ok(manifest)) = (name, env::var("CARGO_MANIFEST_DIR")) else {
        return run(args); // `rustc -vV`, `--print`: Cargo asking.
    };
    let manifest = PathBuf::from(manifest);
    let root = PathBuf::from(env::var(ROOT).unwrap_or_default());
    let cargo = PathBuf::from(env::var("CARGO_HOME").unwrap_or_default());
    let canon = canonical(&manifest, &root, &cargo);

    let target = args
        .iter()
        .position(|a| a == "--target")
        .and_then(|i| args.get(i + 1))
        .map_or("host", String::as_str);
    let mut features: Vec<&str> = args
        .windows(2)
        .filter(|w| w[0] == "--cfg" && w[1].starts_with("feature="))
        .map(|w| w[1].as_str())
        .collect();
    features.sort();
    let identity = format!(
        "{name}\n{}\n{}\n{target}",
        env::var("CARGO_PKG_VERSION").unwrap_or_default(),
        features.join("\n")
    );
    let metadata = format!("{:016x}", fnv(&identity, 1));

    let mut out: Vec<String> = Vec::with_capacity(args.len() + 64);
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if a == "-C" && args.get(i + 1).is_some_and(|v| v.starts_with("metadata=")) {
            out.push("-C".into());
            out.push(format!("metadata={metadata}"));
            i += 2;
            continue;
        }
        // The source: in full, with the separators of the system.
        if a.ends_with(".rs") && !a.starts_with('-') {
            let p = Path::new(a);
            let full = if p.is_absolute() {
                p.to_path_buf()
            } else {
                env::current_dir().unwrap_or_default().join(p)
            };
            out.push(full.to_string_lossy().into_owned());
            i += 1;
            continue;
        }
        out.push(a.clone());
        i += 1;
    }
    let mut files = Vec::new();
    sources(&manifest, false, &mut files);
    for f in &files {
        if let Ok(rest) = f.strip_prefix(&manifest) {
            out.push(format!(
                "--remap-path-prefix={}={canon}/{}",
                f.display(),
                unix(rest)
            ));
        }
    }
    // What a build script generated: its folder is under `target`, wherever that is.
    if let Ok(generated) = env::var("OUT_DIR") {
        let generated = PathBuf::from(generated);
        let mut files = Vec::new();
        sources(&generated, true, &mut files);
        for f in &files {
            if let Ok(rest) = f.strip_prefix(&generated) {
                out.push(format!(
                    "--remap-path-prefix={}=/out/{name}/{}",
                    f.display(),
                    unix(rest)
                ));
            }
        }
    }
    run(&out)
}
