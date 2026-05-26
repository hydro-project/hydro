//! Feature-gated backend implementations of `ReplicableService`.

#[cfg(feature = "backend_redb")]
pub mod redb;

#[cfg(feature = "backend_sled")]
pub mod sled;

#[cfg(feature = "backend_fjall")]
pub mod fjall;

#[cfg(feature = "backend_rusqlite")]
pub mod rusqlite;
