//! Storj DCS Encryption key.

use crate::uplink_c::Ensurer;
use crate::{helpers, Error, Result};

use uplink_sys as ulksys;

/// Represents a key for encrypting and decrypting data.
#[derive(Debug)]
pub struct EncryptionKey {
    /// The encryption key type of the FFI that an instance of this struct represents and guards its
    /// lifetime until the instances drops.
    /// It's an encryption result because it's the one that holds the encryption
    /// key and allows to free its memory.
    inner: ulksys::UplinkEncryptionKeyResult,
}

impl EncryptionKey {
    /// Derives a salted encryption key for `passphrase` using the passed salt.
    ///
    /// It's mostly useful for implementing multitenancy in a single app bucket.
    /// See [Multitenancy in a Single Application Bucket](https://pkg.go.dev/storj.io/uplink#hdr-Multitenancy_in_a_Single_Application_Bucket)
    /// section in the original Uplink library.
    pub fn derive(passphrase: &str, salt: &[u8]) -> Result<Self> {
        use std::ffi::c_void;
        use std::os::raw::c_char;

        let passphrase = helpers::cstring_from_str_fn_arg("passphrase", passphrase)?;

        // SAFETY: we trust that the FFI is safe creating an instance of its own types.
        // Note that we get a non-mutable pointer to the `salt` argument but we apply a conversion
        // to to mutable rather than using the `as_mut_ptr` method because otherwise it will require
        // the `salt` parameter to be mutable but the FFI function doesn't mutate it despite that
        // the function parameters is specified as mutable.
        let uc_res = unsafe {
            ulksys::uplink_derive_encryption_key(
                passphrase.as_ptr() as *mut c_char,
                salt.as_ptr() as *mut c_void,
                salt.len() as u64,
            )
        };

        (&uc_res).ensure();

        if let Some(err) = Error::new_uplink(uc_res.error) {
            // SAFETY: we trust the FFI is safe freeing the memory of a valid pointer.
            unsafe { ulksys::uplink_free_encryption_key_result(uc_res) };
            return Err(err);
        }

        Ok(Self { inner: uc_res })
    }

    /// Returns the FFI representation of this encryption key.
    pub(crate) fn as_ffi_encryption_key(&self) -> *mut ulksys::UplinkEncryptionKey {
        self.inner.encryption_key
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::error;
    use crate::Error;

    #[test]
    fn test_derive_invalid_argument() {
        if let Error::InvalidArguments(error::Args { names, msg }) =
            EncryptionKey::derive("pass\0phrase", &[0])
                .expect_err("when passing a passphrase with NULL bytes")
        {
            assert_eq!(names, "passphrase", "invalid error argument name");
            assert_eq!(
                msg, "cannot contains null bytes (0 byte). Null byte found at 4",
                "invalid error argument message"
            );
        } else {
            panic!("expected an invalid argument error");
        }
    }
}
