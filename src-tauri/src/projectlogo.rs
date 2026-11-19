//! A project's tile on the rail (ESC-167): the letters chosen in place of its initials, and the
//! logo that takes the place of the letters. Both are of this machine, as its colour: never in its
//! `.escouade.yml`. The logo is a copy in the app's data folder, under the project
//! (`DataDir::project_files`), whatever becomes of the file it was chosen from; the window shows
//! it as an image (`data:` URL in an `<img>`), an SVG's scripts never run.

use crate::core::{size_label, MB};
use crate::i18n::Lang;
use crate::model::new_id;
use crate::paths::{is_plain_id, DataDir};
use crate::ticketimages::plain_name;
use anyhow::{bail, Result};
use base64::Engine;
use std::path::Path;
use unicode_normalization::UnicodeNormalization;

/// The heaviest logo taken: a tile is 36 px, a 512 px PNG weighs far less; an ICO with all its
/// sizes, up to 256 px, stays under it too.
pub(crate) const MAX_LOGO: usize = MB;

/// The most letters a tile shows: three still fit in its 36 px at 11 px, a fourth would not.
pub(crate) const MAX_LETTERS: usize = 3;

/// A logo read and recognised by its first bytes, whatever its extension says.
#[derive(Debug)]
pub(crate) struct Logo {
    pub ext: &'static str,
    pub media_type: &'static str,
    pub bytes: Vec<u8>,
}

/// What the bytes are, of the formats a logo may be: PNG, JPEG, WebP, ICO, SVG.
fn kind_of(b: &[u8]) -> Option<(&'static str, &'static str)> {
    if b.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some(("png", "image/png"));
    }
    if b.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some(("jpg", "image/jpeg"));
    }
    if b.len() >= 12 && &b[..4] == b"RIFF" && &b[8..12] == b"WEBP" {
        return Some(("webp", "image/webp"));
    }
    // An icon's header: reserved 0, type 1, then how many images (at least one).
    if b.len() >= 6 && b.starts_with(&[0, 0, 1, 0]) && (b[4] | b[5]) != 0 {
        return Some(("ico", "image/x-icon"));
    }
    let text = std::str::from_utf8(b.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(b)).ok()?;
    let text = text.trim_start();
    let svg = text.starts_with('<') && text.to_ascii_lowercase().contains("<svg");
    svg.then_some(("svg", "image/svg+xml"))
}

/// The name of the file the user knows it by.
fn shown_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

/// The logo of the file `path`, checked: one that weighs more than `MAX_LOGO`, cannot be read or is
/// no image of the formats taken is refused, saying why.
pub(crate) fn read_in(lang: Lang, path: &Path) -> Result<Logo> {
    let name = shown_name(path);
    let unreadable = |e: std::io::Error| {
        anyhow::anyhow!(tr_in!(
            lang,
            "Impossible de lire « {name} » : {e}",
            "Can’t read “{name}”: {e}"
        ))
    };
    let meta = std::fs::metadata(path).map_err(unreadable)?;
    // Its size, before it is read: a huge file is never loaded.
    if meta.len() > MAX_LOGO as u64 {
        let size = size_label(lang, meta.len() as usize);
        let max = size_label(lang, MAX_LOGO);
        bail!(tr_in!(
            lang,
            "« {name} » pèse {size} : un logo pèse {max} au plus.",
            "“{name}” weighs {size}: a logo weighs {max} at most."
        ));
    }
    let bytes = std::fs::read(path).map_err(unreadable)?;
    checked(lang, &name, bytes)
}

/// The logo of `bytes` sent by a client of the remote API (`name`: its file's name, for what is said), checked as a
/// file is (`read_in`): the image of this computer sent to a server (ESC-538).
pub(crate) fn checked(lang: Lang, name: &str, bytes: Vec<u8>) -> Result<Logo> {
    if bytes.len() > MAX_LOGO {
        let size = size_label(lang, bytes.len());
        let max = size_label(lang, MAX_LOGO);
        bail!(tr_in!(
            lang,
            "« {name} » pèse {size} : un logo pèse {max} au plus.",
            "“{name}” weighs {size}: a logo weighs {max} at most."
        ));
    }
    let Some((ext, media_type)) = kind_of(&bytes) else {
        bail!(tr_in!(
            lang,
            "« {name} » n’est pas une image PNG, JPEG, WebP, SVG ou ICO.",
            "“{name}” isn’t a PNG, JPEG, WebP, SVG or ICO image."
        ));
    };
    Ok(Logo {
        ext,
        media_type,
        bytes,
    })
}

/// The `data:` URL of the image, for an `<img>`.
pub(crate) fn data_url(media_type: &str, bytes: &[u8]) -> String {
    let data = base64::engine::general_purpose::STANDARD.encode(bytes);
    format!("data:{media_type};base64,{data}")
}

/// The logo of `source` copied under the project, as a file of its own (`logo-<uniq>.<ext>`): the
/// project's previous logo goes once the copy is there, a refusal leaves it as it was. Returns the
/// copy's name.
pub(crate) fn import(
    lang: Lang,
    data: &DataDir,
    project_id: &str,
    source: &Path,
) -> Result<String> {
    if !is_plain_id(project_id) {
        bail!("project id {project_id}");
    }
    let logo = read_in(lang, source)?;
    keep(data, project_id, &logo)
}

/// The logo `logo`, checked already, copied under the project (`import`): its file's name.
pub(crate) fn keep(data: &DataDir, project_id: &str, logo: &Logo) -> Result<String> {
    if !is_plain_id(project_id) {
        bail!("project id {project_id}");
    }
    let dir = data.project_files(project_id);
    std::fs::create_dir_all(&dir)?;
    let file = format!("logo-{}.{}", new_id(), logo.ext);
    std::fs::write(dir.join(&file), &logo.bytes)?;
    keep_only(&dir, Some(&file));
    Ok(file)
}

/// The files of the project's folder but `keep` removed (its earlier logos).
fn keep_only(dir: &Path, keep: Option<&str>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if Some(name.as_str()) != keep && entry.path().is_file() {
            if let Err(e) = std::fs::remove_file(entry.path()) {
                log::warn!("old logo {name} not removed: {e}");
            }
        }
    }
}

/// The `data:` URL of the project's logo `file`, or None: a name that is no plain file name (a
/// state edited by hand) is never read, nor a file gone or no longer an image.
pub(crate) fn url_of(data: &DataDir, project_id: &str, file: &str) -> Option<String> {
    if !is_plain_id(project_id) || !plain_name(file) {
        return None;
    }
    let bytes = std::fs::read(data.project_files(project_id).join(file)).ok()?;
    let (_, media_type) = (bytes.len() <= MAX_LOGO)
        .then(|| kind_of(&bytes))
        .flatten()?;
    Some(data_url(media_type, &bytes))
}

/// The project's files (its logo) erased, with their folder: the project goes for good.
pub(crate) fn remove(data: &DataDir, project_id: &str) {
    if !is_plain_id(project_id) {
        return;
    }
    let dir = data.project_files(project_id);
    match std::fs::remove_dir_all(&dir) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => log::warn!("files of project {project_id} not removed: {e}"),
    }
}

/// The project's logo removed, the folder kept empty.
pub(crate) fn clear(data: &DataDir, project_id: &str) {
    if is_plain_id(project_id) {
        keep_only(&data.project_files(project_id), None);
    }
}

/// « Lettres de la tuile » as saved: up to `MAX_LETTERS` letters or digits, else one emoji, else
/// nothing (the initials of the name). The window has cut it already: this keeps a state edited
/// by hand, or an older window, from filling the tile.
pub(crate) fn clean_letters(text: &str) -> String {
    // Spaces and controls are no letters of a tile.
    let text: String = text
        .nfc()
        .filter(|c| !c.is_whitespace() && !c.is_control())
        .collect();
    if text.chars().any(char::is_alphanumeric) {
        return text
            .chars()
            .filter(|c| c.is_alphanumeric())
            .take(MAX_LETTERS)
            .collect();
    }
    // An emoji, with its joiners and modifiers: a family is up to 11 characters.
    let pictograph = text.chars().any(|c| c as u32 >= 0x2190);
    if pictograph && text.chars().count() <= 11 {
        text
    } else {
        String::new()
    }
}

/// The most characters of « Libellé sous la tuile » kept (ESC-168). The window takes twelve
/// letters, an emoji's joiners and accents aside; this keeps a state edited by hand from filling
/// the rail.
pub(crate) const MAX_LABEL_CHARS: usize = 48;

/// « Libellé sous la tuile » as saved: on one line (its spaces and controls one space), trimmed, cut
/// at `MAX_LABEL_CHARS`; empty is the name of the project.
pub(crate) fn clean_label(text: &str) -> String {
    let line: Vec<&str> = text
        .split(|c: char| c.is_whitespace() || c.is_control())
        .filter(|w| !w.is_empty())
        .collect();
    line.join(" ")
        .nfc()
        .take(MAX_LABEL_CHARS)
        .collect::<String>()
        .trim_end()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Lang::{En, Fr};
    use crate::paths::test_dir;

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n-logo";
    const SVG: &str = "<?xml version=\"1.0\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\"><script>alert(1)</script></svg>";

    fn file(dir: &Path, name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, bytes).unwrap();
        p
    }

    fn refused(lang: Lang, path: &Path) -> String {
        match read_in(lang, path) {
            Err(e) => e.to_string(),
            Ok(l) => panic!("accepted as {}", l.media_type),
        }
    }

    #[test]
    fn the_five_formats_are_known_by_their_bytes_not_their_extension() {
        let dir = test_dir("logo-formats");
        let cases: [(&str, Vec<u8>, &str); 6] = [
            ("a.png", PNG.to_vec(), "image/png"),
            ("b.jpeg", vec![0xFF, 0xD8, 0xFF, 0xE0, 1, 2], "image/jpeg"),
            ("c.webp", b"RIFF\x10\0\0\0WEBPVP8 ".to_vec(), "image/webp"),
            ("d.ico", vec![0, 0, 1, 0, 1, 0, 16, 16], "image/x-icon"),
            ("e.svg", SVG.as_bytes().to_vec(), "image/svg+xml"),
            // A PNG named .txt is a PNG.
            ("f.txt", PNG.to_vec(), "image/png"),
        ];
        for (name, bytes, media_type) in cases {
            let logo = read_in(Fr, &file(&dir, name, &bytes)).unwrap();
            assert_eq!(logo.media_type, media_type, "{name}");
            assert_eq!(logo.bytes, bytes);
        }
        // A BOM and spaces before an SVG's first tag.
        let svg = format!("\u{feff}  {SVG}");
        assert_eq!(
            read_in(Fr, &file(&dir, "g.svg", svg.as_bytes()))
                .unwrap()
                .ext,
            "svg"
        );
    }

    #[test]
    fn a_file_too_heavy_unreadable_or_of_another_format_is_refused_saying_why() {
        let dir = test_dir("logo-refused");
        let mut big = PNG.to_vec();
        big.resize(MAX_LOGO + 1, 0);
        let big = file(&dir, "big.png", &big);
        assert_eq!(
            refused(Fr, &big),
            "« big.png » pèse 1 Mo : un logo pèse 1 Mo au plus."
        );
        assert_eq!(
            refused(En, &big),
            "“big.png” weighs 1 MB: a logo weighs 1 MB at most."
        );
        // One byte under the limit is taken.
        let mut max = PNG.to_vec();
        max.resize(MAX_LOGO, 0);
        assert!(read_in(Fr, &file(&dir, "max.png", &max)).is_ok());

        let gif = file(&dir, "anim.gif", b"GIF89a....");
        assert_eq!(
            refused(Fr, &gif),
            "« anim.gif » n’est pas une image PNG, JPEG, WebP, SVG ou ICO."
        );
        assert_eq!(
            refused(En, &gif),
            "“anim.gif” isn’t a PNG, JPEG, WebP, SVG or ICO image."
        );
        let text = file(&dir, "notes.svg", b"pas une image <svg> du tout");
        assert!(refused(Fr, &text).contains("n’est pas une image"));
        let empty = file(&dir, "vide.png", b"");
        assert!(refused(Fr, &empty).contains("n’est pas une image"));

        let gone = dir.join("parti.png");
        assert!(refused(Fr, &gone).starts_with("Impossible de lire « parti.png » : "));
        assert!(refused(En, &gone).starts_with("Can’t read “parti.png”: "));
        // A folder is no image either.
        assert!(read_in(Fr, &dir).is_err());
    }

    #[test]
    fn a_logo_is_copied_under_the_project_and_outlives_its_file() {
        let root = test_dir("logo-import");
        let data = DataDir::new(root.join("data"));
        let source = file(&root, "logo.png", PNG);
        let name = import(Fr, &data, "p1", &source).unwrap();
        assert!(plain_name(&name) && name.starts_with("logo-") && name.ends_with(".png"));
        std::fs::remove_file(&source).unwrap();
        let url = url_of(&data, "p1", &name).unwrap();
        assert_eq!(url, data_url("image/png", PNG));
        assert!(url.starts_with("data:image/png;base64,"));

        // Another logo takes its place, the first one's file gone; a refusal keeps it.
        let svg = file(&root, "logo.svg", SVG.as_bytes());
        let second = import(Fr, &data, "p1", &svg).unwrap();
        assert_ne!(second, name);
        assert!(url_of(&data, "p1", &name).is_none());
        let files: Vec<_> = std::fs::read_dir(data.project_files("p1"))
            .unwrap()
            .collect();
        assert_eq!(files.len(), 1);
        assert!(import(Fr, &data, "p1", &file(&root, "x.gif", b"GIF89a")).is_err());
        assert!(url_of(&data, "p1", &second)
            .unwrap()
            .starts_with("data:image/svg+xml;base64,"));

        // Removed, the folder goes; cleared, the logo alone.
        clear(&data, "p1");
        assert!(url_of(&data, "p1", &second).is_none());
        assert!(data.project_files("p1").is_dir());
        import(Fr, &data, "p1", &svg).unwrap();
        remove(&data, "p1");
        assert!(!data.project_files("p1").exists());
        remove(&data, "p1");
    }

    #[test]
    fn a_hand_edited_name_never_reads_outside_the_projects_folder() {
        let root = test_dir("logo-outside");
        let data = DataDir::new(root.join("data"));
        file(&root, "secret.png", PNG);
        std::fs::create_dir_all(data.project_files("p1")).unwrap();
        for name in ["../../secret.png", "..\\..\\secret.png", "C:secret.png", ""] {
            assert!(url_of(&data, "p1", name).is_none(), "{name}");
        }
        assert!(url_of(&data, "..", "secret.png").is_none());
        assert!(import(Fr, &data, "../x", &root.join("secret.png")).is_err());
        remove(&data, "..");
        assert!(root.join("secret.png").exists());
    }

    #[test]
    fn the_letters_are_three_letters_or_digits_or_one_emoji() {
        assert_eq!(clean_letters(""), "");
        assert_eq!(clean_letters("  "), "");
        assert_eq!(clean_letters("ab"), "ab");
        assert_eq!(clean_letters(" API "), "API");
        assert_eq!(clean_letters("ABCD"), "ABC");
        assert_eq!(clean_letters("v2"), "v2");
        assert_eq!(clean_letters("É.1"), "É1");
        // Decomposed accents count as one letter.
        assert_eq!(clean_letters("e\u{301}tat"), "éta");
        assert_eq!(clean_letters("🚀"), "🚀");
        assert_eq!(clean_letters("👩‍💻"), "👩‍💻");
        assert_eq!(clean_letters("a b"), "ab");
        assert_eq!(clean_letters("\u{7}"), "");
        assert_eq!(clean_letters("-."), "");
        assert_eq!(clean_letters(&"🚀".repeat(20)), "");
    }

    #[test]
    fn the_label_is_one_line_trimmed_and_bounded() {
        assert_eq!(clean_label(""), "");
        assert_eq!(clean_label("   "), "");
        assert_eq!(clean_label(" API  v2 "), "API v2");
        assert_eq!(clean_label("a\nb\t\u{7}c"), "a b c");
        assert_eq!(clean_label("e\u{301}tat"), "\u{e9}tat");
        assert_eq!(
            clean_label(&"x".repeat(100)).chars().count(),
            MAX_LABEL_CHARS
        );
    }
}
