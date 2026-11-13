//! Claude Code installed in WSL, run from Windows: `wsl.exe --exec`, its paths translated both
//! ways (`/mnt/c/…` for what Escouade keeps on Windows, `\\wsl.localhost\<distro>\…` for what
//! Claude Code keeps in Linux).

use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// A program named in the settings that lives in WSL, before it is looked for: the distribution
/// named in it (`\\wsl.localhost\<distro>\…`, `\\wsl$\<distro>\…`), none for a Linux path (the
/// distribution of the settings, else the default one), and its Linux path, `~/…` for one in the
/// home folder.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Spec {
    pub distro: Option<String>,
    pub path: String,
}

/// A program found in a WSL distribution: what it takes to run it, and to reach its files from
/// Windows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    /// `wsl.exe`.
    pub exe: PathBuf,
    pub distro: String,
    /// Its absolute Linux path.
    pub path: String,
    /// The home folder of the distribution's default user, the one `wsl.exe` runs it as.
    pub home: String,
    /// Where the distribution mounts the Windows drives (`/mnt/`).
    pub mount_root: String,
}

impl Program {
    /// The Linux path of a Windows one (see `to_linux`).
    pub fn linux_path(&self, windows: &str) -> Option<String> {
        to_linux(windows, &self.distro, &self.mount_root)
    }

    /// The Windows path of a Linux one (see `to_windows`).
    pub fn windows_path(&self, linux: &str) -> PathBuf {
        PathBuf::from(to_windows(linux, &self.distro, &self.mount_root))
    }
}

/// What WSL has on this machine.
#[derive(Debug, Clone, Default)]
pub struct Machine {
    /// `wsl.exe`, when WSL is installed.
    pub exe: Option<PathBuf>,
    /// The distributions installed: their name and the user `wsl.exe` runs programs as (uid).
    pub distros: Vec<(String, u32)>,
    /// The default distribution's name.
    pub default: Option<String>,
    /// Where Windows sees the distributions' files: `\\wsl.localhost`.
    pub share: PathBuf,
}

/// Where the native installer, the old local install and npm put `claude` in Linux, in the order
/// they are looked for when the settings name none and Windows has none.
const USUAL_PLACES: [&str; 4] = [
    "~/.local/bin/claude",
    "~/.claude/local/claude",
    "/usr/local/bin/claude",
    "/usr/bin/claude",
];

/// The program `spec` names, looked for in its distribution (`setting`: the distribution of the
/// settings, empty for the default one): None without WSL, the distribution or the file.
pub fn find_in(spec: &Spec, setting: &str, machine: &Machine) -> Option<Program> {
    let exe = machine.exe.clone()?;
    let wanted = spec
        .distro
        .as_deref()
        .or(Some(setting.trim()).filter(|s| !s.is_empty()))
        .or(machine.default.as_deref())?;
    let (distro, uid) = machine
        .distros
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(wanted))?;
    let root = machine.share.join(distro);
    let read = |linux: &str| std::fs::read_to_string(under(&root, linux)).unwrap_or_default();
    let (conf, passwd) = (read("/etc/wsl.conf"), read("/etc/passwd"));
    // The user wsl.exe runs programs as: the one wsl.conf names (an imported distribution's,
    // whose registry says root), else the registry's; asked to the distribution when its files
    // say nothing.
    let home = default_user(&conf)
        .and_then(|user| home_named(&passwd, &user))
        .or_else(|| home_of(&passwd, *uid))
        .or_else(|| ask_home(&exe, distro));
    let path = match spec.path.strip_prefix('~') {
        Some(rest) => format!("{}{rest}", home.as_deref()?.trim_end_matches('/')),
        None => spec.path.clone(),
    };
    let found = present(&under(&root, &path)) || there_in(&exe, distro, &path);
    found.then(|| Program {
        exe,
        distro: distro.clone(),
        path,
        home: home.unwrap_or_else(|| "/root".into()),
        mount_root: mount_root(&conf),
    })
}

/// The Linux path `linux` under the folder `root` that shows the distribution's `/`.
fn under(root: &Path, linux: &str) -> PathBuf {
    let mut out = root.to_path_buf();
    out.extend(linux.split('/').filter(|c| !c.is_empty()));
    out
}

/// A file or a link: Windows cannot follow the links of a distribution (`is_file` says no to
/// them), and the native installer's `~/.local/bin/claude` is one.
fn present(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|m| !m.is_dir())
}

/// Whether the file `path` is there for Linux, asked to the distribution itself: the links of a
/// folder of its path are followed (`/bin` is one in most distributions), which the share cannot
/// do.
fn there_in(exe: &Path, distro: &str, path: &str) -> bool {
    run_in(exe, distro, r#"[ -f "$0" ]"#, path).is_some_and(|(ok, _)| ok)
}

/// The home folder of the user `wsl.exe` runs programs as in `distro`, asked to it.
fn ask_home(exe: &Path, distro: &str) -> Option<String> {
    run_in(exe, distro, r#"printf %s "$HOME""#, "sh")
        .filter(|(ok, home)| *ok && home.starts_with('/'))
        .map(|(_, home)| home)
}

/// What `script` (`sh -c`, `$0` being `arg`) writes in `distro`, and whether it succeeded; None
/// when WSL does not answer within a while (a distribution that starts takes seconds).
fn run_in(exe: &Path, distro: &str, script: &str, arg: &str) -> Option<(bool, String)> {
    const ANSWER_WITHIN: Duration = Duration::from_secs(20);
    let mut cmd = std::process::Command::new(exe);
    cmd.args(["-d", distro, "--exec", "/bin/sh", "-c", script, arg])
        .env("WSL_UTF8", "1")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(crate::proc::CREATE_NO_WINDOW);
    }
    let mut child = cmd.spawn().ok()?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            // A few bytes at most: the pipe never fills before the end.
            Ok(Some(status)) => {
                let mut out = String::new();
                if let Some(mut stdout) = child.stdout.take() {
                    let _ = std::io::Read::read_to_string(&mut stdout, &mut out);
                }
                return Some((status.success(), out));
            }
            Ok(None) if started.elapsed() < ANSWER_WITHIN => {
                std::thread::sleep(Duration::from_millis(30))
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}

/// How long a program not found stays so before it is looked for again by itself: a look into a
/// stopped distribution starts it (seconds, and the VM it keeps up).
const MISSING_FOR: Duration = Duration::from_secs(300);

type Found = HashMap<(Spec, String), (Instant, Option<Program>)>;

/// What `find` found or did not, and when.
static FOUND: Mutex<Option<Found>> = Mutex::new(None);

/// `find_in` on this machine, kept, for what asks often (the board, the quota, the hooks): a
/// program found stays found, one that is not is looked for again after `MISSING_FOR`, or at once
/// after `forget`.
pub fn find(spec: &Spec, setting: &str) -> Option<Program> {
    let key = (spec.clone(), setting.trim().to_string());
    if let Some((at, found)) = FOUND.lock().get_or_insert_with(HashMap::new).get(&key) {
        if found.is_some() || at.elapsed() < MISSING_FOR {
            return found.clone();
        }
    }
    let found = find_in(spec, setting, &Machine::this());
    let mut kept = FOUND.lock();
    let kept = kept.get_or_insert_with(HashMap::new);
    // What was not found long ago goes: the prefixes typed in the field of the settings.
    kept.retain(|_, (at, found)| found.is_some() || at.elapsed() < MISSING_FOR);
    kept.insert(key, (Instant::now(), found.clone()));
    found
}

/// Forgets what `find` did not find, and what it found too when `all`: the user asks again (the
/// field of the settings, a path saved), maybe after installing Claude Code or removing it.
pub fn forget(all: bool) {
    if let Some(kept) = FOUND.lock().as_mut() {
        kept.retain(|_, (_, found)| !all && found.is_some());
    }
}

/// `claude` where it usually is in the distribution of the settings (the default one when they
/// name none): the settings name no program and Windows has none.
pub fn detect(setting: &str) -> Option<Program> {
    USUAL_PLACES.iter().find_map(|path| {
        let spec = Spec {
            distro: None,
            path: path.to_string(),
        };
        find(&spec, setting)
    })
}

impl Machine {
    /// This machine's, from the registry where WSL keeps its distributions.
    #[cfg(windows)]
    pub fn this() -> Self {
        const LXSS: &str = r"Software\Microsoft\Windows\CurrentVersion\Lxss";
        let system_root = std::env::var_os("SystemRoot").unwrap_or_else(|| r"C:\Windows".into());
        let exe = Path::new(&system_root).join(r"System32\wsl.exe");
        let default_key = registry::string(LXSS, "DefaultDistribution");
        let mut distros = Vec::new();
        let mut default = None;
        for key in registry::subkeys(LXSS) {
            let sub = format!(r"{LXSS}\{key}");
            let Some(name) = registry::string(&sub, "DistributionName") else {
                continue;
            };
            // 1: installed (not being installed or removed).
            if registry::dword(&sub, "State").is_some_and(|s| s != 1) {
                continue;
            }
            if default_key.as_deref() == Some(key.as_str()) {
                default = Some(name.clone());
            }
            distros.push((name, registry::dword(&sub, "DefaultUid").unwrap_or(0)));
        }
        Machine {
            exe: exe.is_file().then_some(exe),
            distros,
            default,
            share: PathBuf::from(r"\\wsl.localhost"),
        }
    }

    /// No WSL away from Windows.
    #[cfg(not(windows))]
    pub fn this() -> Self {
        Machine::default()
    }
}

/// The values WSL keeps in the user's registry (`HKEY_CURRENT_USER`).
#[cfg(windows)]
mod registry {
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegEnumKeyExW, RegGetValueW, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER, KEY_READ,
        RRF_RT_REG_DWORD, RRF_RT_REG_SZ,
    };

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain([0]).collect()
    }

    pub fn string(key: &str, value: &str) -> Option<String> {
        let (k, v) = (wide(key), wide(value));
        let mut size = 0u32;
        // SAFETY: the names end with a NUL; the first call only asks for the size in bytes.
        let first = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                k.as_ptr(),
                v.as_ptr(),
                RRF_RT_REG_SZ,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut size,
            )
        };
        if first != 0 || size == 0 {
            return None;
        }
        let mut buf = vec![0u16; size as usize / 2 + 1];
        let mut bytes = (buf.len() * 2) as u32;
        // SAFETY: `buf` holds `bytes` bytes.
        let read = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                k.as_ptr(),
                v.as_ptr(),
                RRF_RT_REG_SZ,
                std::ptr::null_mut(),
                buf.as_mut_ptr().cast(),
                &mut bytes,
            )
        };
        if read != 0 {
            return None;
        }
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        Some(String::from_utf16_lossy(&buf[..len]))
    }

    pub fn dword(key: &str, value: &str) -> Option<u32> {
        let (k, v) = (wide(key), wide(value));
        let mut out = 0u32;
        let mut size = 4u32;
        // SAFETY: `out` holds the 4 bytes of a DWORD.
        let read = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                k.as_ptr(),
                v.as_ptr(),
                RRF_RT_REG_DWORD,
                std::ptr::null_mut(),
                (&mut out as *mut u32).cast(),
                &mut size,
            )
        };
        (read == 0).then_some(out)
    }

    pub fn subkeys(key: &str) -> Vec<String> {
        let k = wide(key);
        let mut handle: HKEY = std::ptr::null_mut();
        // SAFETY: the name ends with a NUL; the key is closed below.
        if unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, k.as_ptr(), 0, KEY_READ, &mut handle) } != 0 {
            return Vec::new();
        }
        let mut out = Vec::new();
        for index in 0.. {
            // A key's name is at most 255 characters.
            let mut name = [0u16; 256];
            let mut len = name.len() as u32;
            // SAFETY: `name` holds `len` characters; the other outputs are not asked for.
            let r = unsafe {
                RegEnumKeyExW(
                    handle,
                    index,
                    name.as_mut_ptr(),
                    &mut len,
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            };
            if r != 0 {
                break;
            }
            out.push(String::from_utf16_lossy(&name[..len as usize]));
        }
        // SAFETY: opened above.
        unsafe { RegCloseKey(handle) };
        out
    }
}

/// The program of `configured` when it is one in WSL: a path to the WSL share, a Linux path, or
/// one in the Linux home folder (`~/…`, when it names no Windows program: see
/// `Engine::locate`).
pub fn parse(configured: &str) -> Option<Spec> {
    let given = configured.trim();
    let slashed = given.replace('\\', "/");
    if let Some((distro, path)) = share_path(&slashed) {
        return (!distro.is_empty() && path != "/").then(|| Spec {
            distro: Some(distro.to_string()),
            path,
        });
    }
    // `~` either way (`~\.local\bin\claude` is a Linux path typed with Windows' separators), but
    // `/` only as Linux writes it: `\tools\claude.exe` is a folder of the current drive.
    let linux = slashed == "~" || slashed.starts_with("~/") || given.starts_with('/');
    (linux && !slashed.starts_with("//")).then_some(Spec {
        distro: None,
        path: slashed,
    })
}

/// Whether `path` is in a distribution of WSL, seen from Windows (`\\wsl.localhost\…`,
/// `\\wsl$\…`).
pub fn on_share(path: &Path) -> bool {
    share_path(&path.to_string_lossy().replace('\\', "/")).is_some()
}

/// The distribution and the Linux path (`/` for its root) of a path to the WSL share written with
/// `/` (`//wsl.localhost/<distro>/…`, `//wsl$/<distro>/…`); None for any other.
pub fn share_path(slashed: &str) -> Option<(&str, String)> {
    let (host, rest) = slashed.strip_prefix("//")?.split_once('/')?;
    if !(host.eq_ignore_ascii_case("wsl.localhost") || host.eq_ignore_ascii_case("wsl$")) {
        return None;
    }
    let (distro, path) = rest.split_once('/').unwrap_or((rest, ""));
    Some((distro, format!("/{}", path.trim_end_matches('/'))))
}

/// Where WSL mounts the Windows drives, as `[automount] root` of `/etc/wsl.conf` (`text`) says:
/// `/mnt/` by default, always with its last `/`.
pub fn mount_root(text: &str) -> String {
    ini_value(text, "automount", "root")
        .filter(|root| root.starts_with('/'))
        .map(|root| format!("{}/", root.trim_end_matches('/')))
        .unwrap_or_else(|| "/mnt/".to_string())
}

/// Whether WSL shares the network of Windows, as `[wsl2] networkingMode` of `.wslconfig` (`text`)
/// says (`mirrored`): only then does a program of WSL reach a server on 127.0.0.1 of Windows
/// (Escouade's MCP server, the hooks of the Claude Code view). NAT by default.
pub fn mirrored(text: &str) -> bool {
    ini_value(text, "wsl2", "networkingMode").is_some_and(|m| m.eq_ignore_ascii_case("mirrored"))
}

/// The user `wsl.exe` runs programs as, when `[user] default` of `/etc/wsl.conf` (`text`) names
/// one (an imported distribution's: its registry says root).
pub fn default_user(text: &str) -> Option<String> {
    ini_value(text, "user", "default").filter(|user| !user.is_empty())
}

/// The value of `key` in `[section]` of the INI file `text` (WSL's: `#` or `;` start a comment,
/// names are read whatever their case), without its quotes; the first one.
fn ini_value(text: &str, section: &str, key: &str) -> Option<String> {
    let mut current = String::new();
    for line in text.lines() {
        let line = line.split(['#', ';']).next().unwrap_or("").trim();
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            current = name.trim().to_string();
        } else if let Some((k, v)) = line.split_once('=') {
            if current.eq_ignore_ascii_case(section) && k.trim().eq_ignore_ascii_case(key) {
                return Some(v.trim().trim_matches('"').to_string());
            }
        }
    }
    None
}

/// `mirrored` for this machine's `%UserProfile%\.wslconfig`.
pub fn mirrored_here() -> bool {
    dirs::home_dir()
        .and_then(|home| std::fs::read_to_string(home.join(".wslconfig")).ok())
        .is_some_and(|text| mirrored(&text))
}

/// The home folder of the user `uid` in `/etc/passwd` (`text`).
pub fn home_of(text: &str, uid: u32) -> Option<String> {
    passwd_home(text, |fields| fields[2].parse() == Ok(uid))
}

/// The home folder of the user `name` in `/etc/passwd` (`text`).
pub fn home_named(text: &str, name: &str) -> Option<String> {
    passwd_home(text, |fields| fields[0] == name)
}

/// The home folder of the first entry of `/etc/passwd` (`text`) that `wanted` takes.
fn passwd_home(text: &str, wanted: impl Fn(&[&str]) -> bool) -> Option<String> {
    text.lines().find_map(|line| {
        let fields: Vec<&str> = line.split(':').collect();
        (fields.len() >= 7 && wanted(&fields)).then(|| fields[5].to_string())
    })
}

/// The Linux path of the Windows path `path` for a program of `distro`: a drive's under
/// `mount_root`, the distribution's own through its share; None for anything else (another
/// distribution, a network share).
pub fn to_linux(path: &str, distro: &str, mount_root: &str) -> Option<String> {
    let path = path.trim();
    let slashed = path
        .strip_prefix(r"\\?\")
        .unwrap_or(path)
        .replace('\\', "/");
    if slashed.starts_with("//") {
        let (d, linux) = share_path(&slashed)?;
        return d.eq_ignore_ascii_case(distro).then_some(linux);
    }
    if slashed.starts_with('/') {
        return Some(slashed);
    }
    let b = slashed.as_bytes();
    if b.len() < 2 || !b[0].is_ascii_alphabetic() || b[1] != b':' || (b.len() > 2 && b[2] != b'/') {
        return None;
    }
    let drive = (b[0] as char).to_ascii_lowercase();
    let rest = slashed[2..].trim_matches('/');
    Some(if rest.is_empty() {
        format!("{mount_root}{drive}")
    } else {
        format!("{mount_root}{drive}/{rest}")
    })
}

/// The Windows path of the Linux path `path` of `distro`: a drive's under `mount_root`, any other
/// through the distribution's share.
pub fn to_windows(path: &str, distro: &str, mount_root: &str) -> String {
    if let Some(rest) = path.strip_prefix(mount_root) {
        let (drive, tail) = rest.split_once('/').unwrap_or((rest, ""));
        if drive.len() == 1 && drive.as_bytes()[0].is_ascii_alphabetic() {
            return format!(
                r"{}:\{}",
                drive.to_ascii_uppercase(),
                tail.replace('/', r"\")
            );
        }
    }
    format!(r"\\wsl.localhost\{distro}{}", path.replace('/', r"\"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(distro: Option<&str>, path: &str) -> Option<Spec> {
        Some(Spec {
            distro: distro.map(str::to_string),
            path: path.to_string(),
        })
    }

    #[test]
    fn reads_the_paths_the_issue_tried() {
        // The three forms of the issue (GitHub #3), with the share's two names and either slash.
        assert_eq!(
            parse(r"\\wsl.localhost\Ubuntu-24.04\home\me\.local\bin\claude"),
            spec(Some("Ubuntu-24.04"), "/home/me/.local/bin/claude")
        );
        assert_eq!(
            parse(r"\\wsl$\Ubuntu\usr\local\bin\claude"),
            spec(Some("Ubuntu"), "/usr/local/bin/claude")
        );
        assert_eq!(
            parse("//WSL.localhost/Debian/opt/claude"),
            spec(Some("Debian"), "/opt/claude")
        );
        assert_eq!(
            parse("~/.local/bin/claude"),
            spec(None, "~/.local/bin/claude")
        );
        assert_eq!(
            parse(r"~\.local\bin\claude"),
            spec(None, "~/.local/bin/claude")
        );
        assert_eq!(
            parse("  /home/me/.local/bin/claude "),
            spec(None, "/home/me/.local/bin/claude")
        );
    }

    #[test]
    fn leaves_windows_paths_and_bare_names_to_windows() {
        for path in [
            r"C:\Users\me\.local\bin\claude.exe",
            "C:/tools/claude.cmd",
            r"\\server\share\claude.exe",
            "claude",
            "",
            r"\\wsl.localhost\Ubuntu",
            r"\\wsl.localhost\\home\me",
        ] {
            assert_eq!(parse(path), None, "{path}");
        }
    }

    #[test]
    fn knows_whether_wsl_shares_the_network_of_windows() {
        assert!(!mirrored(""));
        assert!(!mirrored("[wsl2]\nmemory=8GB\n"));
        assert!(mirrored("[wsl2]\nnetworkingMode=mirrored\n"));
        assert!(mirrored(
            "[WSL2]\r\n  networkingmode = Mirrored  # Windows 11\r\n"
        ));
        assert!(!mirrored("[wsl2]\nnetworkingMode=NAT\n"));
        assert!(!mirrored("[experimental]\nnetworkingMode=mirrored\n"));
    }

    #[test]
    fn knows_a_folder_of_a_distribution() {
        assert!(on_share(Path::new(
            r"\\wsl.localhost\Ubuntu\home\me\.claude"
        )));
        assert!(on_share(Path::new(r"\\WSL$\Debian\root")));
        assert!(!on_share(Path::new(r"C:\Users\me\.claude")));
        assert!(!on_share(Path::new(r"\\server\share\x")));
    }

    #[test]
    fn finds_where_the_drives_are_mounted() {
        assert_eq!(mount_root(""), "/mnt/");
        assert_eq!(mount_root("[boot]\nsystemd=true\n"), "/mnt/");
        assert_eq!(mount_root("[automount]\nenabled = true\nroot = /\n"), "/");
        assert_eq!(
            mount_root("[automount]\r\n  root=/win  # drives\r\n"),
            "/win/"
        );
        // The key of another section is not the drives'.
        assert_eq!(
            mount_root("[other]\nroot = /x/\n[automount]\noptions = metadata\n"),
            "/mnt/"
        );
    }

    #[test]
    fn finds_a_users_home_in_passwd() {
        let passwd = "root:x:0:0:root:/root:/bin/bash\nme:x:1000:1000:,,,:/home/me:/bin/bash\n";
        assert_eq!(home_of(passwd, 1000).as_deref(), Some("/home/me"));
        assert_eq!(home_of(passwd, 0).as_deref(), Some("/root"));
        assert_eq!(home_of(passwd, 1001), None);
        assert_eq!(home_of("broken line\n", 1000), None);
    }

    #[test]
    fn translates_windows_paths_for_linux() {
        let linux = |p: &str| to_linux(p, "Ubuntu", "/mnt/");
        assert_eq!(
            linux(r"C:\Users\me\proj").as_deref(),
            Some("/mnt/c/Users/me/proj")
        );
        assert_eq!(linux("D:/work/a b/").as_deref(), Some("/mnt/d/work/a b"));
        assert_eq!(linux(r"C:\").as_deref(), Some("/mnt/c"));
        assert_eq!(
            linux(r"\\?\C:\Users\me").as_deref(),
            Some("/mnt/c/Users/me")
        );
        assert_eq!(
            linux(r"\\wsl.localhost\ubuntu\home\me").as_deref(),
            Some("/home/me")
        );
        assert_eq!(linux(r"\\wsl$\Ubuntu\home\me").as_deref(), Some("/home/me"));
        assert_eq!(linux(r"\\wsl.localhost\Ubuntu").as_deref(), Some("/"));
        assert_eq!(linux("/home/me").as_deref(), Some("/home/me"));
        assert_eq!(linux(r"\\wsl.localhost\Debian\home\me"), None);
        assert_eq!(linux(r"\\server\share\x"), None);
        assert_eq!(linux("relative\\path"), None);
        assert_eq!(
            to_linux(r"C:\Users\me", "Ubuntu", "/").as_deref(),
            Some("/c/Users/me")
        );
    }

    /// A machine whose share is a test folder holding the distribution `Ubuntu` (user 1000, `me`,
    /// drives under `/win/`) with `claude` in `~/.local/bin`, and `Debian` with nothing.
    fn machine(name: &str) -> Machine {
        // Built a component at a time: the tests run on every system.
        let share = escouade_base::testing::test_dir(name);
        let ubuntu = share.join("Ubuntu");
        std::fs::create_dir_all(under(&ubuntu, "/etc")).unwrap();
        std::fs::create_dir_all(under(&ubuntu, "/home/me/.local/bin")).unwrap();
        std::fs::create_dir_all(under(&share, "/Debian/etc")).unwrap();
        std::fs::write(
            under(&ubuntu, "/etc/passwd"),
            "root:x:0:0:root:/root:/bin/bash\nme:x:1000:1000:,,,:/home/me:/bin/bash\n",
        )
        .unwrap();
        std::fs::write(
            under(&ubuntu, "/etc/wsl.conf"),
            "[automount]\nroot = /win\n",
        )
        .unwrap();
        std::fs::write(under(&ubuntu, "/home/me/.local/bin/claude"), "ELF").unwrap();
        Machine {
            exe: Some(share.join("wsl.exe")),
            distros: vec![("Ubuntu".into(), 1000), ("Debian".into(), 0)],
            default: Some("Ubuntu".into()),
            share,
        }
    }

    #[test]
    fn finds_a_program_in_its_distribution() {
        let m = machine("wsl-find");
        let found = |path: &str, setting: &str| find_in(&parse(path).unwrap(), setting, &m);
        let claude = Program {
            exe: m.share.join("wsl.exe"),
            distro: "Ubuntu".into(),
            path: "/home/me/.local/bin/claude".into(),
            home: "/home/me".into(),
            mount_root: "/win/".into(),
        };
        // The forms of the issue: the share (the distribution's name as it is installed), the
        // home folder and the Linux path in the default distribution.
        assert_eq!(
            found(r"\\wsl.localhost\ubuntu\home\me\.local\bin\claude", ""),
            Some(claude.clone())
        );
        assert_eq!(found("~/.local/bin/claude", ""), Some(claude.clone()));
        assert_eq!(
            found("/home/me/.local/bin/claude", ""),
            Some(claude.clone())
        );
        // A Linux path goes to the distribution of the settings when they name one.
        assert_eq!(found("/home/me/.local/bin/claude", "Debian"), None);
        assert_eq!(found("~/.local/bin/claude", " ubuntu "), Some(claude));
        // Not there, a distribution that is not installed, a folder.
        assert_eq!(found("~/.local/bin/nothing", ""), None);
        assert_eq!(found(r"\\wsl$\Arch\usr\bin\claude", ""), None);
        assert_eq!(found("~/.local/bin", ""), None);
        // No WSL.
        let none = Machine {
            exe: None,
            ..m.clone()
        };
        assert_eq!(
            find_in(&parse("~/.local/bin/claude").unwrap(), "", &none),
            None
        );
    }

    #[test]
    fn the_default_user_of_wsl_conf_comes_before_the_registrys() {
        // An imported distribution: the registry says root, /etc/wsl.conf names the user.
        let mut m = machine("wsl-find-imported");
        let ubuntu = m.share.join("Ubuntu");
        std::fs::write(
            under(&ubuntu, "/etc/wsl.conf"),
            "[automount]\nroot = /win\n[user]\ndefault = me\n",
        )
        .unwrap();
        m.distros = vec![("Ubuntu".into(), 0)];
        let found = find_in(&parse("~/.local/bin/claude").unwrap(), "", &m).unwrap();
        assert_eq!(found.home, "/home/me");
        assert_eq!(found.path, "/home/me/.local/bin/claude");
    }

    #[test]
    fn reads_the_default_user_and_a_users_home_by_name() {
        assert_eq!(default_user("[user]\ndefault=me\n").as_deref(), Some("me"));
        assert_eq!(default_user("[boot]\nsystemd=true\n"), None);
        assert_eq!(default_user("[user]\ndefault =  \n"), None);
        let passwd = "root:x:0:0:root:/root:/bin/bash\nme:x:1000:1000:,,,:/home/me:/bin/bash\n";
        assert_eq!(home_named(passwd, "me").as_deref(), Some("/home/me"));
        assert_eq!(home_named(passwd, "you"), None);
    }

    #[test]
    fn a_link_of_a_real_distribution_is_a_program() {
        // Only where WSL has a distribution (not on the CI's Windows runners).
        let m = Machine::this();
        let (Some(_), Some(distro)) = (&m.exe, m.default.clone()) else {
            eprintln!("no WSL distribution: skipped");
            return;
        };
        let dir = format!("/tmp/escouade-test-{}", std::process::id());
        let made = std::process::Command::new(m.exe.as_ref().unwrap())
            .args(["-d", &distro, "--exec", "/bin/sh", "-c"])
            .arg(format!(
                "mkdir -p {dir} && touch {dir}/real && ln -sf {dir}/real {dir}/link"
            ))
            .status()
            .unwrap();
        assert!(made.success());
        let found = find_in(&parse(&format!("{dir}/link")).unwrap(), "", &m);
        assert_eq!(found.map(|p| p.path), Some(format!("{dir}/link")));
        let _ = std::process::Command::new(m.exe.as_ref().unwrap())
            .args(["-d", &distro, "--exec", "rm", "-rf", &dir])
            .status();
    }

    #[test]
    fn translates_linux_paths_for_windows() {
        let windows = |p: &str| to_windows(p, "Ubuntu", "/mnt/");
        assert_eq!(windows("/mnt/c/Users/me/proj"), r"C:\Users\me\proj");
        assert_eq!(windows("/mnt/d"), r"D:\");
        assert_eq!(
            windows("/home/me/.claude"),
            r"\\wsl.localhost\Ubuntu\home\me\.claude"
        );
        // A folder of /mnt that is no drive stays in the distribution.
        assert_eq!(windows("/mnt/wslg/x"), r"\\wsl.localhost\Ubuntu\mnt\wslg\x");
        assert_eq!(to_windows("/c/x", "Ubuntu", "/"), r"C:\x");
    }
}
