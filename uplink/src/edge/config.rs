//! Storj DCS Edge services configuration.

use crate::edge::credentials;
use crate::{access, helpers, Error, Result};

use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;

use uplink_sys as ulksys;

/// It contains the configuration parameters on how to access edge services.
pub struct Config {
    /// The host and port of the Auth service to use.
    ///
    /// Storj DCS Edge services valid addresses are `auth.[ap|eu|us].storjsahre.io:443`. This field
    /// can contain any third party hosted Auth service.
    // auth_service_addr: &'a str,
    /// The root certificate(s) or chain(s) against which Uplink checks the auth service.
    /// They must be in PEM format
    ///
    /// It's intended for testing against self-hosted Auth service or improving the security.
    // certificate_pem: Option<&'a [u8]>,
    inner: ulksys::EdgeConfig,
}

impl Config {
    /// Creates a new configuration that uses the specified Auth service address.
    ///
    /// Address must have host and port. It checks that it contains both parts and the port is a
    /// valid port.
    ///
    /// Storj DCS Edge services valid addresses are `auth.[ap|eu|us].storjsahre.io:443`. This field
    /// can contain any third party hosted Auth service.
    pub fn new(auth_service_addr: &str) -> Result<Self> {
        let parts: Vec<&str> = auth_service_addr.split(':').collect();
        if parts.len() != 2 {
            return Err(Error::new_invalid_arguments(
                "auth_service_addr",
                "invalid address, missing port or it contains more than one colon",
            ));
        }

        if let Err(err) = parts[1].parse::<u16>() {
            return Err(Error::new_invalid_arguments(
                "auth_service_addr",
                &format!("invalid port. {}", err),
            ));
        }

        let addr = helpers::cstring_from_str_fn_arg("auth_service_addr", auth_service_addr)?;
        Ok(Self {
            inner: ulksys::EdgeConfig {
                auth_service_address: addr.into_raw(),
                certificate_pem: ptr::null_mut(),
            },
        })

        // Ok(Self {
        //     auth_service_addr,
        //     certificate_pem: None,
        // })
    }

    /// Creates a new configuration that uses the specified Auth service address and the root
    /// certificate(s) or chain(s) in PEM formatagainst which Uplink checks the auth service.
    ///
    /// See [`new` constructor](Self::new) for knowing about the `auth_service_addr` format.
    ///
    /// `cert_pem` is used by Uplink for checking the Auth service. It's intended for testing
    /// against self-hosted Auth service or improving the security.
    ///
    /// The only verification that this constructor does with `cert_pem` is that it doesn't contain
    /// any null byte (0 byte) because it's a FFI requirement. Hence an invalid or malformed
    /// certificate won't return an error and the errors will be returned when using this
    /// configuration.
    pub fn with_certificate(auth_service_addr: &str, cert_pem: &[u8]) -> Result<Self> {
        let mut this = Self::new(auth_service_addr)?;
        match CString::new(cert_pem) {
            Err(e) => Err(Error::new_invalid_arguments(
                "cert_pem",
                &format!(
                    "cannot contains null bytes (0 byte). Null byte found at {}",
                    e.nul_position()
                ),
            )),
            Ok(cert) => {
                this.inner.certificate_pem = cert.into_raw();
                Ok(this)
            }
        }
    }

    /// Get the credentials for the Storj-hosted Gateway and linksharing (see
    /// [`crate::edge::linksharing::share_url`]) services.
    ///
    /// All the files accessible under the `access` are then also accessible via those services. If
    /// there is a need to call this function frequently, we recommend to limit the lifetime of the
    /// credentials by setting [`crate::access::Permission::set_not_after`] when creating the
    /// access grant if the use case doesn't have a specific constraint for not doing it.
    pub fn register_gateway_access(
        &self,
        access: access::Grant,
        opts: Option<&OptionsRegisterAccess>,
    ) -> Result<credentials::Gateway> {
        let uc_opts = if let Some(o) = opts {
            &o.as_ffi_options_register_access() as *const ulksys::EdgeRegisterAccessOptions
        } else {
            ptr::null()
        };

        // SAFETY: we trust the FFI is safe creating an instance of its own types and rely in our
        // implemented FFI methods to return valid FFI values with correct lifetimes.
        let uc_res = unsafe {
            ulksys::edge_register_access(
                self.inner,
                access.as_ffi_access(),
                uc_opts as *mut ulksys::EdgeRegisterAccessOptions,
            )
        };

        credentials::Gateway::from_ffi_credentials_result(uc_res)
    }
}

impl Drop for Config {
    fn drop(&mut self) {
        // SAFETY: Retake ownership of the `CString`(s) created when this instance was created.
        // The constructors create them and transfer their ownership to the FFI value hold by this
        // instance, whose guards its lifetime.
        // This is instance is dropped here so we retake the ownership to drop them and not leaking
        // memory.
        unsafe {
            drop(CString::from_raw(
                self.inner.auth_service_address as *mut c_char,
            ));
            if !self.inner.certificate_pem.is_null() {
                drop(CString::from_raw(self.inner.certificate_pem as *mut c_char));
            }
        };
    }
}

/// Contains the options parameters for access registration.
/// See [`Config::register_gateway_access` method](Config::register_gateway_access).
pub struct OptionsRegisterAccess {
    /// Determines whether objects can be read without authentication.
    pub public: bool,
}

impl OptionsRegisterAccess {
    /// Returns the FFI representation of register access options.
    pub(crate) fn as_ffi_options_register_access(&self) -> ulksys::EdgeRegisterAccessOptions {
        ulksys::EdgeRegisterAccessOptions {
            is_public: self.public,
        }
    }
}
