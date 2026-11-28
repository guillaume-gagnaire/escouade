//! The page of the browser's client into the crate (BC-1, ESC-704): the folder `ESCOUADE_WEB_DIST` names
//! (`npm run build:web`'s `dist-web/`; relative: to the repository), each file compressed by gzip into `OUT_DIR`, and
//! `page.rs` that lists them: `index.html`, `assets/` and what Vite copies of `public/` (What's up ?, the tour's images, the
//! logo). Refused, the build stops: a name it would not serve as it is, a link, or a mark of Tauri's IPC in a file (the
//! browser's build has none).

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};

const VAR: &str = "ESCOUADE_WEB_DIST";
/// What the page must never hold (src/web/tauri-refused.ts, `FORBIDDEN_IN_BUNDLE`).
const FORBIDDEN: [&str; 2] = ["__TAURI_INTERNALS__", "ipc.localhost"];

fn main() {
    println!("cargo:rerun-if-env-changed={VAR}");
    println!("cargo:rerun-if-changed=build.rs");
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"));
    let dir = std::env::var_os(VAR).filter(|d| !d.is_empty()).map(|d| {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        repo.join(d)
    });
    let files = match &dir {
        Some(dir) => {
            println!("cargo:rerun-if-changed={}", dir.display());
            read(dir)
        }
        None => Vec::new(),
    };
    let web = out.join("web");
    let _ = std::fs::remove_dir_all(&web);
    std::fs::create_dir_all(&web).expect("OUT_DIR/web");
    let mut rs = String::from(
        "/// The files of the page, `index.html` first.\npub static FILES: &[File] = &[\n",
    );
    let (mut raw, mut packed) = (0u64, 0u64);
    for (i, (name, bytes)) in files.iter().enumerate() {
        let mut gz = flate2::GzBuilder::new()
            .mtime(0)
            .write(Vec::new(), flate2::Compression::best());
        gz.write_all(bytes).expect("gzip");
        let gz = gz.finish().expect("gzip");
        raw += bytes.len() as u64;
        packed += gz.len() as u64;
        std::fs::write(web.join(format!("{i}.gz")), &gz).expect("OUT_DIR/web");
        let _ = writeln!(
            rs,
            "    ({name:?}, include_bytes!(concat!(env!(\"OUT_DIR\"), \"/web/{i}.gz\"))),"
        );
    }
    rs.push_str("];\n");
    // A mark found in the binary (`scripts/server-package.sh`): written here alone, never in a source.
    let mark = if files.is_empty() {
        String::new()
    } else {
        format!(
            "{}{}:{}:{raw}:{packed}",
            "escouade-web-",
            "page",
            files.len()
        )
    };
    let _ = writeln!(
        rs,
        "/// `escouade-web-page:<files>:<bytes>:<compressed>` when it has a page, else empty.\npub static MARK: &str = {mark:?};"
    );
    std::fs::write(out.join("page.rs"), rs).expect("OUT_DIR/page.rs");
    if !files.is_empty() {
        println!(
            "cargo:warning=escouade-web: {} files, {} kB, {} kB compressed",
            files.len(),
            raw / 1000,
            packed / 1000
        );
    }
}

/// Every file of `dir`, by its path from it (`/` between folders), `index.html` first, the rest by name. A link is
/// refused (never followed out of the folder), as a name the server would not serve as it is.
fn read(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let fail = |why: String| -> ! { panic!("{VAR}={}: {why}", dir.display()) };
    let Ok(html) = std::fs::read(dir.join("index.html")) else {
        fail("no index.html (npm run build:web)".into())
    };
    let mut rest = Vec::new();
    let mut folders = vec![(dir.to_path_buf(), String::new())];
    while let Some((folder, prefix)) = folders.pop() {
        for entry in std::fs::read_dir(&folder).unwrap_or_else(|e| fail(e.to_string())) {
            let entry = entry.unwrap_or_else(|e| fail(e.to_string()));
            let name = format!("{prefix}{}", entry.file_name().to_string_lossy());
            let kind = entry.file_type().unwrap_or_else(|e| fail(e.to_string()));
            if !servable(&name) {
                fail(format!("{name}: a name the server would not serve"));
            }
            if kind.is_symlink() {
                fail(format!("{name}: a link"));
            } else if kind.is_dir() {
                folders.push((entry.path(), format!("{name}/")));
            } else if name != "index.html" {
                let bytes = std::fs::read(entry.path()).unwrap_or_else(|e| fail(e.to_string()));
                rest.push((name, bytes));
            }
        }
    }
    rest.sort();
    let mut files = vec![("index.html".to_string(), html)];
    files.extend(rest);
    for (name, bytes) in &files {
        let text = String::from_utf8_lossy(bytes);
        for mark in FORBIDDEN {
            if text.contains(mark) {
                fail(format!(
                    "{name} holds {mark}: the browser's build has no Tauri"
                ));
            }
        }
    }
    files
}

/// A path the server serves as it is (`remoteapi::page::servable`): segments of printable ASCII, none empty nor hidden,
/// no `\`, `%`, `?` or `#`.
fn servable(name: &str) -> bool {
    !name.is_empty()
        && name.split('/').all(|segment| {
            !segment.is_empty()
                && !segment.starts_with('.')
                && segment
                    .bytes()
                    .all(|b| b.is_ascii_graphic() && !matches!(b, b'\\' | b'%' | b'?' | b'#'))
        })
}
