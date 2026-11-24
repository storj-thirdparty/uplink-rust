//! Storj DCS Edge services credentials.

use crate::guard::Guard;
use crate::uplink_c::Ensurer;
use crate::{Error, Result};

use std::ffi::CStr;
use std::os::raw::c_char;

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
    /// Creates a new instance from the FFI representation for a credentials' result.
    ///
    /// It returns the following errors:
    /// * an [`Error::Uplink`] if `uc_result` contains an error.
    /// * an [`Error::Internal`] if any of the credentials contains invalid UTF-8 characters.
    pub(crate) fn from_ffi_credentials_result(
        uc_result: Guard<ulksys::EdgeCredentialsResult>,
    ) -> Result<Self> {
        uc_result.ensure();
        uc_result.check()?;

        let creds = uc_result
            .value()
            .expect("successful credentials result must always have credentials");
        creds.ensure();

        // Likely these values shouldn't contain invalid UTF-8 characters, but we don't panic if
        // they contain some and we return an internal error because we see it's a limitation of
        // Rust and C interoperability and consumers of this crate would have a chance to deal with
        // them appropriately.
        let to_string = |c_str: *const c_char, err_msg| {
            // SAFETY: we have checked that the fields of the credentials aren't NULL through their
            // `ensure` method.
            let c_str = unsafe { CStr::from_ptr(c_str) };
            c_str
                .to_str()
                .map(String::from)
                .map_err(|err| Error::new_internal(err_msg, err.into()))
        };

        Ok(Self {
            access_key_id: to_string(
                creds.access_key_id,
                "FFI returned an invalid access key ID; it contains invalid UTF-8 characters",
            )?,
            secret_key: to_string(
                creds.secret_key,
                "FFI returned an invalid secret key; it contains invalid UTF-8 characters",
            )?,
            endpoint: to_string(
                creds.endpoint,
                "FFI returned an invalid endpoint; it contains invalid UTF-8 characters",
            )?,
        })
    }
}
