//! Storj DCS Encryption key.

use crate::{helpers, Ensurer, Result};

use uplink_sys as ulksys;

/// Represents a key for encrypting and decrypting data.
#[derive(Debug)]
pub struct EncryptionKey {
    /// The encryption key type of the underlying c-bindings Rust crate that an
    /// instance of this struct represents and guards its life time until the
    /// instances drops.
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

        // SAFETY: we trust that the underlying c-binding is safe, nonetheless
        // we ensure enckres is correct through the ensure method of the
        // implemented Ensurer trait.
        // Note that we get a non-mutable pointer to the `salt` argument but we
        // apply a conversion to to mutable rather than using the `as_mut_ptr`
        // method because otherwise it will require the `salt` parameter to be
        // mutable but the c-binding function doesn't mutate it despite that
        // the function parameters is specified as mutable.
        let enckres = unsafe {
            ulksys::uplink_derive_encryption_key(
                passphrase.as_ptr() as *mut c_char,
                salt.as_ptr() as *mut c_void,
                salt.len() as u64,
            )
        };

        Ok(Self { inner: enckres })
    }

    /// Returns the underlying c-bindings representation of this encryption key.
    /// The returned encryption key will be valid for as long as self.
    pub(crate) fn as_uplink_c(&self) -> *mut ulksys::UplinkEncryptionKey {
        self.inner.encryption_key
    }
}

impl Ensurer for ulksys::UplinkEncryptionKeyResult {
    fn ensure(&self) -> &Self {
        assert!(!self.encryption_key.is_null() || !self.error.is_null(), "underlying c-binding returned an invalid UplinkEncryptionKeyResult; encryption_key and error fields are both NULL");
        assert!((self.encryption_key.is_null() && !self.error.is_null())
            || (!self.encryption_key.is_null() && self.error.is_null()),
            "underlying c-binding returned an invalid UplinkEncryptionKeyResult; encryption_key and error fields are both NOT NULL");
        self
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

    use std::ptr;

    #[test]
    fn test_ensurer_ulksys_encryption_key_result_valid() {
        {
            // Has an encryption key.
            let enckey_res = ulksys::UplinkEncryptionKeyResult {
                encryption_key: &mut ulksys::UplinkEncryptionKey { _handle: 0 },
                error: ptr::null_mut::<ulksys::UplinkError>(),
            };

            enckey_res.ensure();
        }

        {
            // Has an error.
            let enckey_res = ulksys::UplinkEncryptionKeyResult {
                encryption_key: ptr::null_mut::<ulksys::UplinkEncryptionKey>(),
                error: &mut ulksys::UplinkError {
                    code: 0,
                    message: ptr::null_mut(),
                },
            };

            enckey_res.ensure();
        }
    }

    #[test]
    #[should_panic(
        expected = "underlying c-binding returned an invalid UplinkEncryptionKeyResult; encryption_key and error fields are both NULL"
    )]
    fn test_ensurer_ulksys_access_result_invalid_both_null() {
        let enckey_res = ulksys::UplinkEncryptionKeyResult {
            encryption_key: ptr::null_mut::<ulksys::UplinkEncryptionKey>(),
            error: ptr::null_mut::<ulksys::UplinkError>(),
        };

        enckey_res.ensure();
    }

    #[test]
    #[should_panic(
        expected = "underlying c-binding returned an invalid UplinkEncryptionKeyResult; encryption_key and error fields are both NOT NULL"
    )]
    fn test_ensurer_ulksys_access_result_invalid_both_not_null() {
        // Has an encryption key.
        let enckey_res = ulksys::UplinkEncryptionKeyResult {
            encryption_key: &mut ulksys::UplinkEncryptionKey { _handle: 0 },
            error: &mut ulksys::UplinkError {
                code: 0,
                message: ptr::null_mut(),
            },
        };

        enckey_res.ensure();
    }
}
