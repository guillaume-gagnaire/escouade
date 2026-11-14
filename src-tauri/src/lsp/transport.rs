//! The base protocol of LSP on a server's stdin and stdout (ESC-446): `Content-Length: n`, a blank line, n bytes of
//! JSON. Bounded, as the stream of a harness is: a header line, the headers and a message have a size past which the
//! stream is refused, and a length announced too big is refused before anything is read (a server does not make the
//! core allocate what it says).

use std::fmt;

use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt};

/// The biggest message taken, either way: rust-analyzer's semantic tokens of a long file are a few MB.
pub const MAX_MESSAGE: usize = 32 << 20;
/// The longest header line.
const MAX_HEADER_LINE: usize = 1024;
/// The most header lines before the blank one (LSP knows two: `Content-Length`, `Content-Type`).
const MAX_HEADERS: usize = 16;

/// Why a stream can no longer be read: past it, nothing of it is trusted.
#[derive(Debug, PartialEq, Eq)]
pub enum FrameError {
    /// The announced length is over `MAX_MESSAGE`.
    TooBig(usize),
    /// A header line too long, too many, or no `Content-Length` readable.
    BadHeader(String),
    /// The body is not UTF-8.
    NotText,
    /// The stream ended in a message.
    Cut,
    Io(String),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::TooBig(n) => write!(
                f,
                "message of {n} bytes announced, more than the {MAX_MESSAGE} allowed"
            ),
            FrameError::BadHeader(why) => write!(f, "bad header: {why}"),
            FrameError::NotText => write!(f, "a message that is not UTF-8"),
            FrameError::Cut => write!(f, "the stream ended inside a message"),
            FrameError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for FrameError {}

/// One line of the header, read up to `MAX_HEADER_LINE` bytes; None at the end of the stream.
async fn header_line<R: AsyncBufRead + Unpin>(r: &mut R) -> Result<Option<String>, FrameError> {
    let mut line = Vec::new();
    let n = (&mut *r)
        .take(MAX_HEADER_LINE as u64 + 1)
        .read_until(b'\n', &mut line)
        .await
        .map_err(|e| FrameError::Io(e.to_string()))?;
    if n == 0 {
        return Ok(None);
    }
    if line.last() != Some(&b'\n') {
        return Err(if n > MAX_HEADER_LINE {
            FrameError::BadHeader(format!("a line longer than {MAX_HEADER_LINE} bytes"))
        } else {
            FrameError::Cut
        });
    }
    let text = String::from_utf8(line).map_err(|_| FrameError::BadHeader("not text".into()))?;
    Ok(Some(text.trim_end_matches(['\r', '\n']).to_string()))
}

/// Reads one message; None when the stream ends between two.
pub async fn read_frame<R: AsyncBufRead + Unpin>(r: &mut R) -> Result<Option<String>, FrameError> {
    let mut len: Option<usize> = None;
    let mut headers = 0;
    loop {
        let Some(line) = header_line(r).await? else {
            return if headers == 0 {
                Ok(None)
            } else {
                Err(FrameError::Cut)
            };
        };
        if line.is_empty() {
            if headers == 0 {
                // Blank lines between messages: some servers print one more.
                continue;
            }
            break;
        }
        headers += 1;
        if headers > MAX_HEADERS {
            return Err(FrameError::BadHeader(format!(
                "more than {MAX_HEADERS} header lines"
            )));
        }
        let Some((k, v)) = line.split_once(':') else {
            return Err(FrameError::BadHeader(format!("“{line}”")));
        };
        if k.trim().eq_ignore_ascii_case("content-length") {
            let n: usize = v
                .trim()
                .parse()
                .map_err(|_| FrameError::BadHeader(format!("Content-Length “{}”", v.trim())))?;
            if n > MAX_MESSAGE {
                return Err(FrameError::TooBig(n));
            }
            len = Some(n);
        }
    }
    let n = len.ok_or_else(|| FrameError::BadHeader("no Content-Length".into()))?;
    let mut body = vec![0u8; n];
    r.read_exact(&mut body).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::UnexpectedEof {
            FrameError::Cut
        } else {
            FrameError::Io(e.to_string())
        }
    })?;
    String::from_utf8(body)
        .map(Some)
        .map_err(|_| FrameError::NotText)
}

/// One message framed for the server's stdin; refused over `MAX_MESSAGE`.
pub fn frame(msg: &str) -> Result<Vec<u8>, FrameError> {
    if msg.len() > MAX_MESSAGE {
        return Err(FrameError::TooBig(msg.len()));
    }
    let mut out = format!("Content-Length: {}\r\n\r\n", msg.len()).into_bytes();
    out.extend_from_slice(msg.as_bytes());
    Ok(out)
}
