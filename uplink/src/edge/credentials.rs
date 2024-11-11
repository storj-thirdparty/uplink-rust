//! Storj DCS Edge services credentials.

use crate::uplink_c::Ensurer;
use crate::{Error, Result};

use std::ffi::CStr;

use uplink_sys as ulksys;

/// Contains the credentials for accessing to the multi-tenant gateways.
/// This works in compatible Amazon S3 clients.
#[derive(Debug, Clone)]
pub struct Gateway {
    /// The access key ID in base32 encoding. It's also used in the linksharing URL path.
    pub access_key_id: String,
    /// The secret key in base32 encoding.
    pub secret_key: String,
    /// The HTTP(S) URL of the S3 gateway.
    pub endpoint: String,
}

impl Gateway {
    pub(crate) fn from_ffi_credentials_result(
        uc_result: ulksys::EdgeCredentialsResult,
    ) -> Result<Self> {
        uc_result.ensure();

        if let Some(err) = Error::new_uplink(uc_result.error) {
            // SAFETY: we trust the FFI is safe freeing the memory of a valid pointer.
            unsafe { ulksys::edge_free_credentials_result(uc_result) };
            return Err(err);
        }

        let access_key_id: String;
        let secret_key: String;
        let endpoint: String;
        // SAFETY: we have checked that the `uc_result` isn't an error so `credentials` field isn't
        // NULL through the `ensure` method of the result. Inside of the block we check with the
        // credentials ensure method that their fields aren't NULL, so we are not accessing to any
        // NULL pointer.
        unsafe {
            // Likely these values shouldn't contain invalid UTF-8 characters, but we don't panic
            // if they contain some and we return an internal error because we see it's a limitation
            // of Rust and C interoperability and consumers of this crate would have a chance to
            // deal with them appropriately.
            let creds = *uc_result.credentials;
            creds.ensure();

            access_key_id = CStr::from_ptr(creds.access_key_id)
                .to_str()
                .map_err(|err| {
                    ulksys::edge_free_credentials_result(uc_result);
                    Error::new_internal(
                    "FFI returned an invalid access key ID; it contains invalid UTF-8 characters",
                    err.into(),
                    )
                })?
                .to_string();

            secret_key = CStr::from_ptr(creds.secret_key)
                .to_str()
                .map_err(|err| {
                    ulksys::edge_free_credentials_result(uc_result);
                    Error::new_internal(
                        "FFI returned an invalid secret key; it contains invalid UTF-8 characters",
                        err.into(),
                    )
                })?
                .to_string();

            endpoint = CStr::from_ptr(creds.endpoint)
                .to_str()
                .map_err(|err| {
                    ulksys::edge_free_credentials_result(uc_result);
                    Error::new_internal(
                        "FFI returned an invalid endpoint; it contains invalid UTF-8 characters",
                        err.into(),
                    )
                })?
                .to_string();

            ulksys::edge_free_credentials_result(uc_result);
        }

        Ok(Self {
            access_key_id,
            secret_key,
            endpoint,
        })
    }
}
