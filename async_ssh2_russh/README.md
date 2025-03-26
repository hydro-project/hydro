# `async_ssh2_russh`

An asynchronous SSH client wrapper around [`russh`](https://crates.io/crates/russh).

## Features

Thin wrapper around [`russh`](https://crates.io/crates/russh), providing a few additional features:

* Asynchronous reading from `stdout` and `stderr` separately.
* Asynchronous writing to `stdin`.
* Asynchronous event handling for exit status codes, EOF, closing, etc.
* [`AsyncSession::open_sftp`] for SFTP support via [`russh-sftp`](https://crates.io/crates/russh-sftp) (requires the `sftp` feature).

## Usage

This crate provides two main types, [`AsyncSession`] and [`AsyncChannel`]. A session represents a connection to an SSH
server, while a channel represents a single communication thread within that session, for example for executing a
command, or opening an SFTP session, etc. These two structs are thin wrappers around [`russh::client::Handler`] and
[`russh::ChannelWriteHalf`], respectively, with additional methods for asynchronous stream and event handling. They each
implement [`Deref`] for their underlying types, so you can use them as if they were the original types.

### Example

```rust,no_run
# let _ = async {
# let my_target_addr = "127.0.0.1:22";
# let my_ssh_user = "user";
# let my_ssh_key_path = "/path/to/private/key";

use async_ssh2_russh::AsyncSession;
use async_ssh2_russh::russh::client::Config;
use async_ssh2_russh::russh::compression;
use async_ssh2_russh::tokio::io::AsyncBufReadExt;

// Configure to try to use compression, for example.
let mut config = Config::default();
config.preferred.compression = (&[
    compression::ZLIB,
    compression::ZLIB_LEGACY,
    compression::NONE,
]).into();

// Connect and authenticate to the SSH server using public key authentication.
let session = AsyncSession::connect_publickey(
    config,
    my_target_addr,
    my_ssh_user,
    my_ssh_key_path,
).await.unwrap();

let mut channel = session.open_channel().await.unwrap();
// Connect stdout before running the command, to ensure no output is lost.
let mut stdout = channel.stdout().lines();
channel.exec(false, "echo 'Hello, world!'").await.unwrap();
while let Some(line) = stdout.next_line().await.unwrap() {
    println!("Command output: {}", line);
}
println!("Command finished with exit status: {:?}", channel.recv_exit_status().wait().await);

// Send close to server.
channel.close().await.unwrap();
// Wait to receive close back.
channel.wait_close().await;

# };
```

## Comparisons

* Why not use [`russh`](https://crates.io/crates/russh) directly? `russh` does not provide easy ways to read from
  `stdout` and `stderr` separately, or to wait for specific events like exit status codes or EOF. Perhaps this
  functionality will be subsumed by `russh` in the future and make this crate obsolete.
* Why not [`async-ssh2-tokio`](https://crates.io/crates/async-ssh2-tokio)? `async-ssh2-tokio` is also a wrapper around
  `russh`, but only provides a monolithic [`execute`](https://docs.rs/async-ssh2-tokio/0.8.14/async_ssh2_tokio/client/struct.Client.html#method.execute)
  method which [prevents asynchronous/dynamic interaction with the command's `stdout`, `stderr`, and `stdin`](https://github.com/Miyoshi-Ryota/async-ssh2-tokio/issues/62).
* Why not [`async-ssh2-lite`](https://crates.io/crates/async-ssh2-lite)? `async-ssh2-lite` is a wrapper around the
  `libssh2` C library, causing additional build complexity and compile time. However `async-ssh2-lite`'s APIs are very
  similar to this crate's, and likely more complete.
