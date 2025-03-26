//! SFTP support for the `async_ssh2_russh` crate.

use russh_sftp::client::SftpSession;
pub use russh_sftp::client::error::Error as SftpError;

use crate::{AsyncChannel, AsyncSession, SshError};

impl AsyncSession {
    /// Opens an SFTP channel.
    ///
    /// Equivalent to [`Session::open_channel()`] followed by requesting the SFTP subsystem:
    /// ```rust,ignore
    /// channel.request_subsystem(true, "sftp").await?;
    /// ```
    #[cfg(feature = "sftp")]
    pub async fn open_sftp(&self) -> Result<SftpSession, SshOrSftpError> {
        let channel = self.open_channel().await?;
        channel.request_subsystem(true, "sftp").await?;
        let sftp = channel.sftp().await?;
        Ok(sftp)
    }
}

impl AsyncChannel {
    /// Starst an SFTP session on this channel.
    ///
    /// Make sure this channel was opened with [`Session::open_sftp`], or if not, make sure to
    /// request the SFTP subsystem before calling this:
    /// ```rust,ignore
    /// channel.request_subsystem(true, "sftp").await.unwrap();
    /// ```
    #[cfg(feature = "sftp")]
    pub async fn sftp(&self) -> Result<SftpSession, SftpError> {
        SftpSession::new(tokio::io::join(self.stdout(), self.stdin())).await
    }
}

/// Error enum containing either an [`SshError`] or [`SftpError`].
///
/// This is used to unify error handling for SSH and SFTP operations.
#[derive(Debug, thiserror::Error)]
pub enum SshOrSftpError {
    /// SSH error.
    #[error("SSH Error: {0}")]
    SshError(#[from] SshError),
    /// SFTP error.
    #[error("SFTP Error: {0}")]
    SftpError(#[from] SftpError),
}
