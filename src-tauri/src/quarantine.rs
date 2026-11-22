//! The mark of a file Escouade copies or receives: under macOS `com.apple.quarantine`, so that
//! Gatekeeper judges it when it is opened; under Windows the mark of the web (`Zone.Identifier`)
//! its source already carries, kept on its copy. Linux has no such mark.

use std::path::Path;

/// The extended attribute macOS reads to have Gatekeeper judge a file on its opening.
#[cfg(target_os = "macos")]
pub(crate) const ATTRIBUTE: &str = "com.apple.quarantine";

/// The value of the attribute Escouade sets: a file from elsewhere (`0081`), when, by whom.
#[cfg(target_os = "macos")]
fn value() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("0081;{now:08x};Escouade;")
}

/// The quarantine attribute of `path`, if it has one.
#[cfg(target_os = "macos")]
pub(crate) fn read(path: &Path) -> Option<String> {
    use std::os::unix::ffi::OsStrExt;
    let p = std::ffi::CString::new(path.as_os_str().as_bytes()).ok()?;
    let name = std::ffi::CString::new(ATTRIBUTE).ok()?;
    let mut buf = vec![0u8; 256];
    // SAFETY: both strings end with their NUL, the buffer is as long as said.
    let n = unsafe {
        libc::getxattr(
            p.as_ptr(),
            name.as_ptr(),
            buf.as_mut_ptr().cast(),
            buf.len(),
            0,
            0,
        )
    };
    (n >= 0).then(|| String::from_utf8_lossy(&buf[..n as usize]).into_owned())
}

/// `path`, a file Escouade copied or received, marked as one from elsewhere: under macOS its
/// quarantine attribute set when it has none (one it has is kept: it says where it came from).
/// A failure is only logged: the file stays, refused to open by `sentfiles::runnable` as before.
pub(crate) fn mark(path: &Path) {
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::ffi::OsStrExt;
        if read(path).is_some() {
            return;
        }
        let (Ok(p), Ok(name)) = (
            std::ffi::CString::new(path.as_os_str().as_bytes()),
            std::ffi::CString::new(ATTRIBUTE),
        ) else {
            return;
        };
        let v = value();
        // SAFETY: both strings end with their NUL, the value is as long as said.
        let r =
            unsafe { libc::setxattr(p.as_ptr(), name.as_ptr(), v.as_ptr().cast(), v.len(), 0, 0) };
        if r != 0 {
            log::warn!(
                "{} not put in quarantine: {}",
                path.display(),
                std::io::Error::last_os_error()
            );
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = path;
}

/// The mark of the web of `from` (its `Zone.Identifier` stream), if it has one, given to its copy
/// `to` (under Windows; nothing elsewhere). `std::fs::copy` keeps it already; a copy written from
/// the bytes read does not.
pub(crate) fn carry(from: &Path, to: &Path) {
    #[cfg(windows)]
    {
        let stream = |p: &Path| {
            let mut s = p.as_os_str().to_os_string();
            s.push(":Zone.Identifier");
            std::path::PathBuf::from(s)
        };
        if let Ok(zone) = std::fs::read(stream(from)) {
            if let Err(e) = std::fs::write(stream(to), zone) {
                log::warn!("{}: its mark of the web not kept: {e}", to.display());
            }
        }
    }
    #[cfg(not(windows))]
    let _ = (from, to);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "macos")]
    #[test]
    fn a_file_escouade_copies_is_put_in_quarantine_and_one_already_marked_keeps_its_mark() {
        let dir = crate::paths::test_dir("quarantine-macos");
        let f = dir.join("a.pdf");
        std::fs::write(&f, b"%PDF-1.4").unwrap();
        assert!(read(&f).is_none());
        mark(&f);
        let v = read(&f).expect("quarantined");
        assert!(v.starts_with("0081;") && v.ends_with(";Escouade;"), "{v}");
        // Marked already (a download): its mark kept.
        let g = dir.join("b.pdf");
        std::fs::write(&g, b"%PDF-1.4").unwrap();
        let out = std::process::Command::new("xattr")
            .args([
                "-w",
                ATTRIBUTE,
                "0083;00000001;Safari;",
                g.to_str().unwrap(),
            ])
            .status()
            .unwrap();
        assert!(out.success());
        mark(&g);
        assert_eq!(read(&g).as_deref(), Some("0083;00000001;Safari;"));
    }

    /// A file of a request, copied by `asks::copy_files` from where the agent left it.
    #[cfg(any(windows, target_os = "macos"))]
    fn ask_copy(dir: &Path, from: &Path) -> std::path::PathBuf {
        let len = std::fs::metadata(from).unwrap().len();
        let files = vec![crate::asks::Incoming {
            name: "rapport.docx".into(),
            declared: None,
            option: None,
            source: crate::asks::Source::File(
                std::fs::File::open(from).unwrap(),
                len,
                Some(from.to_path_buf()),
            ),
        }];
        let out =
            crate::asks::copy_files(crate::i18n::Lang::Fr, &dir.join("ask"), files, 0, 0, false)
                .unwrap();
        crate::asks::file_path(&dir.join("ask"), &out[0].file).unwrap()
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn the_file_of_a_request_is_copied_in_quarantine() {
        let dir = crate::paths::test_dir("quarantine-ask-macos");
        let from = dir.join("rapport.docx");
        std::fs::write(&from, b"PK\x03\x04").unwrap();
        let copy = ask_copy(&dir, &from);
        assert!(read(&copy).is_some_and(|v| v.ends_with(";Escouade;")));
    }

    #[cfg(windows)]
    #[test]
    fn the_file_of_a_request_keeps_the_mark_of_the_web_of_its_source() {
        let dir = crate::paths::test_dir("quarantine-ask-windows");
        let from = dir.join("rapport.docx");
        std::fs::write(&from, b"PK").unwrap();
        let zone = "[ZoneTransfer]\r\nZoneId=3\r\n";
        std::fs::write(dir.join("rapport.docx:Zone.Identifier"), zone).unwrap();
        let copy = ask_copy(&dir, &from);
        let mut stream = copy.into_os_string();
        stream.push(":Zone.Identifier");
        assert_eq!(std::fs::read_to_string(stream).unwrap(), zone);
    }

    #[cfg(windows)]
    #[test]
    fn a_copy_keeps_the_mark_of_the_web_its_source_has_and_gets_none_from_one_without() {
        let dir = crate::paths::test_dir("quarantine-windows");
        let from = dir.join("a.docx");
        std::fs::write(&from, b"PK").unwrap();
        let zone = "[ZoneTransfer]\r\nZoneId=3\r\n";
        std::fs::write(dir.join("a.docx:Zone.Identifier"), zone).unwrap();
        // Copied by the system: kept.
        let copied = dir.join("copied.docx");
        std::fs::copy(&from, &copied).unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.join("copied.docx:Zone.Identifier")).unwrap(),
            zone
        );
        // Written from its bytes: carried.
        let written = dir.join("written.docx");
        std::fs::write(&written, std::fs::read(&from).unwrap()).unwrap();
        carry(&from, &written);
        assert_eq!(
            std::fs::read_to_string(dir.join("written.docx:Zone.Identifier")).unwrap(),
            zone
        );
        // A source without one: none made up.
        let plain = dir.join("plain.txt");
        std::fs::write(&plain, b"x").unwrap();
        let to = dir.join("plain-copy.txt");
        std::fs::write(&to, b"x").unwrap();
        carry(&plain, &to);
        assert!(std::fs::metadata(dir.join("plain-copy.txt:Zone.Identifier")).is_err());
        mark(&to);
        assert!(std::fs::metadata(dir.join("plain-copy.txt:Zone.Identifier")).is_err());
    }
}
