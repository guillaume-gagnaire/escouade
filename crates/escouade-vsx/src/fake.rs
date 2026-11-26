//! A fake Open VSX, on the loopback: its API (`/api/-/search`, `/api/<ns>/<name>[/<version>]`), the files of each
//! version (VSIX, `.sha256`, `.sigzip`, icon, README) and its public key, the VSIX signed with a key of its own as Open
//! VSX signs them. Each version may be spoilt (a false sum, a false signature, no signature, a trapped archive). It
//! answers a request in the form a proxy is sent (`GET http://host/path`) as well, and says so (`asked`).

use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::io::Write;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// How a version is spoilt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spoilt {
    No,
    /// Its `.sha256` is another file's.
    Sum,
    /// Its `.sigzip` signs other bytes.
    Signature,
    /// It has no `.sigzip` nor key.
    Unsigned,
}

/// A version of the fake.
#[derive(Clone, Debug)]
pub struct Version {
    pub namespace: String,
    pub name: String,
    pub version: String,
    pub verified: bool,
    pub pre_release: bool,
    pub vsix: Vec<u8>,
    pub spoilt: Spoilt,
}

/// A request it was sent: its path, and whether it came as to a proxy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asked {
    pub path: String,
    pub proxied: bool,
}

#[derive(Default)]
struct State {
    versions: Vec<Version>,
    asked: Vec<Asked>,
}

pub struct FakeOpenVsx {
    /// `http://127.0.0.1:<port>`.
    pub url: String,
    state: Arc<Mutex<State>>,
}

/// The fake's signing key (fixed: the tests read nothing secret of it).
pub fn signing_key() -> SigningKey {
    SigningKey::from_bytes(&[42u8; 32])
}

/// Its public key, in SPKI PEM, as Open VSX serves its own.
pub fn public_pem() -> String {
    let mut der = vec![
        0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
    ];
    der.extend_from_slice(signing_key().verifying_key().as_bytes());
    format!(
        "-----BEGIN PUBLIC KEY-----\n{}\n-----END PUBLIC KEY-----\n",
        base64::engine::general_purpose::STANDARD.encode(der)
    )
}

/// A `.sigzip` of `vsix`, as Open VSX makes it.
pub fn sigzip(vsix: &[u8]) -> Vec<u8> {
    let sig = signing_key().sign(vsix).to_bytes();
    zip(&[
        (".signature.sig", &sig[..]),
        (".signature.manifest", b"{}"),
        (".signature.p7s", b""),
    ])
}

/// An archive of `entries`, deflated.
pub fn zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut w = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, bytes) in entries {
        w.start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        w.write_all(bytes).unwrap();
    }
    w.finish().unwrap().into_inner()
}

/// An archive whose entry `link` is a symbolic link to `target`, beside `entries`.
pub fn zip_with_link(entries: &[(&str, &[u8])], link: &str, target: &str) -> Vec<u8> {
    let mut w = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, bytes) in entries {
        w.start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        w.write_all(bytes).unwrap();
    }
    w.add_symlink(link, target, zip::write::SimpleFileOptions::default())
        .unwrap();
    w.finish().unwrap().into_inner()
}

/// The `package.json` of `namespace.name` at `version`, with `extra` laid over it.
pub fn package(namespace: &str, name: &str, version: &str, extra: Value) -> Vec<u8> {
    let mut p = json!({
        "publisher": namespace,
        "name": name,
        "displayName": format!("{name} ({namespace})"),
        "version": version,
        "engines": { "vscode": "^1.80.0" },
    });
    if let (Some(p), Some(extra)) = (p.as_object_mut(), extra.as_object()) {
        for (k, v) in extra {
            p.insert(k.clone(), v.clone());
        }
    }
    serde_json::to_vec_pretty(&p).unwrap()
}

/// A colour theme's VSIX: `themes/<name>.json`, labelled `label`.
pub fn theme_vsix(namespace: &str, name: &str, version: &str, label: &str) -> Vec<u8> {
    let pkg = package(
        namespace,
        name,
        version,
        json!({ "contributes": { "themes": [{ "label": label, "uiTheme": "vs-dark", "path": "./themes/theme.json" }] } }),
    );
    let theme = json!({ "name": label, "type": "dark", "colors": { "editor.background": "#102030" }, "tokenColors": [] });
    zip(&[
        ("extension.vsixmanifest", b"<PackageManifest/>"),
        ("[Content_Types].xml", b"<Types/>"),
        ("extension/package.json", &pkg),
        (
            "extension/themes/theme.json",
            &serde_json::to_vec(&theme).unwrap(),
        ),
        (
            "extension/README.md",
            b"# A theme\n\n![remote](https://example.com/x.png)\n",
        ),
    ])
}

/// An extension with code: for the web host (`browser`) or for Node (`main`).
pub fn code_vsix(namespace: &str, name: &str, version: &str, browser: bool) -> Vec<u8> {
    let entry = if browser {
        json!({ "browser": "./dist/web.js" })
    } else {
        json!({ "main": "./out/main.js" })
    };
    let pkg = package(namespace, name, version, entry);
    zip(&[
        ("extension/package.json", &pkg),
        ("extension/dist/web.js", b"exports.activate = () => {};"),
        ("extension/out/main.js", b"exports.activate = () => {};"),
    ])
}

impl Version {
    pub fn new(namespace: &str, name: &str, version: &str, vsix: Vec<u8>) -> Version {
        Version {
            namespace: namespace.into(),
            name: name.into(),
            version: version.into(),
            verified: true,
            pre_release: false,
            vsix,
            spoilt: Spoilt::No,
        }
    }

    pub fn spoilt(mut self, spoilt: Spoilt) -> Self {
        self.spoilt = spoilt;
        self
    }

    pub fn unverified(mut self) -> Self {
        self.verified = false;
        self
    }

    fn stem(&self) -> String {
        format!("{}.{}-{}", self.namespace, self.name, self.version)
    }
}

fn respond(status: u16, mime: &str, body: &[u8]) -> Vec<u8> {
    let mut out = format!(
        "HTTP/1.1 {status} X\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    out.extend_from_slice(body);
    out
}

impl FakeOpenVsx {
    pub async fn start() -> FakeOpenVsx {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let state = Arc::new(Mutex::new(State::default()));
        let (st, base) = (state.clone(), url.clone());
        tokio::spawn(async move {
            while let Ok((mut sock, _)) = listener.accept().await {
                let (st, base) = (st.clone(), base.clone());
                tokio::spawn(async move {
                    let mut head = Vec::new();
                    let mut byte = [0u8; 1];
                    while !head.ends_with(b"\r\n\r\n") {
                        if sock.read(&mut byte).await.unwrap_or(0) == 0 {
                            return;
                        }
                        head.push(byte[0]);
                    }
                    let head = String::from_utf8_lossy(&head).to_string();
                    let target = head.split(' ').nth(1).unwrap_or("/").to_owned();
                    let proxied = target.starts_with("http://");
                    let path = if proxied {
                        let rest = &target["http://".len()..];
                        rest.find('/')
                            .map_or("/".to_owned(), |i| rest[i..].to_owned())
                    } else {
                        target
                    };
                    st.lock().asked.push(Asked {
                        path: path.clone(),
                        proxied,
                    });
                    let out = answer(&st, &base, &path);
                    let _ = sock.write_all(&out).await;
                    let _ = sock.shutdown().await;
                });
            }
        });
        FakeOpenVsx { url, state }
    }

    pub fn add(&self, v: Version) {
        self.state.lock().versions.push(v);
    }

    /// What it was asked, in order.
    pub fn asked(&self) -> Vec<Asked> {
        self.state.lock().asked.clone()
    }

    /// Whether a file of a version (`/files/…`) was asked.
    pub fn downloaded(&self) -> bool {
        self.asked().iter().any(|a| a.path.starts_with("/files/"))
    }
}

fn card(base: &str, v: &Version, all: &[&Version]) -> Value {
    let stem = v.stem();
    let f = |ext: &str| format!("{base}/files/{stem}.{ext}");
    let mut files = json!({
        "download": f("vsix"),
        "sha256": f("sha256"),
        "icon": format!("{base}/files/{stem}/icon.png"),
        "readme": format!("{base}/files/{stem}/README.md"),
    });
    if v.spoilt != Spoilt::Unsigned {
        files["signature"] = json!(f("sigzip"));
        files["publicKey"] = json!(format!("{base}/api/-/public-key/fake"));
    }
    let mut versions = serde_json::Map::new();
    versions.insert("latest".into(), json!(""));
    for o in all {
        versions.insert(o.version.clone(), json!(""));
    }
    json!({
        "namespace": v.namespace,
        "name": v.name,
        "version": v.version,
        "displayName": format!("{} ({})", v.name, v.namespace),
        "description": format!("The fake {}", v.name),
        "verified": v.verified,
        "preRelease": v.pre_release,
        "downloadable": true,
        "downloadCount": 7,
        "publishedBy": { "loginName": v.namespace, "fullName": format!("{} Inc.", v.namespace) },
        "allVersions": versions,
        "files": files,
    })
}

fn answer(st: &Mutex<State>, base: &str, path: &str) -> Vec<u8> {
    let st = st.lock();
    let (path, query) = path.split_once('?').unwrap_or((path, ""));
    let json_of = |v: Value| respond(200, "application/json", &serde_json::to_vec(&v).unwrap());
    // A redirection to `to` (ESC-648): what a registry's storage answers.
    if path == "/redirect" {
        let to = query
            .split('&')
            .find_map(|p| p.strip_prefix("to="))
            .map(|t| {
                percent_encoding::percent_decode_str(t)
                    .decode_utf8_lossy()
                    .into_owned()
            })
            .unwrap_or_default();
        return format!("HTTP/1.1 302 Found\r\nLocation: {to}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .into_bytes();
    }
    if path == "/api/-/public-key/fake" {
        return respond(200, "text/plain", public_pem().as_bytes());
    }
    if path == "/api/-/search" {
        let q = query
            .split('&')
            .find_map(|p| p.strip_prefix("query="))
            .unwrap_or_default()
            .to_lowercase();
        let mut seen = Vec::new();
        let mut list = Vec::new();
        for v in st.versions.iter().rev() {
            let id = format!("{}.{}", v.namespace, v.name);
            if seen.contains(&id) || !id.to_lowercase().contains(&q) {
                continue;
            }
            seen.push(id);
            list.push(card(base, v, &[]));
        }
        return json_of(json!({ "offset": 0, "totalSize": list.len(), "extensions": list }));
    }
    if let Some(rest) = path.strip_prefix("/api/") {
        let parts: Vec<&str> = rest.split('/').collect();
        let of: Vec<&Version> = st
            .versions
            .iter()
            .filter(|v| parts.len() >= 2 && v.namespace == parts[0] && v.name == parts[1])
            .collect();
        let pick = match parts.get(2) {
            Some(want) => of.iter().find(|v| v.version == *want).copied(),
            None => of
                .iter()
                .max_by(|a, b| crate::registry::compare_versions(&a.version, &b.version))
                .copied(),
        };
        return match pick {
            Some(v) => json_of(card(base, v, &of)),
            None => respond(404, "application/json", br#"{"error":"not found"}"#),
        };
    }
    if let Some(name) = path.strip_prefix("/files/") {
        for v in &st.versions {
            let stem = v.stem();
            if name == format!("{stem}.vsix") {
                return respond(200, "application/octet-stream", &v.vsix);
            }
            if name == format!("{stem}.sha256") {
                let sum = if v.spoilt == Spoilt::Sum {
                    crate::verify::sha256_hex(b"another file")
                } else {
                    crate::verify::sha256_hex(&v.vsix)
                };
                return respond(200, "text/plain", sum.as_bytes());
            }
            if name == format!("{stem}.sigzip") && v.spoilt != Spoilt::Unsigned {
                let signed = if v.spoilt == Spoilt::Signature {
                    b"other bytes".to_vec()
                } else {
                    v.vsix.clone()
                };
                return respond(200, "application/zip", &sigzip(&signed));
            }
            if name == format!("{stem}/icon.png") {
                return respond(200, "image/png", b"\x89PNG\r\n\x1a\nfake");
            }
            if name == format!("{stem}/README.md") {
                return respond(200, "text/markdown", b"# Fake\n\nA README.\n");
            }
        }
    }
    respond(404, "text/plain", b"not found")
}
