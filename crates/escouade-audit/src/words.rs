//! The strings an event may carry, each of a fixed grammar (`never.rs`): what does not fit is refused, a token
//! anywhere is refused, and the two free kinds (a name, a reason) are scrubbed.

use std::fmt;

use serde::de::{self, Deserializer, Visitor};
use serde::{Deserialize, Serialize, Serializer};
use sha2::{Digest as _, Sha256};

use crate::never;

/// Why a string was not taken into an event.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Refused {
    #[error("not a valid {0}")]
    Shape(&'static str),
    #[error("a {0} that holds a secret")]
    Secret(&'static str),
}

/// The visitor of a string given as a newtype (`deserialize_newtype_struct`): JSON gives the string itself, the trapped
/// data's test gives it by the type's name.
struct NewtypeStr;

impl<'de> Visitor<'de> for NewtypeStr {
    type Value = String;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a string")
    }
    fn visit_newtype_struct<D: Deserializer<'de>>(self, d: D) -> Result<String, D::Error> {
        String::deserialize(d)
    }
    fn visit_str<E: de::Error>(self, v: &str) -> Result<String, E> {
        Ok(v.to_owned())
    }
    fn visit_string<E: de::Error>(self, v: String) -> Result<String, E> {
        Ok(v)
    }
}

/// Reads a string newtype named `name`, then makes it with `make`.
pub(crate) fn read_word<'de, D, T>(
    d: D,
    name: &'static str,
    make: fn(String) -> Result<T, Refused>,
) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
{
    let s = d.deserialize_newtype_struct(name, NewtypeStr)?;
    make(s).map_err(de::Error::custom)
}

/// A string newtype of a fixed grammar, refused when it holds a token.
macro_rules! word {
    ($(#[$m:meta])* $name:ident, $check:expr) => {
        $(#[$m])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            /// Takes `s` if it fits the grammar and holds no token.
            pub fn new(s: impl Into<String>) -> Result<Self, Refused> {
                let s = s.into();
                let fits: fn(&str) -> bool = $check;
                if !fits(&s) {
                    return Err(Refused::Shape(stringify!($name)));
                }
                if never::secret_kind(&s).is_some() || never::email_like(&s) {
                    return Err(Refused::Secret(stringify!($name)));
                }
                Ok(Self(s))
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_str(&self.0)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                read_word(d, stringify!($name), |s| Self::new(s))
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

fn len_in(s: &str, min: usize, max: usize) -> bool {
    (min..=max).contains(&s.len())
}

word!(
    /// An opaque id (a team, a member, a device, a project, an invitation, a directive…): lower case letters, digits,
    /// `-`, `_` and `.`, 1 to 96 characters.
    Id,
    |s| len_in(s, 1, 96)
        && s.chars().next().is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_' | '.'))
);

word!(
    /// A ticket's key (`ESC-608`), never its title (A3-B).
    TicketKey,
    |s| match s.split_once('-') {
        Some((p, n)) => {
            len_in(p, 1, 10)
                && p.chars().next().is_some_and(|c| c.is_ascii_uppercase())
                && p.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
                && len_in(n, 1, 9)
                && !n.starts_with('0')
                && n.chars().all(|c| c.is_ascii_digit())
        }
        None => false,
    }
);

word!(
    /// A key of `.escouade.yml` (`agents.engines`, `plugins.dev.escouade.slack.channel`), up to 128 characters.
    SettingKey,
    |s| len_in(s, 1, 128)
        && s.chars().next().is_some_and(|c| c.is_ascii_lowercase())
        && s.split('.').all(|part| {
            !part.is_empty() && part.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
        })
);

word!(
    /// A word of a closed vocabulary that grows outside of this crate: a harness, a model, a tool, a permission, an
    /// agent's mode, a program (`git`), a stream (`team`). Up to 64 characters, no space nor separator.
    Token,
    |s| len_in(s, 1, 64)
        && s.chars().next().is_some_and(|c| c.is_ascii_alphanumeric())
        && s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | ':' | '+' | '@' | '-'))
);

word!(
    /// A plugin's id (`dev.escouade.slack`).
    PluginId,
    |s| len_in(s, 3, 100)
        && s.contains('.')
        && s.split('.').all(|part| {
            !part.is_empty()
                && part.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        })
);

word!(
    /// An espace de noms of the private store (`fr.acme.*`).
    Namespace,
    |s| match s.strip_suffix(".*") {
        Some(base) => len_in(base, 1, 96)
            && base.split('.').all(|part| {
                !part.is_empty()
                    && part.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            }),
        None => false,
    }
);

word!(
    /// A version (`2.3.0`, `1.0.0-beta.2`).
    Version,
    |s| len_in(s, 1, 32)
        && s.chars().next().is_some_and(|c| c.is_ascii_digit())
        && s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+'))
);

word!(
    /// A SHA-256 fingerprint, `sha256:` then 64 lower case hexadecimal digits.
    Digest,
    |s| s
        .strip_prefix("sha256:")
        .is_some_and(|h| h.len() == 64 && h.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)))
);

word!(
    /// An Ed25519 public key, `ed25519:` then its 32 bytes in base64.
    PublicKey,
    |s| s.strip_prefix("ed25519:").is_some_and(|b| b64_len(b) == Some(32))
);

word!(
    /// An Ed25519 signature, `ed25519:` then its 64 bytes in base64.
    Signature,
    |s| s.strip_prefix("ed25519:").is_some_and(|b| b64_len(b) == Some(64))
);

word!(
    /// The six words of a fingerprint (the team's, read out of band), lower case, separated by spaces.
    SixWords,
    |s| {
        let words: Vec<&str> = s.split(' ').collect();
        words.len() == 6
            && words
                .iter()
                .all(|w| len_in(w, 1, 16) && w.chars().all(|c| c.is_ascii_lowercase()))
    }
);

word!(
    /// An IP address (A6: given whole by the server, its option `--audit-ip` truncates or removes it, AU-2).
    Ip,
    |s| s.parse::<std::net::IpAddr>().is_ok()
);

fn b64_len(b: &str) -> Option<usize> {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD
        .decode(b)
        .ok()
        .map(|v| v.len())
}

impl Digest {
    /// The fingerprint of `bytes`.
    pub fn of(bytes: &[u8]) -> Digest {
        Digest(format!("sha256:{}", hex::encode(Sha256::digest(bytes))))
    }
}

/// A free string (`never.rs`): scrubbed, never refused for what it holds, but for its length or emptiness.
macro_rules! free {
    ($(#[$m:meta])* $name:ident, $max:expr) => {
        $(#[$m])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            /// Takes `s`, scrubbed (`never::scrub`); refused when empty or over its length.
            pub fn new(s: impl Into<String>) -> Result<Self, Refused> {
                let s = s.into();
                let n = s.chars().count();
                if n == 0 || n > $max || s.trim().is_empty() {
                    return Err(Refused::Shape(stringify!($name)));
                }
                Ok(Self(never::scrub(&s)))
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_str(&self.0)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                read_word(d, stringify!($name), |s| Self::new(s))
            }
        }
    };
}

free!(
    /// A name: of a member, a device, a project (its short name), a team, a budget (A3-B). Up to 80 characters.
    Name,
    80
);

free!(
    /// A reason an administrator wrote (a kill switch, a setting, a revocation), up to 500 characters.
    Reason,
    500
);

/// A command an agent was allowed to run, or one a station approved, as it goes out (A8-A): its program and the
/// fingerprint of its whole text, which compares to the one `.escouade.yml` approves (`projectfile.rs`, SHA-256 of the
/// text). Never the text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandPrint {
    pub program: Token,
    pub digest: Digest,
}

impl CommandPrint {
    /// The program and the fingerprint of `command`: the first word that does not give a variable its value, without
    /// its folder, its quotes nor its extension (`.exe`, `.cmd`, `.bat`, `.ps1`), in lower case; `unknown` when it is
    /// no word of a program.
    pub fn of(command: &str) -> CommandPrint {
        let program = shell_words(command)
            .into_iter()
            .find(|w| !w.contains('='))
            .map(|w| {
                let base = w
                    .rsplit(['/', '\\'])
                    .next()
                    .unwrap_or(&w)
                    .to_ascii_lowercase();
                [".exe", ".cmd", ".bat", ".ps1"]
                    .iter()
                    .find_map(|ext| base.strip_suffix(ext).map(str::to_owned))
                    .unwrap_or(base)
            })
            .and_then(|p| Token::new(p).ok())
            .unwrap_or_else(|| Token("unknown".to_owned()));
        CommandPrint {
            program,
            digest: Digest::of(command.as_bytes()),
        }
    }
}

/// The words of a command line, a quoted run (`"C:\Program Files\git.exe"`) one word without its quotes.
fn shell_words(command: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quote: Option<char> = None;
    for c in command.chars() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => word.push(c),
            None if c == '"' || c == '\'' => quote = Some(c),
            None if c.is_whitespace() => {
                if !word.is_empty() {
                    words.push(std::mem::take(&mut word));
                }
            }
            None => word.push(c),
        }
    }
    if !word.is_empty() {
        words.push(word);
    }
    words
}

/// An instant, to the millisecond, written in RFC 3339 in UTC (`2026-11-21T10:42:07.120Z`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Time(i64);

impl Time {
    /// The instant `ms` milliseconds after 1970-01-01T00:00:00Z.
    pub fn from_millis(ms: i64) -> Time {
        Time(ms)
    }
    pub fn millis(self) -> i64 {
        self.0
    }
    /// Now, by the system's clock.
    pub fn now() -> Time {
        let d = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        Time(d.as_millis() as i64)
    }
    /// The day (`2026-11-21`).
    pub fn day(self) -> String {
        let (y, m, d) = civil(self.0.div_euclid(86_400_000));
        format!("{y:04}-{m:02}-{d:02}")
    }
    pub fn parse(s: &str) -> Result<Time, Refused> {
        let bad = || Refused::Shape("Time");
        let b = s.as_bytes();
        if b.len() != 24
            || b[4] != b'-'
            || b[7] != b'-'
            || b[10] != b'T'
            || b[13] != b':'
            || b[16] != b':'
            || b[19] != b'.'
            || b[23] != b'Z'
        {
            return Err(bad());
        }
        let num = |a: usize, z: usize| -> Result<i64, Refused> {
            let p = &s[a..z];
            if p.chars().all(|c| c.is_ascii_digit()) {
                p.parse().map_err(|_| bad())
            } else {
                Err(bad())
            }
        };
        let (y, mo, d, h, mi, se, ms) = (
            num(0, 4)?,
            num(5, 7)?,
            num(8, 10)?,
            num(11, 13)?,
            num(14, 16)?,
            num(17, 19)?,
            num(20, 23)?,
        );
        if !(1..=12).contains(&mo) || d < 1 || d > month_days(y, mo) || h > 23 || mi > 59 || se > 59
        {
            return Err(bad());
        }
        let days = days_from_civil(y, mo, d);
        Ok(Time(((days * 24 + h) * 60 + mi) * 60_000 + se * 1000 + ms))
    }
}

fn month_days(y: i64, m: i64) -> i64 {
    match m {
        2 if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Days since 1970-01-01 of a date (H. Hinnant's algorithm).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The date of a day since 1970-01-01.
fn civil(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let days = self.0.div_euclid(86_400_000);
        let rest = self.0.rem_euclid(86_400_000);
        let (y, m, d) = civil(days);
        let (h, mi, s, ms) = (
            rest / 3_600_000,
            rest / 60_000 % 60,
            rest / 1000 % 60,
            rest % 1000,
        );
        write!(f, "{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}.{ms:03}Z")
    }
}

impl Serialize for Time {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Time {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        read_word(d, "Time", |s| Time::parse(&s))
    }
}

/// An event's id: a ULID (48 bits of milliseconds, 80 random), 26 characters of Crockford's base 32.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ulid(String);

const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

impl Ulid {
    /// A new id at `at`.
    pub fn new(at: Time) -> Ulid {
        let mut random = [0u8; 10];
        getrandom::fill(&mut random).expect("the system's randomness");
        Ulid::of(at, random)
    }
    /// The id of `at` and these 80 bits.
    pub fn of(at: Time, random: [u8; 10]) -> Ulid {
        let mut n: u128 = (at.millis().max(0) as u128 & ((1 << 48) - 1)) << 80;
        for (i, b) in random.iter().enumerate() {
            n |= (*b as u128) << (8 * (9 - i));
        }
        let s = (0..26)
            .rev()
            .map(|i| CROCKFORD[((n >> (5 * i)) & 31) as usize] as char)
            .collect();
        Ulid(s)
    }
    pub fn parse(s: String) -> Result<Ulid, Refused> {
        if s.len() == 26 && s.as_bytes()[0] <= b'7' && s.bytes().all(|b| CROCKFORD.contains(&b)) {
            Ok(Ulid(s))
        } else {
            Err(Refused::Shape("Ulid"))
        }
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Serialize for Ulid {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Ulid {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        read_word(d, "Ulid", Ulid::parse)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_instant_reads_back_as_it_was_written() {
        let t = Time::parse("2026-11-21T10:42:07.120Z").unwrap();
        assert_eq!(t.to_string(), "2026-11-21T10:42:07.120Z");
        assert_eq!(t.day(), "2026-11-21");
        assert_eq!(Time::from_millis(0).to_string(), "1970-01-01T00:00:00.000Z");
        assert_eq!(
            Time::parse("2024-02-29T23:59:59.999Z").unwrap().to_string(),
            "2024-02-29T23:59:59.999Z"
        );
        for bad in [
            "2026-02-30T00:00:00.000Z",
            "2026-11-21T10:42:07Z",
            "2026-11-21 10:42:07.120Z",
            "x",
        ] {
            assert!(Time::parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn an_id_is_a_ulid_of_its_instant() {
        let t = Time::parse("2026-11-21T10:42:07.120Z").unwrap();
        let a = Ulid::of(t, [0; 10]);
        assert_eq!(a.as_str().len(), 26);
        assert!(a.as_str().ends_with("0000000000000000"));
        assert!(Ulid::new(t) > a);
        assert!(Ulid::parse(a.as_str().to_owned()).is_ok());
        assert!(Ulid::parse("01JD7-not-a-ulid".into()).is_err());
    }

    #[test]
    fn the_words_refuse_what_does_not_fit_and_a_token() {
        assert!(Id::new("d-77e0").is_ok());
        assert!(Id::new("Rachat de la société").is_err());
        assert!(Id::new("sk-ant-api03-abcdefghijklmnopqrstuvwx").is_err());
        assert!(TicketKey::new("ESC-608").is_ok());
        assert!(TicketKey::new("ESC-0").is_err());
        assert!(SettingKey::new("plugins.dev.escouade.slack.channel").is_ok());
        assert!(SettingKey::new("src/lib.rs").is_err());
        assert!(Digest::new(Digest::of(b"x").as_str()).is_ok());
        assert!(Digest::new("sha256:ABC").is_err());
        assert!(Namespace::new("fr.acme.*").is_ok());
        assert!(SixWords::new("abandon ability able about above absent").is_ok());
        assert!(Ip::new("2001:db8::1").is_ok());
        assert!(Name::new("").is_err());
        assert!(Name::new("x".repeat(81)).is_err());
    }

    #[test]
    fn a_command_goes_out_as_its_program_and_its_fingerprint() {
        let text = "GITHUB_TOKEN=ghp_abcdefghijklmnopqrstuvwxyz0123 \"C:\\Program Files\\Git\\bin\\git.exe\" push";
        let p = CommandPrint::of(text);
        assert_eq!(p.program.as_str(), "git");
        let p = CommandPrint::of("NODE_ENV=production /usr/bin/npm publish --tag beta");
        assert_eq!(p.program.as_str(), "npm");
        assert_eq!(
            p.digest,
            Digest::of(b"NODE_ENV=production /usr/bin/npm publish --tag beta")
        );
        assert_eq!(CommandPrint::of("").program.as_str(), "unknown");
    }
}
