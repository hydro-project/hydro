use std::io::{BufRead, Read};
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use russh::CryptoVec;
use tokio::io::{AsyncBufRead, AsyncRead, ReadBuf};
use tokio::sync::mpsc;

/// Read byte data from an SSH channel stream.
///
/// Implements [`AsyncRead`], [`AsyncBufRead`], [`Read`], and [`BufRead`].
pub struct ReadStream {
    recv: mpsc::UnboundedReceiver<CryptoVec>,
    buffer: Option<(CryptoVec, usize)>,
}
impl ReadStream {
    pub(crate) fn from_recv(recv: mpsc::UnboundedReceiver<CryptoVec>) -> Self {
        Self { recv, buffer: None }
    }

    fn consume_internal(&mut self, amt: usize) {
        if let Some((buf, offset)) = &mut self.buffer {
            *offset += amt;
            debug_assert!(*offset <= buf.len());
            if *offset == buf.len() {
                self.buffer = None;
            }
        } else {
            debug_assert!(amt == 0);
        }
    }
}
impl AsyncRead for ReadStream {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        // Defer to `AsyncBufRead`.
        let read_buf = ready!(self.as_mut().poll_fill_buf(cx))?;
        let amt = std::cmp::min(read_buf.len(), buf.capacity());
        buf.put_slice(&read_buf[..amt]);
        self.consume(amt);
        Poll::Ready(Ok(()))
    }
}
impl AsyncBufRead for ReadStream {
    fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<&[u8]>> {
        let this = self.get_mut();

        if this.buffer.is_none() {
            let opt_data = ready!(this.recv.poll_recv(cx));
            this.buffer = opt_data.map(|data| (data, 0));
        }

        Poll::Ready(Ok(this
            .buffer
            .as_ref()
            .map(|(buf, offset)| &buf[*offset..])
            .unwrap_or(&[])))
    }

    fn consume(self: Pin<&mut Self>, amt: usize) {
        let this = self.get_mut();
        this.consume_internal(amt)
    }
}
impl Read for ReadStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // Defer to `BufRead`.
        let read_buf = self.fill_buf()?;
        let amt = std::cmp::min(read_buf.len(), buf.len());
        buf.copy_from_slice(&read_buf[..amt]);
        self.consume(amt);
        Ok(amt)
    }
}
impl BufRead for ReadStream {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        if self.buffer.is_none() {
            let opt_data = self.recv.blocking_recv();
            self.buffer = opt_data.map(|data| (data, 0));
        }

        Ok(self.buffer.as_ref().map(|(buf, offset)| &buf[*offset..]).unwrap_or(&[]))
    }

    fn consume(&mut self, amt: usize) {
        self.consume_internal(amt)
    }
}
