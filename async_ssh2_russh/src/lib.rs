#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", env!("CARGO_PKG_README")))]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

use std::collections::HashMap;
use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;

use russh::client::{Config, Handle, Handler, Msg, connect};
use russh::keys::{PrivateKeyWithHashAlg, load_secret_key, ssh_key};
use russh::{ChannelMsg, ChannelWriteHalf, CryptoVec};
use tokio::io::AsyncWrite;
use tokio::net::ToSocketAddrs;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

// `pub` items
#[cfg(feature = "sftp")]
#[cfg_attr(docsrs, doc(cfg(feature = "sftp")))]
pub mod sftp;
#[doc(no_inline)]
pub use russh::Error as SshError;
#[cfg(feature = "sftp")]
#[cfg_attr(docsrs, doc(cfg(feature = "sftp")))]
pub use russh_sftp;
pub use {russh, tokio};
mod promise;
pub use promise::{Promise, PromiseError};
mod read_stream;
pub use read_stream::ReadStream;

/// A handler that does not check the server's public key.
///
/// This should NOT be used unless
pub struct NoCheckHandler;
impl Handler for NoCheckHandler {
    type Error = SshError;

    async fn check_server_key(&mut self, _server_public_key: &ssh_key::PublicKey) -> Result<bool, Self::Error> {
        // TODO(mingwei): we technically should check the server's public key fingerprint here
        // (get it somehow via terraform), but ssh `publickey` authentication already generally
        // prevents MITM attacks.
        Ok(true)
    }
}

/// A thin wrapper around [`russh::client::Handle`] which provides basic authentication and channel
/// management for a SSH session.
///
/// Implements [`Deref`] to allow access to the underlying [`russh::client::Handle`].
pub struct AsyncSession<H: Handler = NoCheckHandler> {
    session: Handle<H>,
}
impl<H: Handler + 'static> AsyncSession<H> {
    /// Connect to an SSH server using the provided configuration and handler, without begining
    /// authentication.
    pub async fn connect_unauthenticated(
        config: Arc<Config>,
        addrs: impl ToSocketAddrs,
        handler: H,
    ) -> Result<Self, H::Error> {
        let session = connect(config, addrs, handler).await?;
        Ok(Self { session })
    }
}

impl AsyncSession {
    /// Connect to an SSH server and authenticate with the given `user` and `key_path` via publickey
    /// authentication.
    ///
    /// Uses [`NoCheckHandler`] to skip server public key verification, as publickey authentication provides protection
    /// against MITM attacks.
    pub async fn connect_publickey(
        config: impl Into<Arc<Config>>,
        addrs: impl ToSocketAddrs,
        user: impl Into<String>,
        key_path: impl AsRef<Path>,
    ) -> Result<Self, SshError> {
        let key_pair = load_secret_key(key_path, None)?;

        let mut session = connect(config.into(), addrs, NoCheckHandler).await?;

        // use publickey authentication.
        let auth_res = session
            .authenticate_publickey(
                user,
                PrivateKeyWithHashAlg::new(Arc::new(key_pair), session.best_supported_rsa_hash().await?.flatten()),
            )
            .await?;

        if auth_res.success() {
            Ok(Self { session })
        } else {
            Err(SshError::NotAuthenticated)
        }
    }

    /// Opens an [`AsyncChannel`] in this session.
    ///
    /// [`AsyncChannel`] is the asnyc wrapper for [`russh::Channel`].
    pub async fn open_channel(&self) -> Result<AsyncChannel, SshError> {
        let russh_channel = self.session.channel_open_session().await?;
        Ok(AsyncChannel::from(russh_channel))
    }
}

impl<H: Handler> Deref for AsyncSession<H> {
    type Target = Handle<H>;
    fn deref(&self) -> &Self::Target {
        &self.session
    }
}

/// A thin wrapper around [`russh::Channel`] which provides access to async read/write streams (stdout/stderr/stdin) and
/// async event handling.
///
/// Implements [`Deref`] to allow access to the underlying [`russh::ChannelWriteHalf`].
///
/// # Shutdown lifecycle
///
/// During shutdown, events _may_ be received in the following order. However this should not be relied upon, as the
/// order may be different and none of these events are guaranteed to occur, except for [`Self::wait_close`] which will
/// always happen last.
///
/// 1. [`Self::recv_success_failure`].
/// 2. [`Self::recv_eof`] - Guarantees all stream data has been received, i.e. stdout/stderr will produce no more data.
///    Channels may be closed without sending EOF; see [this StackOverflow answer](https://stackoverflow.com/a/23257958).
/// 3. [`Self::recv_exit_status`] - The exit status of the command run, if applicable.
/// 4. [`Self::wait_close`] - This channel is closed, no more events will occur.
pub struct AsyncChannel {
    write_half: ChannelWriteHalf<Msg>,
    subscribe_send: mpsc::UnboundedSender<(Option<u32>, mpsc::UnboundedSender<CryptoVec>)>,
    success_failure: Promise<bool>,
    eof: Promise<()>,
    exit_status: Promise<u32>,
    reader: JoinHandle<()>,
}

impl From<russh::Channel<Msg>> for AsyncChannel {
    fn from(inner: russh::Channel<Msg>) -> Self {
        let (mut read_half, write_half) = inner.split();
        let (mut resolve_success_failure, success_failure) = promise::channel();
        let (mut resolve_eof, eof) = promise::channel();
        let (mut resolve_exit_status, exit_status) = promise::channel();
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

impl AsyncChannel {
    /// Returns the specified stream as a [`ReadStream`].
    ///
    /// Note that the returned stream will only receive data after this call, so call this before calling
    /// [`exec`](ChannelWriteHalf::exec).
    ///
    /// When this is called for the same `ext` more than once, the later call will disconnect the
    /// first.
    pub fn read_stream(&self, ext: Option<u32>) -> ReadStream {
        let (send, recv) = mpsc::unbounded_channel();
        let _ = self.subscribe_send.send((ext, send));
        ReadStream::from_recv(recv)
    }

    /// Returns stdout as an stream as a [`ReadStream`].
    ///
    /// Note that the returned stream will only receive data after this call, so call this before calling
    /// [`exec`](ChannelWriteHalf::exec).
    ///
    /// When this is called more than once, the later call will disconnect the first.
    pub fn stdout(&self) -> ReadStream {
        self.read_stream(None)
    }

    /// Returns stderr as an stream as a [`ReadStream`].
    ///
    /// Note that the returned stream will only receive data after this call, so call this before calling
    /// [`exec`](ChannelWriteHalf::exec).
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

    /// Resolves when success or failure has been received, where `true` indicates success.
    pub fn recv_success_failure(&self) -> &Promise<bool> {
        &self.success_failure
    }

    /// Resolves when EOF has been received, indicating all stream data is complete.
    ///
    /// At that point, any streams from [`Self::stdout`]/[`Self::stderr`]/[`Self::read_stream`]
    /// will return no additional data.
    pub fn recv_eof(&self) -> &Promise<()> {
        &self.eof
    }

    /// Resolves when the command exit status has been received.
    pub fn recv_exit_status(&self) -> &Promise<u32> {
        &self.exit_status
    }

    /// Returns when the channel has been closed.
    ///
    /// After this point, no more events will resolve.
    pub async fn wait_close(&mut self) {
        let _ = (&mut self.reader).await;
    }

    /// Returns if the channel has been closed. See [`Self::wait_close`].
    pub fn is_closed(&self) -> bool {
        self.reader.is_finished()
    }
}

impl Deref for AsyncChannel {
    type Target = ChannelWriteHalf<Msg>;
    fn deref(&self) -> &Self::Target {
        &self.write_half
    }
}
