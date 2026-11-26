//! The versions (§ 4.2, § 4.7, R10): the connector says its own in the first message of the handshake ([`Hello`]),
//! the server answers its own and its verdict in the second ([`Welcome`]). The server accepts its own minor and the two
//! before (the company updates at its pace, not the user's): one behind is current, two behind is « to update »,
//! further is too old, and so is another major; a connector newer than the server is refused too, the server being the
//! one to update. A refused connector is told the versions the server accepts, so that it says what to do.
//!
//! Both messages begin with the version, forever: a server of any version reads the version of a connector of any
//! other, and answers a verdict it understands. Bytes after the known fields are left for later versions and ignored.

use std::fmt;

/// A version `major.minor.patch`, as `escouade-server` and `escouade-connector` carry theirs (their `Cargo.toml`).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Version {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl Version {
    pub const fn new(major: u16, minor: u16, patch: u16) -> Version {
        Version {
            major,
            minor,
            patch,
        }
    }

    /// `2.3.1`, `2.3.1-beta.2` (what follows the patch is dropped), or `None`.
    pub fn parse(text: &str) -> Option<Version> {
        let core = text.trim().split(['-', '+']).next()?;
        let mut parts = core.split('.');
        let mut next = || parts.next()?.parse::<u16>().ok();
        let v = Version::new(next()?, next()?, next()?);
        parts.next().is_none().then_some(v)
    }

    /// The oldest version a server of this version accepts: its major, two minors before (the patch does not count).
    pub fn oldest_accepted(self) -> Version {
        Version::new(self.major, self.minor.saturating_sub(2), 0)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// What the server says of the connector's version.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Verdict {
    /// Its minor, or the one before.
    Current,
    /// Two minors before: accepted, « to update » in the interface and the connector's journal.
    Outdated,
    /// Further before, or an older major: refused.
    TooOld,
    /// After the server's (a newer minor or major): refused, the server is the one to update.
    TooNew,
}

impl Verdict {
    pub fn accepted(self) -> bool {
        matches!(self, Verdict::Current | Verdict::Outdated)
    }

    fn code(self) -> u8 {
        match self {
            Verdict::Current => 0,
            Verdict::Outdated => 1,
            Verdict::TooOld => 2,
            Verdict::TooNew => 3,
        }
    }

    fn from_code(code: u8) -> Option<Verdict> {
        Some(match code {
            0 => Verdict::Current,
            1 => Verdict::Outdated,
            2 => Verdict::TooOld,
            3 => Verdict::TooNew,
            _ => return None,
        })
    }
}

/// The verdict of a server of version `server` on a connector of version `connector`.
pub fn judge(server: Version, connector: Version) -> Verdict {
    if connector.major < server.major {
        return Verdict::TooOld;
    }
    if connector.major > server.major || connector.minor > server.minor {
        return Verdict::TooNew;
    }
    match server.minor - connector.minor {
        0 | 1 => Verdict::Current,
        2 => Verdict::Outdated,
        _ => Verdict::TooOld,
    }
}

/// The payload of the handshake's first message, from the connector.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Hello {
    pub version: Version,
}

/// The payload of the handshake's second message, from the server.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Welcome {
    /// The server's version.
    pub version: Version,
    pub verdict: Verdict,
    /// The oldest the server accepts (its patch is 0).
    pub oldest: Version,
}

impl Welcome {
    /// The server's answer to `hello`.
    pub fn for_hello(server: Version, hello: &Hello) -> Welcome {
        Welcome {
            version: server,
            verdict: judge(server, hello.version),
            oldest: server.oldest_accepted(),
        }
    }

    /// What a connector of version `connector` is to do, in a line for its journal; `None` when it is current.
    pub fn advice(&self, connector: Version) -> Option<String> {
        let (server, oldest) = (self.version, self.oldest);
        let range = format!(
            "{}.{} to {}.{}",
            oldest.major, oldest.minor, server.major, server.minor
        );
        Some(match self.verdict {
            Verdict::Current => return None,
            Verdict::Outdated => format!(
                "escouade-connector {connector} is accepted by the server ({server}) but is to update: it accepts {range} (escouade-connector update --to {server})"
            ),
            Verdict::TooOld => format!(
                "escouade-connector {connector} is too old for the server ({server}), which accepts {range}: update it (escouade-connector update --to {server})"
            ),
            Verdict::TooNew => format!(
                "escouade-connector {connector} is newer than the server ({server}), which accepts {range}: update escouade-server, or install escouade-connector {server}"
            ),
        })
    }
}

/// Why a payload of the handshake is not read.
#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)]
#[error("the handshake's payload is not understood")]
pub struct BadPayload;

fn put_version(out: &mut Vec<u8>, v: Version) {
    out.extend_from_slice(&v.major.to_be_bytes());
    out.extend_from_slice(&v.minor.to_be_bytes());
    out.extend_from_slice(&v.patch.to_be_bytes());
}

fn get_version(b: &[u8]) -> Option<Version> {
    let n = |i: usize| Some(u16::from_be_bytes([*b.get(i)?, *b.get(i + 1)?]));
    Some(Version::new(n(0)?, n(2)?, n(4)?))
}

impl Hello {
    /// `major:u16 minor:u16 patch:u16`, big-endian; then nothing (what a later version adds is ignored).
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(6);
        put_version(&mut out, self.version);
        out
    }

    pub fn decode(b: &[u8]) -> Result<Hello, BadPayload> {
        Ok(Hello {
            version: get_version(b).ok_or(BadPayload)?,
        })
    }
}

impl Welcome {
    /// The server's version (`u16` ×3), the verdict (`u8`), the oldest accepted (`major:u16 minor:u16`); then
    /// nothing (what a later version adds is ignored).
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(11);
        put_version(&mut out, self.version);
        out.push(self.verdict.code());
        out.extend_from_slice(&self.oldest.major.to_be_bytes());
        out.extend_from_slice(&self.oldest.minor.to_be_bytes());
        out
    }

    pub fn decode(b: &[u8]) -> Result<Welcome, BadPayload> {
        let version = get_version(b).ok_or(BadPayload)?;
        let verdict = Verdict::from_code(*b.get(6).ok_or(BadPayload)?).ok_or(BadPayload)?;
        let rest = b.get(7..11).ok_or(BadPayload)?;
        let oldest = Version::new(
            u16::from_be_bytes([rest[0], rest[1]]),
            u16::from_be_bytes([rest[2], rest[3]]),
            0,
        );
        Ok(Welcome {
            version,
            verdict,
            oldest,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_server_accepts_its_minor_and_the_two_before() {
        let server = Version::new(2, 5, 3);
        let v = |minor, patch| judge(server, Version::new(2, minor, patch));
        assert_eq!(v(5, 9), Verdict::Current, "a newer patch");
        assert_eq!(v(5, 0), Verdict::Current);
        assert_eq!(v(4, 7), Verdict::Current);
        assert_eq!(v(3, 0), Verdict::Outdated, "two minors before: to update");
        assert_eq!(v(2, 9), Verdict::TooOld);
        assert_eq!(v(0, 0), Verdict::TooOld);
        assert_eq!(v(6, 0), Verdict::TooNew);
        assert_eq!(judge(server, Version::new(1, 9, 0)), Verdict::TooOld);
        assert_eq!(judge(server, Version::new(3, 0, 0)), Verdict::TooNew);
        // At the start of a major, nothing before it.
        let fresh = Version::new(3, 0, 0);
        assert_eq!(judge(fresh, Version::new(3, 0, 1)), Verdict::Current);
        assert_eq!(judge(fresh, Version::new(2, 9, 0)), Verdict::TooOld);
        assert_eq!(fresh.oldest_accepted(), Version::new(3, 0, 0));
        assert!(Verdict::Outdated.accepted() && !Verdict::TooOld.accepted());
    }

    #[test]
    fn the_advice_says_what_to_do() {
        let server = Version::new(2, 5, 3);
        let told = |c: Version| Welcome::for_hello(server, &Hello { version: c }).advice(c);
        assert_eq!(told(Version::new(2, 5, 0)), None);
        let old = told(Version::new(2, 1, 4)).unwrap();
        assert!(
            old.contains("too old")
                && old.contains("2.3 to 2.5")
                && old.contains("update --to 2.5.3"),
            "{old}"
        );
        let new = told(Version::new(2, 6, 0)).unwrap();
        assert!(new.contains("update escouade-server"), "{new}");
        assert!(told(Version::new(2, 3, 0)).unwrap().contains("to update"));
    }

    #[test]
    fn versions_are_parsed() {
        assert_eq!(Version::parse("2.3.1"), Some(Version::new(2, 3, 1)));
        assert_eq!(Version::parse("2.3.1-beta.2"), Some(Version::new(2, 3, 1)));
        assert_eq!(Version::parse("2.3"), None);
        assert_eq!(Version::parse("2.3.1.4"), None);
        assert_eq!(Version::parse("2.x.1"), None);
        assert_eq!(Version::new(2, 3, 1).to_string(), "2.3.1");
    }

    #[test]
    fn the_payloads_go_and_come_back_and_ignore_what_follows() {
        let hello = Hello {
            version: Version::new(2, 3, 1),
        };
        assert_eq!(hello.encode(), [0, 2, 0, 3, 0, 1]);
        assert_eq!(Hello::decode(&hello.encode()), Ok(hello));
        let mut later = hello.encode();
        later.extend_from_slice(b"a later version's field");
        assert_eq!(Hello::decode(&later), Ok(hello));
        assert_eq!(Hello::decode(&[0, 2, 0, 3, 0]), Err(BadPayload));

        let welcome = Welcome::for_hello(Version::new(2, 5, 0), &hello);
        assert_eq!(welcome.encode(), [0, 2, 0, 5, 0, 0, 1, 0, 2, 0, 3]);
        assert_eq!(Welcome::decode(&welcome.encode()), Ok(welcome));
        let mut bad = welcome.encode();
        bad[6] = 9;
        assert_eq!(Welcome::decode(&bad), Err(BadPayload));
        assert_eq!(Welcome::decode(&welcome.encode()[..10]), Err(BadPayload));
    }
}
