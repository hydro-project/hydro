use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::path::Path;
use std::pin::Pin;
use std::sync::atomic::{AtomicU8, Ordering};
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

/// SSH channel.
///
/// Shutdown lifecycle (may vary):
/// 1. [`Self::wait_success_failure`].
/// 2. [`Self::wait_eof`] - Guarantees all stream data has been received, stdout/stderr/etc. will produce no more data.
/// 3. [`Self::wait_exit_status`] - The exit status of the command run.
/// 4. [`Self::wait_close`] - This channel is closed, no more events will occur. Will always occur last.
///
/// If the channel is closed abruptly, only [`Self::wait_close`] is guaranteed to occur.
pub struct Channel {
    write_half: ChannelWriteHalf<Msg>,
    subscribe_send: mpsc::UnboundedSender<(Option<u32>, mpsc::UnboundedSender<CryptoVec>)>,
    success_failure: Promise<bool>,
    eof: Promise<()>,
    exit_status: Promise<u32>,
    reader: JoinHandle<()>,
}

impl From<russh::Channel<Msg>> for Channel {
    fn from(inner: russh::Channel<Msg>) -> Self {
        let (mut read_half, write_half) = inner.split();
        let (mut resolve_success_failure, success_failure) = promise();
        let (mut resolve_eof, eof) = promise();
        let (mut resolve_exit_status, exit_status) = promise();
        let (subscribe_send, mut subscribe_recv) = mpsc::unbounded_channel();

        let reader = tokio::task::spawn(async move {
            type Subscribers = Option<HashMap<Option<u32>, mpsc::UnboundedSender<CryptoVec>>>;
            // Map from `ext` to a sender for `CryptoVec`s of data.
            let mut subscribers: Subscribers = Some(HashMap::new());

            fn receive_data(subscribers: &Subscribers, ext: Option<u32>, data: CryptoVec) {
                if let Some(subscribers) = &subscribers {
                    if let Some(send) = subscribers.get(&ext) {
                        let _ = send.send(data);
                    }
                } else {
                    // Unexpectedly received data after EOF.
                }
            }

            loop {
                tokio::select! {
                    biased;
                    Some((ext, send)) = subscribe_recv.recv() => {
                        if let Some(subscribers) = &mut subscribers {
                            subscribers.insert(ext, send);
                        } else {
                            // Subscribing after EOF was received.
                        }
                    },
                    opt_msg = read_half.wait() => {
                        let Some(msg) = opt_msg else {
                            // No more messages, exit!
                            break;
                        };
                        match msg {
                            ChannelMsg::Data { data } => receive_data(&subscribers, None, data),
                            ChannelMsg::ExtendedData { data, ext } => receive_data(&subscribers, Some(ext), data),
                            ChannelMsg::Success => {
                                let _ = resolve_success_failure.resolve(true);
                            }
                            ChannelMsg::Failure => {
                                let _ = resolve_success_failure.resolve(false);
                            }
                            // The command has indicated no more `ChannelMsg::Data`/`ChannelMsg::ExtendedData` will be sent.
                            ChannelMsg::Eof => {
                                let _ = resolve_eof.resolve(());
                                // Disconnect all subscribers.
                                drop(std::mem::take(&mut subscribers));
                            }
                            // The command has returned an exit code
                            ChannelMsg::ExitStatus { exit_status } => {
                                let _ = resolve_exit_status.resolve(exit_status);
                            }
                            // Other
                            _ => {}
                        }
                    },
                }
            }
            // Exiting causes the `self.reader` `JoinHandle` to close.
        });

        Self {
            write_half,
            subscribe_send,
            success_failure,
            eof,
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

    pub async fn wait_success_failure(&self) -> Option<bool> {
        self.success_failure.wait().await.copied()
    }

    pub fn get_success_failure(&self) -> Result<bool, PromiseError> {
        self.success_failure.get().copied()
    }

    /// Returns when EOF has been received, indicating all stream data is complete.
    ///
    /// At that point, any streams from [`Self::stdout`]/[`Self::stderr`]/[`Self::read_stream`]
    /// will return no additional data.
    ///
    /// If this returns `None`, the channel was closed before EOF was received.
    pub async fn wait_eof(&self) -> Option<()> {
        self.eof.wait().await.copied()
    }

    /// Returns if EOF has been receieved, indicating all stream data is complete.
    ///
    /// At that point, any streams from [`Self::stdout`]/[`Self::stderr`]/[`Self::read_stream`]
    /// will return no additional data.
    ///
    /// If this returns [`Err(PromiseError::Dropped)`](PromiseError::Dropped), the channel was
    /// closed before EOF was received.
    pub fn get_eof(&self) -> Result<(), PromiseError> {
        self.eof.get().copied()
    }

    /// Returns when the command exit status has been received.
    ///
    /// If this returns `None`, the channel was closed before the exit status was received.
    pub async fn wait_exit_status(&self) -> Option<u32> {
        self.exit_status.wait().await.copied()
    }

    /// Returns the command exit status, if it has been received.
    ///
    /// If this returns [`Err(PromiseError::Dropped)`](PromiseError::Dropped), the channel was
    /// closed before the exit status was received.
    pub fn get_exit_status(&self) -> Result<u32, PromiseError> {
        self.exit_status.get().copied()
    }

    /// Returns when the channel has been closed (no more events will occur).
    pub async fn wait_close(&mut self) {
        let _ = (&mut self.reader).await;
    }

    /// Returns if the channel has been closed (no more events will occur).
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

// SAFETY: must not be clonable.
pub struct Resolve<T> {
    inner: Option<Weak<Inner<T>>>,
}
impl<T> Resolve<T> {
    pub fn into_resolve(mut self, value: T) {
        self.resolve(value)
            .unwrap_or_else(|_| panic!("already resolved"));
    }

    pub fn resolve(&mut self, value: T) -> Result<(), T> {
        let Some(inner) = self.inner.take().and_then(|weak| weak.upgrade()) else {
            return Err(value);
        };

        // SAFETY: `&mut self: Resolve` has exclusive access to `set_value`, once.
        unsafe {
            inner.value.get().write(MaybeUninit::new(value));
        }

        // Using release ordering so any threads that read a true from this
        // atomic is able to read the value we just stored.
        inner
            .flag
            .store(FLAG_RESOLVED | FLAG_INITIALIZED, Ordering::Release);
        inner.semaphore.close();

        Ok(())
    }
}
impl<T> Drop for Resolve<T> {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.take().and_then(|weak| weak.upgrade()) {
            // SAFETY: `&mut self: Resolve` has exclusive access to `set_value`, once.
            inner.flag.store(FLAG_RESOLVED, Ordering::Release);
            inner.semaphore.close();
        }
    }
}

pub struct Promise<T> {
    inner: Arc<Inner<T>>,
}
impl<T> Promise<T> {
    pub async fn wait(&self) -> Option<&T> {
        self.inner.semaphore.acquire().await.unwrap_err();
        self.get().ok()
    }

    pub fn get(&self) -> Result<&T, PromiseError> {
        self.inner.get()
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum PromiseError {
    /// The resolver has not yet sent a value.
    Unresolved,
    /// The resolver was dropped before sending a value.
    Dropped,
}
impl std::fmt::Display for PromiseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unresolved => write!(f, "not yet resovled"),
            Self::Dropped => write!(f, "resolver dropped without resolving"),
        }
    }
}
impl std::error::Error for PromiseError {}

const FLAG_RESOLVED: u8 = 0b01;
const FLAG_INITIALIZED: u8 = 0b10;

// Based on [`tokio::sync::OnceCell`].
//
// Any thread with an `&self` may access the `value` field according the following rules:
//
//  1. Iff `flag & FLAG_RESOLVED` is false, the `value` field may be modified by the
//     thread holding the permit on the semaphore.
//  2. Iff `flag & FLAG_INITIALIZED` is true, the `value` field may be accessed immutably by
//     any thread.
//
// If `flag & FLAG_RESOLVED` is true, but `flag & FLAG_INITIALIZED` is false, then the resolver
// was dropped before the value was set.
struct Inner<T> {
    flag: AtomicU8,
    value: UnsafeCell<MaybeUninit<T>>,
    semaphore: Semaphore,
}
impl<T> Inner<T> {
    fn new() -> Self {
        Self {
            flag: AtomicU8::new(0),
            value: UnsafeCell::new(MaybeUninit::uninit()),
            semaphore: Semaphore::const_new(0),
        }
    }

    fn get(&self) -> Result<&T, PromiseError> {
        // Using acquire ordering so any threads that read a true from this
        // atomic is able to read the value.
        let flag = self.flag.load(Ordering::Acquire);
        let resolved = 0 != flag & FLAG_RESOLVED;
        if resolved {
            let initialized = 0 != flag & FLAG_INITIALIZED;
            if initialized {
                // SAFETY: Value is initialized.
                Ok(unsafe { &*(*self.value.get()).as_ptr() })
            } else {
                Err(PromiseError::Dropped)
            }
        } else {
            Err(PromiseError::Unresolved)
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
