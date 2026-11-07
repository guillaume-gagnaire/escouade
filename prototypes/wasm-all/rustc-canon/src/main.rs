//! `RUSTC_WRAPPER` of a `run.wasm` compilation (ESC-330, docs/PLUGINS.md §8.5.2): the same rustc on Windows and on Linux
//! gives the same module only if it is told the same things. Cargo tells it three things that depend on the machine:
//!
//! 1. `-C metadata`: its hash takes in the host of `rustc -vV` and the path of a path dependency, with its `\`. It salts
//!    every symbol, and the order of the code follows the symbols. Made here from the crate alone: its name, its version,
//!    its features, the target it is for.
//! 2. the folders: the panic locations hold the path of each source file (`Location::file`). `--remap-path-prefix` takes
//!    away a folder's start, not the `\` that Windows puts after it (`src\de\mod.rs`). So one rule per source file, its
//!    whole path to its canonical path (`/src/...`, `/cargo/...`).
//! 3. the source's own path, relative to a folder that depends on the crate: given here in full.
//!
//! `ESCOUADE_CANON_ROOT` is the repository; `CARGO_HOME` the crates downloaded.

use std::env;
use std::path::{Path, PathBuf};
use std::process::{exit, Command};

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

/// Every `.rs` under `dir`, in an order of their paths.
fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = read.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            if p.file_name().is_some_and(|n| n != "target" && n != "node_modules" && n != ".git") {
                sources(&p, out);
            }
        } else if p.extension().is_some_and(|e| e == "rs") {
            out.push(p);
        }
    }
}

/// The canonical folder of a crate.
fn canonical(manifest: &Path, root: &Path, cargo: &Path) -> String {
    let registry = cargo.join("registry").join("src");
    if let Ok(rest) = manifest.strip_prefix(&registry) {
        // `<index>-<hash>/serde-1.0.0`: the crate's own name is enough, the lock says where it comes from.
        let name: Vec<_> = rest.components().skip(1).collect();
        let name: PathBuf = name.iter().collect();
        return format!("/cargo/{}", unix(&name));
    }
    match manifest.strip_prefix(root) {
        Ok(rest) => format!("/src/{}", unix(rest)),
        Err(_) => format!("/other/{:016x}", fnv(&unix(manifest), 0)),
    }
}

fn main() {
    let mut args = env::args_os().skip(1);
    let rustc = args.next().expect("the rustc");
    let args: Vec<String> = args.map(|a| a.to_string_lossy().into_owned()).collect();
    let run = |args: &[String]| -> ! {
        let status = Command::new(&rustc).args(args).status().expect("rustc");
        exit(status.code().unwrap_or(1));
    };
    let name = args
        .iter()
        .position(|a| a == "--crate-name")
        .and_then(|i| args.get(i + 1))
        .cloned();
    let (Some(name), Ok(manifest)) = (name, env::var("CARGO_MANIFEST_DIR")) else {
        run(&args); // `rustc -vV`, `--print`: Cargo asking.
    };
    let manifest = PathBuf::from(manifest);
    let root = PathBuf::from(env::var("ESCOUADE_CANON_ROOT").expect("ESCOUADE_CANON_ROOT"));
    let cargo = PathBuf::from(env::var("CARGO_HOME").expect("CARGO_HOME"));
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
                env::current_dir().expect("the folder").join(p)
            };
            out.push(full.to_string_lossy().into_owned());
            i += 1;
            continue;
        }
        out.push(a.clone());
        i += 1;
    }
    let mut files = Vec::new();
    sources(&manifest, &mut files);
    for f in &files {
        let rest = f.strip_prefix(&manifest).expect("under its folder");
        out.push(format!(
            "--remap-path-prefix={}={canon}/{}",
            f.display(),
            unix(rest)
        ));
    }
    run(&out);
}
