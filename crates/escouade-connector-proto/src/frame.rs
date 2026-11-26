//! The frames, one per Noise message once the handshake is done.
//!
//! A frame is its kind (`u8`), then its fields; integers big-endian; a `str8` is a length (`u8`) then that many bytes of
//! UTF-8, a `str16` the same with a length `u16`:
//!
//! | kind | frame | fields | sent by |
//! | --- | --- | --- | --- |
//! | `0x01` | `open` | `id:u32 port:u16 host:str8 agent:str8 project:str8` | the server |
//! | `0x02` | `opened` | `id:u32` | the connector |
//! | `0x03` | `refused` | `id:u32 code:u8 reason:str16` | the connector |
//! | `0x04` | `data` | `id:u32`, then the bytes to the end (none: the end of that direction, as TCP's FIN) | both |
//! | `0x05` | `close` | `id:u32` (the stream is gone, both directions) | both |
//! | `0x06` | `window` | `id:u32 increment:u32` | both |
//! | `0x07` | `ping` | `nonce:u64` | both |
//! | `0x08` | `pong` | `nonce:u64` | both |
//! | `0x09` | `ceiling` | `count:u16`, then `count` rules, each a `str16` | the connector |
//!
//! The decoding is strict (no byte left over, every string UTF-8 within its bound), so that a frame has one encoding
//! alone; a kind it does not know is [`DecodeError::UnknownKind`], which a channel ignores (a peer of a later minor).

use std::fmt;

/// A stream, numbered by the server from 1.
pub type StreamId = u32;

/// The most bytes of a host: a DNS name.
pub const MAX_HOST: usize = 253;
/// The most bytes of a refusal's reason.
pub const MAX_REASON: usize = 1024;
/// The most bytes of one rule of the ceiling.
pub const MAX_RULE: usize = 1024;
/// The bytes of a `data` frame before its payload.
pub const DATA_HEADER: usize = 5;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Frame {
    Open(Open),
    Opened {
        id: StreamId,
    },
    Refused {
        id: StreamId,
        refusal: Refusal,
    },
    /// Empty: the sender writes no more on this stream (it still reads).
    Data {
        id: StreamId,
        bytes: Vec<u8>,
    },
    Close {
        id: StreamId,
    },
    Window {
        id: StreamId,
        increment: u32,
    },
    Ping {
        nonce: u64,
    },
    Pong {
        nonce: u64,
    },
    Ceiling(Ceiling),
}

/// « Open `host:port` » for an agent of a project (announced by the server: the connector journals them as said).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Open {
    pub id: StreamId,
    pub host: String,
    pub port: u16,
    pub agent: String,
    pub project: String,
}

/// Why the connector (or the channel) refuses a stream.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RefusalCode {
    /// Not in the ceiling, a literal address, a forbidden address after the resolution (§ 4.4).
    NotAllowed,
    /// The name is not resolved.
    Unresolved,
    /// Resolved, but nothing answers (refused, timed out).
    Unreachable,
    /// A bound: too many streams open, too many opens in a second.
    Limits,
    /// The connector is closing (stopped, unpaired).
    Closing,
    /// A code of a later minor, or none of the above.
    Other(u8),
}

impl RefusalCode {
    pub fn code(self) -> u8 {
        match self {
            RefusalCode::NotAllowed => 1,
            RefusalCode::Unresolved => 2,
            RefusalCode::Unreachable => 3,
            RefusalCode::Limits => 4,
            RefusalCode::Closing => 5,
            RefusalCode::Other(c) => c,
        }
    }

    /// Canonical: `Other` never holds a code that has a name.
    pub fn from_code(code: u8) -> RefusalCode {
        match code {
            1 => RefusalCode::NotAllowed,
            2 => RefusalCode::Unresolved,
            3 => RefusalCode::Unreachable,
            4 => RefusalCode::Limits,
            5 => RefusalCode::Closing,
            c => RefusalCode::Other(c),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Refusal {
    pub code: RefusalCode,
    /// For the agent and the journals: « gitlab.acme.internal:22 is not allowed by the connector of Acme ».
    pub reason: String,
}

impl Refusal {
    /// The reason cut to [`MAX_REASON`] bytes, at a character's boundary.
    pub fn new(code: RefusalCode, reason: impl Into<String>) -> Refusal {
        let mut reason = reason.into();
        if reason.len() > MAX_REASON {
            let mut end = MAX_REASON;
            while !reason.is_char_boundary(end) {
                end -= 1;
            }
            reason.truncate(end);
        }
        Refusal { code, reason }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.reason)
    }
}

/// The connector's ceiling (R4), sent at each connection and again when it reads `connector.toml` again: its rules as
/// it writes them (`gitlab.acme.internal:22`, `*.npm.acme.internal:443`, `10.20.0.0/16`), for the interface to show;
/// the server cannot widen it.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Ceiling {
    pub rules: Vec<String>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)]
pub enum DecodeError {
    #[error("an empty frame")]
    Empty,
    #[error("a frame of an unknown kind ({0:#04x})")]
    UnknownKind(u8),
    #[error("a frame cut short")]
    Short,
    #[error("bytes after the frame")]
    Trailing,
    #[error("a string that is not UTF-8 or is too long")]
    BadString,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)]
pub enum EncodeError {
    #[error("a string too long for its field")]
    TooLong,
    #[error("too many rules in the ceiling")]
    TooManyRules,
}

struct Writer(Vec<u8>);

impl Writer {
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn u16(&mut self, v: u16) {
        self.0.extend_from_slice(&v.to_be_bytes());
    }
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_be_bytes());
    }
    fn u64(&mut self, v: u64) {
        self.0.extend_from_slice(&v.to_be_bytes());
    }
    fn str8(&mut self, s: &str, max: usize) -> Result<(), EncodeError> {
        if s.len() > max.min(255) {
            return Err(EncodeError::TooLong);
        }
        self.u8(s.len() as u8);
        self.0.extend_from_slice(s.as_bytes());
        Ok(())
    }
    fn str16(&mut self, s: &str, max: usize) -> Result<(), EncodeError> {
        if s.len() > max {
            return Err(EncodeError::TooLong);
        }
        self.u16(s.len() as u16);
        self.0.extend_from_slice(s.as_bytes());
        Ok(())
    }
}

struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], DecodeError> {
        if self.0.len() < n {
            return Err(DecodeError::Short);
        }
        let (head, rest) = self.0.split_at(n);
        self.0 = rest;
        Ok(head)
    }
    fn u8(&mut self) -> Result<u8, DecodeError> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, DecodeError> {
        let b = self.take(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }
    fn u32(&mut self) -> Result<u32, DecodeError> {
        let b = self.take(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn u64(&mut self) -> Result<u64, DecodeError> {
        let mut a = [0u8; 8];
        a.copy_from_slice(self.take(8)?);
        Ok(u64::from_be_bytes(a))
    }
    fn string(&mut self, len: usize, max: usize) -> Result<String, DecodeError> {
        if len > max {
            return Err(DecodeError::BadString);
        }
        let b = self.take(len)?;
        String::from_utf8(b.to_vec()).map_err(|_| DecodeError::BadString)
    }
    fn str8(&mut self, max: usize) -> Result<String, DecodeError> {
        let len = self.u8()? as usize;
        self.string(len, max)
    }
    fn str16(&mut self, max: usize) -> Result<String, DecodeError> {
        let len = self.u16()? as usize;
        self.string(len, max)
    }
    fn end(self) -> Result<(), DecodeError> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err(DecodeError::Trailing)
        }
    }
}

impl Frame {
    pub fn kind(&self) -> u8 {
        match self {
            Frame::Open(_) => 0x01,
            Frame::Opened { .. } => 0x02,
            Frame::Refused { .. } => 0x03,
            Frame::Data { .. } => 0x04,
            Frame::Close { .. } => 0x05,
            Frame::Window { .. } => 0x06,
            Frame::Ping { .. } => 0x07,
            Frame::Pong { .. } => 0x08,
            Frame::Ceiling(_) => 0x09,
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut w = Writer(Vec::with_capacity(16));
        w.u8(self.kind());
        match self {
            Frame::Open(o) => {
                w.u32(o.id);
                w.u16(o.port);
                w.str8(&o.host, MAX_HOST)?;
                w.str8(&o.agent, 255)?;
                w.str8(&o.project, 255)?;
            }
            Frame::Opened { id } | Frame::Close { id } => w.u32(*id),
            Frame::Refused { id, refusal } => {
                w.u32(*id);
                w.u8(refusal.code.code());
                w.str16(&refusal.reason, MAX_REASON)?;
            }
            Frame::Data { id, bytes } => {
                w.u32(*id);
                w.0.extend_from_slice(bytes);
            }
            Frame::Window { id, increment } => {
                w.u32(*id);
                w.u32(*increment);
            }
            Frame::Ping { nonce } | Frame::Pong { nonce } => w.u64(*nonce),
            Frame::Ceiling(c) => {
                let count = u16::try_from(c.rules.len()).map_err(|_| EncodeError::TooManyRules)?;
                w.u16(count);
                for rule in &c.rules {
                    w.str16(rule, MAX_RULE)?;
                }
            }
        }
        Ok(w.0)
    }

    pub fn decode(bytes: &[u8]) -> Result<Frame, DecodeError> {
        let (&kind, body) = bytes.split_first().ok_or(DecodeError::Empty)?;
        let mut r = Reader(body);
        let frame = match kind {
            0x01 => Frame::Open(Open {
                id: r.u32()?,
                port: r.u16()?,
                host: r.str8(MAX_HOST)?,
                agent: r.str8(255)?,
                project: r.str8(255)?,
            }),
            0x02 => Frame::Opened { id: r.u32()? },
            0x03 => Frame::Refused {
                id: r.u32()?,
                refusal: Refusal {
                    code: RefusalCode::from_code(r.u8()?),
                    reason: r.str16(MAX_REASON)?,
                },
            },
            0x04 => {
                let id = r.u32()?;
                let bytes = std::mem::take(&mut r.0).to_vec();
                Frame::Data { id, bytes }
            }
            0x05 => Frame::Close { id: r.u32()? },
            0x06 => Frame::Window {
                id: r.u32()?,
                increment: r.u32()?,
            },
            0x07 => Frame::Ping { nonce: r.u64()? },
            0x08 => Frame::Pong { nonce: r.u64()? },
            0x09 => {
                let count = r.u16()? as usize;
                // Each rule takes two bytes at least: a count beyond what is left is a lie, not an allocation.
                if count > r.0.len() / 2 {
                    return Err(DecodeError::Short);
                }
                let mut rules = Vec::with_capacity(count);
                for _ in 0..count {
                    rules.push(r.str16(MAX_RULE)?);
                }
                Frame::Ceiling(Ceiling { rules })
            }
            k => return Err(DecodeError::UnknownKind(k)),
        };
        r.end()?;
        Ok(frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round(frame: Frame) {
        let bytes = frame.encode().unwrap();
        assert_eq!(Frame::decode(&bytes), Ok(frame), "{bytes:02x?}");
    }

    #[test]
    fn every_frame_goes_and_comes_back() {
        round(Frame::Open(Open {
            id: 7,
            host: "gitlab.acme.internal".into(),
            port: 22,
            agent: "Démo · agent 3".into(),
            project: "Démo".into(),
        }));
        round(Frame::Opened { id: u32::MAX });
        round(Frame::Refused {
            id: 1,
            refusal: Refusal::new(RefusalCode::NotAllowed, "pas dans le plafond"),
        });
        round(Frame::Refused {
            id: 1,
            refusal: Refusal::new(RefusalCode::Other(200), ""),
        });
        round(Frame::Data {
            id: 3,
            bytes: vec![0; 16 << 10],
        });
        round(Frame::Data {
            id: 3,
            bytes: vec![],
        });
        round(Frame::Close { id: 9 });
        round(Frame::Window {
            id: 2,
            increment: 65536,
        });
        round(Frame::Ping { nonce: 1 });
        round(Frame::Pong { nonce: u64::MAX });
        round(Frame::Ceiling(Ceiling {
            rules: vec![
                "gitlab.acme.internal:22".into(),
                "*.npm.acme.internal:443".into(),
            ],
        }));
        round(Frame::Ceiling(Ceiling::default()));
    }

    #[test]
    fn what_is_not_a_frame_is_refused() {
        assert_eq!(Frame::decode(&[]), Err(DecodeError::Empty));
        assert_eq!(
            Frame::decode(&[0x42, 1, 2]),
            Err(DecodeError::UnknownKind(0x42))
        );
        assert_eq!(Frame::decode(&[0x02, 0, 0, 1]), Err(DecodeError::Short));
        assert_eq!(
            Frame::decode(&[0x02, 0, 0, 0, 1, 0]),
            Err(DecodeError::Trailing)
        );
        // A host of 254 bytes, a string that is not UTF-8.
        let mut long = vec![0x01, 0, 0, 0, 1, 0, 22, 254];
        long.extend(std::iter::repeat_n(b'a', 254));
        long.extend([0, 0]);
        assert_eq!(Frame::decode(&long), Err(DecodeError::BadString));
        assert_eq!(
            Frame::decode(&[0x01, 0, 0, 0, 1, 0, 22, 1, 0xff, 0, 0]),
            Err(DecodeError::BadString)
        );
        // A ceiling that says 65535 rules in four bytes.
        assert_eq!(
            Frame::decode(&[0x09, 0xff, 0xff, 0, 0]),
            Err(DecodeError::Short)
        );
    }

    #[test]
    fn what_cannot_be_encoded_is_said() {
        let open = |host: String| {
            Frame::Open(Open {
                id: 1,
                host,
                port: 1,
                agent: String::new(),
                project: String::new(),
            })
        };
        assert_eq!(open("a".repeat(254)).encode(), Err(EncodeError::TooLong));
        assert!(open("a".repeat(253)).encode().is_ok());
        let many = Frame::Ceiling(Ceiling {
            rules: vec![String::new(); 70_000],
        });
        assert_eq!(many.encode(), Err(EncodeError::TooManyRules));
        let reason = Refusal::new(RefusalCode::Limits, "é".repeat(600));
        assert!(reason.reason.len() <= MAX_REASON && reason.reason.chars().all(|c| c == 'é'));
    }
}
