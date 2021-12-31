//! Storj DCS Uplink idiomatic and safe Rust bindings.

#![deny(missing_docs)]

pub(crate) mod config;
pub(crate) mod encryption_key;
pub(crate) mod error;
pub(crate) mod helpers;
pub(crate) mod project;

pub mod access;
pub mod bucket;
pub mod metadata;
pub use config::Config;
pub use encryption_key::EncryptionKey;
pub use error::Error;
pub use project::Project;

/// A specialized [`Result`](https://doc.rust-lang.org/std/result/enum.Result.html)
/// type for Storj Uplink operations.
///
/// This type is broadly used across this crate for any operations which may
/// produce an error.
///
/// This type is generally used to avoid writing out `storj_uplink_lib::Error`
/// directly and reduce repetition making the signature functions more concise.
pub type Result<T> = std::result::Result<T, error::Error>;

/// An interface for ensuring that an instance of type returned by the
/// underlying c-binding is correct in terms that it doesn't violate its own
/// rules.
/// For example a UplinkAccessResult struct has 2 fields which are 2 pointers,
/// one is the access and the other is an error, always one and only one can be
/// NULL.
trait Ensurer {
    /// Checks that the instance is correct according its own rules and it
    /// returns itself, otherwise it panics.
    fn ensure(&self) -> &Self;
}
