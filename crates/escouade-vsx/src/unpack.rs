//! A VSIX unpacked in a folder of its own: its `extension/` part alone, every entry checked before anything is written.
//! The whole archive is refused when one entry could leave the folder (`..`, an absolute path, a drive, a link) or
//! name what Windows cannot hold (`con`, `a:b`, a name ending in a dot or a space), and when it holds more files or
//! bytes than its bounds: the sizes the archive declares are not trusted, each entry is read up to what is left.

use crate::{Code, Refusal, Result};
use std::io::Read;
use std::path::Path;

/// The bounds of an archive.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// The VSIX downloaded.
    pub vsix: u64,
    /// The files of its `extension/`.
    pub files: usize,
    /// Their bytes, together.
    pub total: u64,
    /// The bytes of one.
    pub file: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            vsix: 200 << 20,
            files: 20_000,
            total: 512 << 20,
            file: 256 << 20,
        }
    }
}

/// What was unpacked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unpacked {
    pub files: usize,
    pub bytes: u64,
}

fn trapped(why: String) -> Refusal {
    Refusal::new(
        Code::Trapped,
        tr!(
            "L’archive est refusée : {why}. Rien n’en est installé.",
            "The archive is refused: {why}. Nothing of it is installed."
        ),
    )
}

const RESERVED: &[&str] = &[
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// Why the name of an entry may not be written, if it may not.
pub fn bad_name(name: &str) -> Option<String> {
    if name.is_empty() {
        return Some(tr!("une entrée sans nom", "an entry with no name"));
    }
    if name.starts_with('/') || name.starts_with('\\') {
        return Some(tr!(
            "« {name} » est un chemin absolu",
            "“{name}” is an absolute path"
        ));
    }
    if name.contains('\\') || name.contains(':') || name.contains('\0') {
        return Some(tr!(
            "« {name} » a un caractère interdit dans un chemin",
            "“{name}” holds a character a path may not hold"
        ));
    }
    for part in name.trim_end_matches('/').split('/') {
        if part == ".." {
            return Some(tr!(
                "« {name} » remonte hors du dossier de l’extension",
                "“{name}” climbs out of the extension’s folder"
            ));
        }
        let stem = part.split('.').next().unwrap_or(part).to_lowercase();
        if part.is_empty()
            || part == "."
            || part.ends_with('.')
            || part.ends_with(' ')
            || RESERVED.contains(&stem.as_str())
        {
            return Some(tr!(
                "« {name} » n’est pas un nom de fichier qu’Escouade écrit",
                "“{name}” is not a file name Escouade writes"
            ));
        }
    }
    None
}

/// The unix mode of a link (`S_IFLNK`).
const S_IFLNK: u32 = 0o120000;
const S_IFMT: u32 = 0o170000;

/// The `extension/` of `vsix` unpacked into `into` (a new, empty folder), within `limits`.
pub fn unpack(vsix: &[u8], into: &Path, limits: &Limits) -> Result<Unpacked> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(vsix)).map_err(|e| {
        trapped(tr!(
            "ce n’est pas une archive lisible ({e})",
            "it is not an archive that can be read ({e})"
        ))
    })?;
    // Every name first: nothing is written of an archive that holds one trap.
    let mut wanted: Vec<(usize, String)> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for i in 0..zip.len() {
        let entry = zip.by_index_raw(i).map_err(|e| trapped(e.to_string()))?;
        let name = entry
            .name()
            .map_err(|e| trapped(e.to_string()))?
            .into_owned();
        if let Some(why) = bad_name(&name) {
            return Err(trapped(why));
        }
        if entry.unix_mode().is_some_and(|m| m & S_IFMT == S_IFLNK) {
            return Err(trapped(tr!("« {name} » est un lien", "“{name}” is a link")));
        }
        if entry.enclosed_name().is_none() {
            return Err(trapped(tr!(
                "« {name} » sort du dossier de l’extension",
                "“{name}” leaves the extension’s folder"
            )));
        }
        let Some(rel) = name.strip_prefix("extension/") else {
            continue;
        };
        if entry.is_dir() || rel.is_empty() {
            continue;
        }
        // Two names a case-insensitive disk takes for one.
        if !seen.insert(rel.to_lowercase()) {
            return Err(trapped(tr!(
                "« {name} » est en double",
                "“{name}” is there twice"
            )));
        }
        wanted.push((i, rel.to_owned()));
        if wanted.len() > limits.files {
            let max = limits.files;
            return Err(trapped(tr!(
                "plus de {max} fichiers",
                "more than {max} files"
            )));
        }
    }
    if !wanted.iter().any(|(_, r)| r == "package.json") {
        return Err(trapped(tr!(
            "elle n’a pas de extension/package.json",
            "it has no extension/package.json"
        )));
    }
    let mut total = 0u64;
    for (i, rel) in &wanted {
        let mut entry = zip.by_index(*i).map_err(|e| trapped(e.to_string()))?;
        let room = limits.file.min(limits.total - total);
        let mut bytes = Vec::new();
        entry
            .by_ref()
            .take(room + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| trapped(e.to_string()))?;
        if bytes.len() as u64 > room {
            let mb = limits.total >> 20;
            return Err(trapped(tr!(
                "elle se décompresse en plus de {mb} Mo (« {rel} »)",
                "it unpacks to more than {mb} MB (“{rel}”)"
            )));
        }
        total += bytes.len() as u64;
        let path = into.join(rel);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(Refusal::io)?;
        }
        std::fs::write(&path, &bytes).map_err(Refusal::io)?;
    }
    Ok(Unpacked {
        files: wanted.len(),
        bytes: total,
    })
}
