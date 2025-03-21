use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::path::Path;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};
use std::task::{Context, Poll, ready};

use anyhow::Error;
use russh::client::{Config, Handle, Handler, Msg, connect};
use russh::keys::{PrivateKeyWithHashAlg, load_secret_key, ssh_key};
use russh::{ChannelMsg, ChannelWriteHalf, CryptoVec};
use russh_sftp::client::SftpSession;
use russh_sftp::client::rawsession::SftpResult;
use tokio::io::{AsyncBufRead, AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::ToSocketAddrs;
use tokio::sync::{Semaphore, mpsc};
use tokio::task::JoinHandle;

use crate::progress::ProgressTracker;

pub struct NoCheckHandler;
impl Handler for NoCheckHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        // TODO(mingwei): we technically should check the server's public key fingerprint here
        // (get it somehow via terraform), but ssh `publickey` authentication already generally
        // prevents MITM attacks.
        Ok(true)
    }
}

// https://github.com/Eugeny/russh/blob/main/russh/examples/client_exec_simple.rs
/// This struct is a convenience wrapper
/// around a russh client
pub struct Session {
    session: Handle<NoCheckHandler>,
}

impl Session {
    pub async fn connect(
        config: impl Into<Arc<Config>>,
        key_path: impl AsRef<Path>,
        user: impl Into<String>,
        addrs: impl ToSocketAddrs,
    ) -> Result<Self, russh::Error> {
        let key_pair = load_secret_key(key_path, None)?;

        let mut session = connect(config.into(), addrs, NoCheckHandler).await?;

        // use publickey authentication.
        let auth_res = session
            .authenticate_publickey(
                user,
                PrivateKeyWithHashAlg::new(
                    Arc::new(key_pair),
                    session.best_supported_rsa_hash().await?.flatten(),
                ),
            )
            .await?;

        if auth_res.success() {
            Ok(Self { session })
        } else {
            Err(russh::Error::NotAuthenticated)
        }
    }

    pub async fn open_channel(&self) -> Result<Channel, russh::Error> {
        let russh_channel = self.session.channel_open_session().await?;
        Ok(Channel::from(russh_channel))
    }

    pub async fn open_sftp(&self) -> Result<SftpSession, Error> {
        let channel = self.open_channel().await?;
        channel.request_subsystem(true, "sftp").await?;
        let sftp = channel.sftp().await?;
        Ok(sftp)
    }
}

impl Deref for Session {
    type Target = Handle<NoCheckHandler>;
    fn deref(&self) -> &Self::Target {
        &self.session
    }
}

pub struct Channel {
    write_half: ChannelWriteHalf<Msg>,
    subscribe_send: mpsc::UnboundedSender<(Option<u32>, mpsc::UnboundedSender<CryptoVec>)>,
    exit_status: Promise<u32>,
    reader: JoinHandle<()>,
}

impl From<russh::Channel<Msg>> for Channel {
    fn from(inner: russh::Channel<Msg>) -> Self {
        let (mut read_half, write_half) = inner.split();
        let (mut resolve_exit_status, exit_status) = promise();
        let (subscribe_send, mut subscribe_recv) = mpsc::unbounded_channel();

        let reader = tokio::task::spawn(async move {
            // Map from `ext` to a sender for `CryptoVec`s of data.
            let mut subscribers: HashMap<Option<u32>, mpsc::UnboundedSender<CryptoVec>> =
                HashMap::new();
            loop {
                tokio::select! {
                    biased;
                    Some((ext, send)) = subscribe_recv.recv() => {
                        subscribers.insert(ext, send);
                    },
                    opt_msg = read_half.wait() => {
                        let Some(msg) = opt_msg else {
                            // No more messages, exit!
                            break;
                        };
                        let (data, ext) = match msg {
                            // Write data to the terminal
                            ChannelMsg::Data { data } => (data, None),
                            ChannelMsg::ExtendedData { data, ext } => (data, Some(ext)),
                            // The command has returned an exit code
                            ChannelMsg::ExitStatus { exit_status } => {
                                ProgressTracker::eprintln(format!("EXIT STATUS: {}", exit_status));
                                let _ = resolve_exit_status.resolve(exit_status);
                                continue;
                            }
                            ChannelMsg::WindowChange { .. } | ChannelMsg::WindowAdjusted { .. } => continue,
                            other => {
                                ProgressTracker::eprintln(format!("Channel message: {:?}", other));
                                continue;
                            }
                        };

                        if let Some(send) = subscribers.get(&ext) {
                            let _ = send.send(data);
                        }
                    },
                }
            }
            ProgressTracker::eprintln(format!("READER EXIT!!!!!!"));
        });

        Self {
            write_half,
            subscribe_send,
            exit_status,
            reader,
        }
    }
}

impl Channel {
    /// Returns the specified stream as an [`impl AsyncRead`](AsyncRead).
    ///
    /// When this is called for the same `ext` more than once, the later call will disconnect the
    /// first.
    pub fn read_stream(&self, ext: Option<u32>) -> ReadStream {
        let (send, recv) = mpsc::unbounded_channel();
        let _ = self.subscribe_send.send((ext, send));
        ReadStream { recv, buffer: None }
    }

    /// Returns stdout as an [`impl AsyncRead`](AsyncRead).
    ///
    /// When this is called more than once, the later call will disconnect the first.
    pub fn stdout(&self) -> ReadStream {
        self.read_stream(None)
    }

    /// Returns stderr as an [`impl AsyncRead`](AsyncRead).
    ///
    /// When this is called more than once, the later call will disconnect the first.
    pub fn stderr(&self) -> ReadStream {
        self.read_stream(Some(1))
    }

    /// Returns the specified stream as an [`impl AsyncWrite`](AsyncWrite).
    ///
    /// When this is called for the same `ext` more than once, writes to each may be interleaved.
    pub fn write_stream(&self, ext: Option<u32>) -> impl 'static + AsyncWrite {
        self.write_half.make_writer_ext(ext)
    }

    /// Returns stdin as an [`impl AsyncWrite`](AsyncWrite).
    ///
    /// When this is called more than once, writes to each may be interleaved.
    pub fn stdin(&self) -> impl 'static + AsyncWrite {
        self.write_stream(None)
    }

    /// Starst an SFTP session on this channel.
    ///
    /// Make sure to request the SFTP subsystem before calling this:
    /// ```rust,ignore
    /// channel.request_subsystem(true, "sftp").await.unwrap();
    /// ```
    pub async fn sftp(&self) -> SftpResult<SftpSession> {
        SftpSession::new(tokio::io::join(self.stdout(), self.stdin())).await
    }

    pub async fn wait_exit_status(&self) -> u32 {
        *self.exit_status.wait().await
    }

    pub fn get_exit_status(&self) -> Option<u32> {
        self.exit_status.get().copied()
    }

    pub async fn wait_close(&mut self) {
        let _ = (&mut self.reader).await;
    }

    pub fn is_closed(&self) -> bool {
        self.reader.is_finished()
    }
}

impl Deref for Channel {
    type Target = ChannelWriteHalf<Msg>;
    fn deref(&self) -> &Self::Target {
        &self.write_half
    }
}

pub struct ReadStream {
    recv: mpsc::UnboundedReceiver<CryptoVec>,
    buffer: Option<(CryptoVec, usize)>,
}
impl AsyncRead for ReadStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        // Defer to the underlying `AsyncBufRead` implementation.
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
        if let Some((buf, offset)) = &mut this.buffer {
            *offset += amt;
            debug_assert!(*offset <= buf.len());
            if *offset == buf.len() {
                this.buffer = None;
            }
        } else {
            debug_assert!(amt == 0);
        }
    }
}

pub fn promise<T>() -> (Resolve<T>, Promise<T>) {
    let inner = Arc::new(Inner::new());
    (
        Resolve {
            inner: Some(Arc::downgrade(&inner)),
        },
        Promise { inner },
    )
}

pub struct Resolve<T> {
    inner: Option<Weak<Inner<T>>>,
}
impl<T> Resolve<T> {
    pub fn resolve(&mut self, value: T) -> Result<(), T> {
        if let Some(inner) = self.inner.take().as_ref().and_then(Weak::upgrade) {
            // SAFETY: `&mut self: Resolve` has exclusive access to `set_value`, once.
            unsafe { inner.set_value(value) };
            Ok(())
        } else {
            Err(value)
        }
    }
}

pub struct Promise<T> {
    inner: Arc<Inner<T>>,
}
impl<T> Promise<T> {
    pub async fn wait(&self) -> &T {
        self.inner.semaphore.acquire().await.unwrap_err();
        self.get().unwrap()
    }

    pub fn get(&self) -> Option<&T> {
        self.inner.get_value()
    }

    pub fn initialized(&self) -> bool {
        self.get().is_some()
    }
}

// Based on [`tokio::sync::OnceCell`].
//
// Any thread with an `&self` may access the `value` field according the following rules:
//
//  1. When `value_set` is false, the `value` field may be modified by the
//     thread holding the permit on the semaphore.
//  2. When `value_set` is true, the `value` field may be accessed immutably by
//     any thread.
//
// It is an invariant that if the semaphore is closed, then `value_set` is true.
// The reverse does not necessarily hold — but if not, the semaphore may not
// have any available permits.
struct Inner<T> {
    value_set: AtomicBool,
    value: UnsafeCell<MaybeUninit<T>>,
    semaphore: Semaphore,
}
impl<T> Inner<T> {
    fn new() -> Self {
        Self {
            value_set: AtomicBool::new(false),
            value: UnsafeCell::new(MaybeUninit::uninit()),
            semaphore: Semaphore::const_new(0),
        }
    }

    /// SAFETY: Must only be called once, by the `Resolve` that owns this `Inner`.
    unsafe fn set_value(&self, value: T) {
        // SAFETY: We are holding the only permit on the semaphore.
        unsafe {
            self.value.get().write(MaybeUninit::new(value));
        }

        // Using release ordering so any threads that read a true from this
        // atomic is able to read the value we just stored.
        self.value_set.store(true, Ordering::Release);
        self.semaphore.close();
    }

    fn get_value(&self) -> Option<&T> {
        // Using acquire ordering so any threads that read a true from this
        // atomic is able to read the value.
        let initialized = self.value_set.load(Ordering::Acquire);
        if initialized {
            // SAFETY: Value is initialized.
            Some(unsafe { &*(*self.value.get()).as_ptr() })
        } else {
            None
        }
    }
}

// Since `get` gives us access to immutable references of the OnceCell, OnceCell
// can only be Sync if T is Sync, otherwise OnceCell would allow sharing
// references of !Sync values across threads. We need T to be Send in order for
// OnceCell to by Sync because we can use `set` on `&OnceCell<T>` to send values
// (of type T) across threads.
unsafe impl<T: Sync + Send> Sync for Inner<T> {}

// Access to OnceCell's value is guarded by the semaphore permit
// and atomic operations on `value_set`, so as long as T itself is Send
// it's safe to send it to another thread
unsafe impl<T: Send> Send for Inner<T> {}
