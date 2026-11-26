//! The bytes both ways between an agent and where it goes, counted; the head of an HTTP message read; a flow that gives
//! back first what was read ahead of it.

use std::io;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};

/// The longest head of a request or an answer.
pub const HEAD_MAX: usize = 16 * 1024;

/// The time to send a whole head.
pub const HEAD_WITHIN: Duration = Duration::from_secs(10);

/// The head of an HTTP message (up to its empty line) and what came after it in the same reads.
pub async fn read_head<S: AsyncRead + Unpin>(s: &mut S) -> io::Result<(Vec<u8>, Vec<u8>)> {
    let read = async {
        let mut buf = Vec::with_capacity(1024);
        let mut chunk = [0u8; 2048];
        loop {
            let n = s.read(&mut chunk).await?;
            if n == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "closed before its head",
                ));
            }
            let from = buf.len().saturating_sub(3);
            buf.extend_from_slice(&chunk[..n]);
            if let Some(i) = buf[from..].windows(4).position(|w| w == b"\r\n\r\n") {
                let end = from + i + 4;
                let rest = buf.split_off(end);
                return Ok((buf, rest));
            }
            if buf.len() > HEAD_MAX {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "head too long"));
            }
        }
    };
    tokio::time::timeout(HEAD_WITHIN, read)
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "head too slow"))?
}

/// A flow that reads `ahead` first.
pub struct Prefixed<S> {
    ahead: Vec<u8>,
    at: usize,
    inner: S,
}

impl<S> Prefixed<S> {
    pub fn new(ahead: Vec<u8>, inner: S) -> Self {
        Self {
            ahead,
            at: 0,
            inner,
        }
    }
}

impl<S: AsyncRead + Unpin> AsyncRead for Prefixed<S> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if self.at < self.ahead.len() {
            let n = (self.ahead.len() - self.at).min(buf.remaining());
            let at = self.at;
            buf.put_slice(&self.ahead[at..at + n]);
            self.at += n;
            return Poll::Ready(Ok(()));
        }
        Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for Prefixed<S> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

/// Copies `from` to `to` until `from` ends, counting in `n`; then closes `to` for writing.
async fn copy<R, W>(mut from: R, mut to: W, n: Arc<AtomicU64>)
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut buf = vec![0u8; 16 * 1024];
    loop {
        match from.read(&mut buf).await {
            Ok(0) | Err(_) => break,
            Ok(k) => {
                if to.write_all(&buf[..k]).await.is_err() {
                    break;
                }
                n.fetch_add(k as u64, Ordering::Relaxed);
            }
        }
    }
    let _ = to.shutdown().await;
}

/// The bytes both ways between `agent` and `remote` until `remote` ends: the agent closing its side only ends what it
/// sends (the answer still comes), but the remote's end — a host that closed, a connector cut — ends it all at once,
/// with no resuming (§ 4.8). Gives the bytes from the agent and to it.
pub async fn pipe<A, B>(agent: A, remote: B) -> (u64, u64)
where
    A: AsyncRead + AsyncWrite + Unpin,
    B: AsyncRead + AsyncWrite + Unpin,
{
    let (ar, aw) = tokio::io::split(agent);
    let (rr, rw) = tokio::io::split(remote);
    let up = Arc::new(AtomicU64::new(0));
    let down = Arc::new(AtomicU64::new(0));
    let sending = copy(ar, rw, up.clone());
    let receiving = copy(rr, aw, down.clone());
    tokio::pin!(sending, receiving);
    let mut sent = false;
    loop {
        tokio::select! {
            _ = &mut sending, if !sent => sent = true,
            _ = &mut receiving => break,
        }
    }
    (up.load(Ordering::Relaxed), down.load(Ordering::Relaxed))
}
