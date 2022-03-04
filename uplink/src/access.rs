//! Storj DCS Access Grant and bound types.

use crate::config::Config;
use crate::{helpers, EncryptionKey, Ensurer, Error, Result};

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;
use std::time::Duration;
use std::vec::Vec;

use uplink_sys as ulksys;

/// Represents an access grant
///
/// An access grant contains everything to access a project and specific buckets.
///
/// It includes a potentially-restricted API Key, a potentially-restricted set of encryption
/// information, and information about the Satellite responsible for the project's metadata.
#[derive(Debug)]
pub struct Grant {
    /// The access type of the underlying c-bindings than an instance of this struct represents and
    /// guards its life time until this instance drops.
    ///
    /// It's an access result because it's the one that holds the access grant and allows to free
    /// its memory.
    inner: ulksys::UplinkAccessResult,
}

impl Grant {
    /// Creates a new access grant from a serialized access grant string.
    pub fn new(serialized_access: &str) -> Result<Self> {
        let saccess = helpers::cstring_from_str_fn_arg("serialized_access", serialized_access)?;

        // SAFETY: we trust that the underlying c-binding is safe, nonetheless we ensure `accres` is
        // correct through the ensure method of the implemented Ensurer trait.
        let accres =
            unsafe { *ulksys::uplink_parse_access(saccess.as_ptr() as *mut c_char).ensure() };

        if let Some(e) = Error::new_uplink(accres.error) {
            drop_uplink_sys_access_result(accres);
            Err(e)
        } else {
            Ok(Grant { inner: accres })
        }
    }

    /// Generates a new access grant using a passphrase requesting to the satellite a project-based
    /// salt for deterministic key derivation.
    pub fn request_access_with_passphrase(
        satellite_addr: &str,
        api_key: &str,
        passphrase: &str,
    ) -> Result<Self> {
        let satellite_addr = helpers::cstring_from_str_fn_arg("satellite_addr", satellite_addr)?;
        let api_key = helpers::cstring_from_str_fn_arg("api_key", api_key)?;
        let passphrase = helpers::cstring_from_str_fn_arg("passphrase", passphrase)?;

        // SAFETY: we trust that the underlying c-binding is safe, nonetheless we ensure `accres`
        // is correct through the ensure method of the implemented Ensurer trait.
        let accres = unsafe {
            *ulksys::uplink_request_access_with_passphrase(
                satellite_addr.as_ptr() as *mut c_char,
                api_key.as_ptr() as *mut c_char,
                passphrase.as_ptr() as *mut c_char,
            )
            .ensure()
        };

        Error::new_uplink(accres.error).map_or(Ok(Grant { inner: accres }), |err| {
            drop_uplink_sys_access_result(accres);
            Err(err)
        })
    }

    /// Overrides the root encryption key for the prefix in bucket with the encryption key.
    ///
    /// This method is useful for overriding the encryption key in user-specific access grants when
    /// implementing multitenancy in a single app bucket.
    /// See relevant information in the general crate documentation.
    pub fn override_encryption_key(
        &self,
        bucket: &str,
        prefix: &str,
        encryption_key: &EncryptionKey,
    ) -> Result<()> {
        let bucket = helpers::cstring_from_str_fn_arg("bucket", bucket)?;
        let prefix = helpers::cstring_from_str_fn_arg("prefix", prefix)?;

        // SAFETY: we trust that the underlying c-binding is safe.
        let uerr = unsafe {
            ulksys::uplink_access_override_encryption_key(
                self.inner.access,
                bucket.as_ptr() as *mut c_char,
                prefix.as_ptr() as *mut c_char,
                encryption_key.as_uplink_c(),
            )
        };

        Error::new_uplink(uerr).map_or(Ok(()), |err| {
            drop_uplink_sys_error(uerr);
            Err(err)
        })
    }

    /// Returns the satellite node URL associated with this access grant.
    pub fn satellite_address(&self) -> Result<&str> {
        // SAFETY: we trust that the underlying c-binding is safe, nonetheless we ensure `strres` is
        // correct through the ensure method of the implemented Ensurer trait.
        let strres =
            unsafe { *ulksys::uplink_access_satellite_address(self.inner.access).ensure() };

        if let Some(e) = Error::new_uplink(strres.error) {
            drop_uplink_sys_string_result(strres);
            return Err(e);
        }

        let address;
        // SAFETY: at this point we have already checked that `strres`.string is NOT NULL.
        unsafe {
            address = CStr::from_ptr(strres.string).to_str();
            drop_uplink_sys_string_result(strres)
        };

        Ok(address.expect("invalid underlying c-binding"))
    }

    /// Serializes an access grant such that it can be used to create a [`Self::new()`] instance of
    /// this type or parsed with other tools.
    pub fn serialize(&self) -> Result<&str> {
        // SAFETY: we trust that the underlying c-binding is safe, nonetheless we ensure `strres` is
        // correct through the ensure method of the implemented Ensurer trait.
        let strres = unsafe { *ulksys::uplink_access_serialize(self.inner.access).ensure() };

        if let Some(e) = Error::new_uplink(strres.error) {
            drop_uplink_sys_string_result(strres);
            return Err(e);
        }

        let serialized;
        // SAFETY: at this point we have already checked that strres.string is NOT NULL.
        unsafe {
            serialized = CStr::from_ptr(strres.string).to_str();
            drop_uplink_sys_string_result(strres);
        }

        Ok(serialized.expect("invalid underlying c-binding"))
    }

    /// Creates a new access grant with specific permissions.
    ///
    /// An access grant can only have their existing permissions restricted, and the resulting
    /// access will only allow for the intersection of all previous share calls in the access
    /// construction chain.
    ///
    /// Prefixes restrict the access grant (and internal encryption information) to only contain
    /// enough information to allow access to just those prefixes.
    ///
    /// To revoke an access grant see [`Project.revoke_access()`](../project/struct.Project.html#method.revoke_access).
    pub fn share(&self, permission: &Permission, prefixes: Vec<SharePrefix>) -> Result<Grant> {
        let mut ulk_prefixes: Vec<ulksys::UplinkSharePrefix> = Vec::with_capacity(prefixes.len());

        for sp in prefixes {
            ulk_prefixes.push(sp.as_uplink_c())
        }

        // SAFETY: we trust that the underlying c-binding is safe, nonetheless we ensure `accres` is
        // correct through the ensure method of the implemented Ensurer trait.
        let accres = unsafe {
            *ulksys::uplink_access_share(
                self.inner.access,
                permission.to_uplink_c(),
                ulk_prefixes.as_mut_ptr(),
                ulk_prefixes.len() as i64,
            )
            .ensure()
        };

        Error::new_uplink(accres.error).map_or(Ok(Grant { inner: accres }), |err| {
            drop_uplink_sys_access_result(accres);
            Err(err)
        })
    }

    /// Generates a new access grant using the configuration and the specific satellite address, API
    /// key, and passphrase.
    /// It connects to the satellite address for getting a project-based salt for deterministic key
    /// derivation.
    ///
    /// NOTE: this is a CPU-heavy operation that uses a password-based key derivation (Argon2). It
    /// should be a setup-only step. Most common interactions with the library should be using a
    /// serialized access grant through [`Grant::new()`](../access/struct.Grant.html#.method.new).
    pub(crate) fn request_access_with_config_and_passphrase(
        config: &Config,
        satellite_addr: &str,
        api_key: &str,
        passphrase: &str,
    ) -> Result<Self> {
        let satellite_addr = helpers::cstring_from_str_fn_arg("satellite_addr", satellite_addr)?;
        let api_key = helpers::cstring_from_str_fn_arg("api_key", api_key)?;
        let passphrase = helpers::cstring_from_str_fn_arg("passphrase", passphrase)?;

        // SAFETY: we trust that the underlying c-binding is safe, nonetheless we ensure `accres` is
        // correct through the ensure method of the implemented Ensurer trait.
        let accres = unsafe {
            *ulksys::uplink_config_request_access_with_passphrase(
                config.as_uplink_c(),
                satellite_addr.as_ptr() as *mut c_char,
                api_key.as_ptr() as *mut c_char,
                passphrase.as_ptr() as *mut c_char,
            )
            .ensure()
        };

        if let Some(e) = Error::new_uplink(accres.error) {
            drop_uplink_sys_access_result(accres);
            Err(e)
        } else {
            Ok(Grant {
                inner: ulksys::UplinkAccessResult {
                    access: accres.access,
                    error: ptr::null_mut(),
                },
            })
        }
    }
}

impl Drop for Grant {
    fn drop(&mut self) {
        drop_uplink_sys_access_result(self.inner);
    }
}

/// Represents a prefix to be shared.
#[derive(Debug)]
pub struct SharePrefix<'a> {
    bucket: &'a str,
    c_bucket: CString,
    prefix: &'a str,
    c_prefix: CString,
}

impl<'a> SharePrefix<'a> {
    /// Create a new prefix to be shared in the specified bucket.
    /// It returns an error if bucket or prefix contains a null character (0 byte).
    pub fn new(bucket: &'a str, prefix: &'a str) -> Result<Self> {
        let c_bucket = helpers::cstring_from_str_fn_arg("bucket", bucket)?;
        let c_prefix = helpers::cstring_from_str_fn_arg("prefix", prefix)?;

        Ok(SharePrefix {
            bucket,
            c_bucket,
            prefix,
            c_prefix,
        })
    }

    /// Returns the bucket where the prefix to be shared belongs.
    pub fn bucket(&self) -> &str {
        self.bucket
    }

    /// Returns the actual prefix to be shared.
    pub fn prefix(&self) -> &str {
        self.prefix
    }

    /// Returns an `UplinkSharePrefix` with the values of this `SharedPrefix` for interoperating
    /// with the uplink c-bindings. The pointer fields of the returned struct will be valid as long
    /// as `self` is.
    fn as_uplink_c(&self) -> ulksys::UplinkSharePrefix {
        ulksys::UplinkSharePrefix {
            bucket: self.c_bucket.as_ptr(),
            prefix: self.c_prefix.as_ptr(),
        }
    }
}

/// Defines what actions and an optional specific period of time are granted to a shared access
/// grant.
///
/// A shared access grant can never has more permission that its parent, hence even some allowed
/// permission is set for the shared access Grant but not to its parent, the shared access Grant
/// won't be allowed. shared access Grant wont See
/// [`Grant.share()`](struct.Grant.html#method.share).
#[derive(Default)]
pub struct Permission {
    /// Gives permission to download the content of the objects and their associated metadata, but
    /// it does not allow listing buckets.
    pub allow_download: bool,
    /// Gives permission to create buckets and upload new objects. It does not allow overwriting
    /// existing objects unless allow_delete is granted too.
    pub allow_upload: bool,
    /// Gives permission to list buckets and getting the metadata of the objects. It does not allow
    /// downloading the content of the objects.
    pub allow_list: bool,
    /// Gives permission to delete buckets and objects. Unless either allow `allow_download` or
    /// `allow_list` is grated too, neither the metadata of the objects nor error information will
    /// be returned for deleted objects.
    pub allow_delete: bool,
    /// Restricts when the resulting access grant is valid for. If it is set then it must always be
    /// before not_after and the resulting access grant will not work if the satellite believes the
    /// time is before the set it  one.
    ///
    /// The time is measured with the number of seconds since the Unix Epoch time.
    not_before: Option<Duration>,
    /// Restricts when the resulting access grant is valid for. If it is set then it must always be
    /// after not_before and the resulting access grant will not work if the satellite believes the
    /// time is after the set it one.
    ///
    /// The time is measured with the number of seconds since the Unix Epoch
    /// time.
    not_after: Option<Duration>,
}

impl Permission {
    /// Creates a permission that doesn't allow any operation, which is the default permission.
    /// This constructor is useful for creating a permission for after setting the specific allowed
    /// operations when none of the other constructors creates a permission with a set of allowed
    /// operations that works for your use case.
    pub fn new() -> Permission {
        Permission {
            ..Default::default()
        }
    }

    /// Creates a permission that allows all the operations (i.e. Downloading, uploading, listing
    /// and deleting).
    pub fn full() -> Permission {
        Permission {
            allow_download: true,
            allow_upload: true,
            allow_list: true,
            allow_delete: true,
            not_before: None,
            not_after: None,
        }
    }

    /// Creates a permission that allows for reading (i.e. Downloading) and  listing.
    pub fn read_only() -> Permission {
        Permission {
            allow_download: true,
            allow_upload: false,
            allow_list: true,
            allow_delete: false,
            not_before: None,
            not_after: None,
        }
    }

    /// Creates a permission that allows for writing (i.e. Uploading) and deleting.
    pub fn write_only() -> Permission {
        Permission {
            allow_download: false,
            allow_upload: true,
            allow_list: false,
            allow_delete: true,
            not_before: None,
            not_after: None,
        }
    }

    /// Returns the duration from Unix Epoch time since this permission is valid.
    /// Return `None` when there is not before restriction.
    pub fn not_before(&self) -> Option<Duration> {
        self.not_before
    }

    /// Sets a not before valid time for this permission or removing it when `None` is passed.
    /// An error is returned if since is more recent or equal to the current not after valid time of
    /// the permission, when not after is set. The time is measured with the number of seconds since
    /// the Unix Epoch time.
    pub fn set_not_before(&mut self, since: Option<Duration>) -> Result<()> {
        if let Some(since) = since {
            if let Some(until) = self.not_after {
                if since >= until {
                    return Err(
                        Error::new_invalid_arguments(
                            "since",
                            "cannot be more recent or equal to the not after valid time of the permission",
                        ));
                }
            }
        }

        self.not_before = since;
        Ok(())
    }

    /// Returns the duration from Unix Epoch time until this permission is valid.
    /// Return `None` when there is not after restriction.
    pub fn not_after(&self) -> Option<Duration> {
        self.not_after
    }

    /// Sets a not after valid time for this permission or removing it when `None` is passed.
    ///
    /// An error is returned if until is previous or equal to the current not before valid time of
    /// the permission, when not before is set.
    ///
    /// The time is measured with the number of seconds since the Unix Epoch time.
    pub fn set_not_after(&mut self, until: Option<Duration>) -> Result<()> {
        if let Some(until) = until {
            if let Some(since) = self.not_before {
                if until <= since {
                    return Err(
                        Error::new_invalid_arguments(
                            "until",
                            "cannot be previous or equal to the not before valid time of the permission",
                        ));
                }
            }
        }

        self.not_after = until;
        Ok(())
    }

    /// Returns an UplinkPermission with the values of this Permission for interoperating with the
    /// uplink c-bindings.
    fn to_uplink_c(&self) -> ulksys::UplinkPermission {
        ulksys::UplinkPermission {
            allow_download: self.allow_download,
            allow_upload: self.allow_upload,
            allow_list: self.allow_list,
            allow_delete: self.allow_delete,
            not_before: self.not_before.map_or(0, |d| d.as_secs()) as i64,
            not_after: self.not_after.map_or(0, |d| d.as_secs()) as i64,
        }
    }
}

impl Ensurer for ulksys::UplinkAccessResult {
    fn ensure(&self) -> &Self {
        assert!(!self.access.is_null() || !self.error.is_null(), "underlying c-binding returned an invalid UplinkAccessResult; access and error fields are both NULL");
        assert!((self.access.is_null() && !self.error.is_null())
            || (!self.access.is_null() && self.error.is_null()),
            "underlying c-binding returned an invalid UplinkAccessResult; access and error fields are both NOT NULL");
        self
    }
}

impl Ensurer for ulksys::UplinkStringResult {
    fn ensure(&self) -> &Self {
        assert!(!self.string.is_null() || !self.error.is_null(), "underlying c-binding returned an invalid UplinkStringResult; string and error fields are both NULL");
        assert!((self.string.is_null() && !self.error.is_null())
            || (!self.string.is_null() && self.error.is_null())
            , "underlying c-binding returned an invalid UplinkStringResult; string and error fields are both NOT NULL");
        self
    }
}

/// Calls the associated `free` underlying c-bindings function for releasing the associated
/// resources of an access result.
fn drop_uplink_sys_access_result(accres: ulksys::UplinkAccessResult) {
    // SAFETY: we trust that the underlying c-binding is safe freeing the
    // memory of a correct UplinkAccessResult value.
    unsafe {
        ulksys::uplink_free_access_result(accres);
    }
}

/// Calls, only if `error` is not null,  the associated `free` underlying c-bindings function for
/// releasing the associated resources with `error` and to free the memory pointed by it.
fn drop_uplink_sys_error(error: *mut ulksys::UplinkError) {
    if !error.is_null() {
        // SAFETY: We just checked that the pointer is not null and we trust
        // that the underlying c-binding is safe freeing its associated
        // resources and itself.
        unsafe {
            ulksys::uplink_free_error(error);
        }
    }
}

/// Calls the associated `free` underlying c-bindings function for releasing the associated
/// resources of a string result.
fn drop_uplink_sys_string_result(strres: ulksys::UplinkStringResult) {
    // SAFETY: we trust that the underlying c-binding is safe freeing the
    // memory of a correct UplinkStringResult value.
    unsafe {
        ulksys::uplink_free_string_result(strres);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::error;

    /*** Grant tests ***/
    #[test]
    fn test_grant_new_invalid_param() {
        if let Error::InvalidArguments(error::Args { names, msg }) = Grant::new("serialized\0")
            .expect_err("when passing an serialized access grant with NULL bytes")
        {
            assert_eq!(names, "serialized_access", "invalid error argument name");
            assert_eq!(
                msg, "cannot contains null bytes (0 byte). Null byte found at 10",
                "invalid error argument message"
            );
        } else {
            panic!("expected an invalid argument error");
        }
    }

    #[test]
    fn test_grant_request_access_with_passphrase_invalid_params() {
        {
            // Invalid satelite address.
            if let Error::InvalidArguments(error::Args { names, msg }) =
                Grant::request_access_with_passphrase("localh\0st", "some-key", "pass")
                    .expect_err("when passing an satellite address with NULL bytes")
            {
                assert_eq!(names, "satellite_addr", "invalid error argument name");
                assert_eq!(
                    msg, "cannot contains null bytes (0 byte). Null byte found at 6",
                    "invalid error argument message"
                );
            } else {
                panic!("expected an invalid argument error");
            }
        }

        {
            // Invalid API Key.
            if let Error::InvalidArguments(error::Args { names, msg }) =
                Grant::request_access_with_passphrase("localhost", "s\0me-key", "pass")
                    .expect_err("when passing an API key with NULL bytes")
            {
                assert_eq!(names, "api_key", "invalid error argument name");
                assert_eq!(
                    msg, "cannot contains null bytes (0 byte). Null byte found at 1",
                    "invalid error argument message"
                );
            } else {
                panic!("expected an invalid argument error");
            }
        }

        {
            // Invalid passphrase.
            if let Error::InvalidArguments(error::Args { names, msg }) =
                Grant::request_access_with_passphrase("localhost", "some-key", "pass\0")
                    .expect_err("when passing an passphrase with NULL bytes")
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

    #[test]
    fn test_grant_request_access_with_config_and_passphrase_invalid_params() {
        {
            // Invalid satelite address.
            let config = Config::new("rust-bindings", Duration::new(1, 0), None)
                .expect("new shouldn't fail when 'user agent' doesn't contain ny nul character");

            if let Error::InvalidArguments(error::Args { names, msg }) =
                Grant::request_access_with_config_and_passphrase(
                    &config,
                    "localh\0st",
                    "some-key",
                    "pass",
                )
                .expect_err("when passing an satellite address with NULL bytes")
            {
                assert_eq!(names, "satellite_addr", "invalid error argument name");
                assert_eq!(
                    msg, "cannot contains null bytes (0 byte). Null byte found at 6",
                    "invalid error argument message"
                );
            } else {
                panic!("expected an invalid argument error");
            }
        }

        {
            // Invalid API Key.
            let config = Config::new("rust-bindings", Duration::new(1, 0), None)
                .expect("new shouldn't fail when 'user agent' doesn't contain ny nul character");
            if let Error::InvalidArguments(error::Args { names, msg }) =
                Grant::request_access_with_config_and_passphrase(
                    &config,
                    "localhost",
                    "s\0me-key",
                    "pass",
                )
                .expect_err("when passing an API key with NULL bytes")
            {
                assert_eq!(names, "api_key", "invalid error argument name");
                assert_eq!(
                    msg, "cannot contains null bytes (0 byte). Null byte found at 1",
                    "invalid error argument message"
                );
            } else {
                panic!("expected an invalid argument error");
            }
        }

        {
            // Invalid passphrase.
            let config = Config::new("rust-bindings", Duration::new(1, 0), None)
                .expect("new shouldn't fail when 'user agent' doesn't contain ny nul character");

            if let Error::InvalidArguments(error::Args { names, msg }) =
                Grant::request_access_with_config_and_passphrase(
                    &config,
                    "localhost",
                    "some-key",
                    "pass\0",
                )
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

    #[test]
    fn test_grant_override_encryption_key_invalid_params() {
        // TODO: uncomment this test when the implementation of the
        // EncryptionKey exists.
        /*
        let grant = Grant::new("serialized");
        let enc_key = EncryptionKey::new();

        {
            // Invalid bucket.
            if let Error::InvalidArguments(error::Args { names, msg }) = grant
                .override_encryption_key("\0a-bucket", "prefix", enc_key)
                .expect_err("when passing a bucket name with NULL bytes")
            {
                assert_eq!(names, "bucket", "invalid error argument name");
                assert_eq!(
                    msg, "cannot contains null bytes (0 byte). Null byte found at 0",
                    "invalid error argument message"
                );
            } else {
                panic!("expected an invalid argument error");
            }
        }

        {
            // Invalid prefix.
            if let Error::InvalidArguments(error::Args { names, msg }) = grant
                .override_encryption_key("a-bucket", "pre\0fix", enc_key)
                .expect_err("when passing a bucket name with NULL bytes")
            {
                assert_eq!(names, "prefix", "invalid error argument name");
                assert_eq!(
                    msg, "cannot contains null bytes (0 byte). Null byte found at 3",
                    "invalid error argument message"
                );
            } else {
                panic!("expected an invalid argument error");
            }
        }
        */
    }

    /*** SharePrefix tests ***/
    #[test]
    fn test_share_prefix() {
        {
            // Pass a valid bucket and prefix.
            let sp = SharePrefix::new("a-bucket", "a/b/c")
                .expect("new shouldn't fail when passing a valid bucket and prefix");
            assert_eq!(sp.bucket(), "a-bucket", "bucket");
            assert_eq!(sp.prefix(), "a/b/c", "prefix");
        }

        {
            // Pass an invalid bucket.
            if let Error::InvalidArguments(error::Args { names, msg }) =
                SharePrefix::new("a\0bucket\0", "a/b/c")
                    .expect_err("new passing a bucket with NULL bytes")
            {
                assert_eq!(names, "bucket", "invalid error argument name");
                assert_eq!(
                    msg, "cannot contains null bytes (0 byte). Null byte found at 1",
                    "invalid error argument message"
                );
            } else {
                panic!("expected an invalid argument error");
            }
        }

        {
            // Pass an invalid prefix.
            if let Error::InvalidArguments(error::Args { names, msg }) =
                SharePrefix::new("a-bucket", "a/b\0/c")
                    .expect_err("new passing a prefix with NULL bytes")
            {
                assert_eq!(names, "prefix", "invalid error argument name");
                assert_eq!(
                    msg, "cannot contains null bytes (0 byte). Null byte found at 3",
                    "invalid error argument message"
                );
            } else {
                panic!("expected an invalid argument error");
            }
        }

        {
            // Pass an invalid bucket and prefix.
            if let Error::InvalidArguments(error::Args { names, msg }) =
                SharePrefix::new("a\0bucket", "a/b\0/c")
                    .expect_err("new passing a bucket and prefix with NULL bytes")
            {
                assert_eq!(names, "bucket", "invalid error argument name");
                assert_eq!(
                    msg, "cannot contains null bytes (0 byte). Null byte found at 1",
                    "invalid error argument message"
                );
            } else {
                panic!("expected an invalid argument error");
            }
        }
    }

    /*** Permission tests ***/
    #[test]
    fn test_permission_default() {
        let perm = Permission::new();

        assert!(!perm.allow_download, "allow download");
        assert!(!perm.allow_upload, "allow upload");
        assert!(!perm.allow_list, "allow list");
        assert!(!perm.allow_delete, "allow delete");
        assert_eq!(perm.not_before(), None, "not before");
        assert_eq!(perm.not_after(), None, "not after");
    }

    #[test]
    fn test_permission_full() {
        let perm = Permission::full();

        assert!(perm.allow_download, "allow download");
        assert!(perm.allow_upload, "allow upload");
        assert!(perm.allow_list, "allow list");
        assert!(perm.allow_delete, "allow delete");
        assert_eq!(perm.not_before(), None, "not before");
        assert_eq!(perm.not_after(), None, "not after");
    }

    #[test]
    fn test_permission_read_only() {
        let perm = Permission::read_only();

        assert!(perm.allow_download, "allow download");
        assert!(!perm.allow_upload, "allow upload");
        assert!(perm.allow_list, "allow list");
        assert!(!perm.allow_delete, "allow delete");
        assert_eq!(perm.not_before(), None, "not before");
        assert_eq!(perm.not_after(), None, "not after");
    }

    #[test]
    fn test_permission_write_only() {
        let perm = Permission::write_only();

        assert!(!perm.allow_download, "allow download");
        assert!(perm.allow_upload, "allow upload");
        assert!(!perm.allow_list, "allow list");
        assert!(perm.allow_delete, "allow delete");
        assert_eq!(perm.not_before(), None, "not before");
        assert_eq!(perm.not_after(), None, "not after");
    }

    #[test]
    fn test_permission_time_boundaries() {
        let mut perm = Permission::full();

        assert_eq!(perm.not_before(), None, "not before");
        assert_eq!(perm.not_after(), None, "not after");

        // set not before and after without violating their constraints.
        {
            perm.set_not_before(Some(Duration::new(5, 50)))
                .expect("set not before");
            assert_eq!(
                perm.not_before(),
                Some(Duration::new(5, 50)),
                "set not before"
            );

            perm.set_not_after(Some(Duration::new(5, 51)))
                .expect("set not after");
            assert_eq!(
                perm.not_after(),
                Some(Duration::new(5, 51)),
                "set not after"
            );
        }

        // set not before violating its constraints.
        {
            if let Error::InvalidArguments(error::Args { names, msg }) = perm
                .set_not_before(Some(Duration::new(5, 52)))
                .expect_err("set not before")
            {
                assert_eq!(names, "since", "invalid error argument name");
                assert_eq!(
                    msg,
                    "cannot be more recent or equal to the not after valid time of the permission",
                    "invalid error argument message"
                );
            } else {
                panic!("expected an invalid argument error");
            }
        }

        // set not after violating its constraints.
        {
            if let Error::InvalidArguments(error::Args { names, msg }) = perm
                .set_not_after(Some(Duration::new(5, 50)))
                .expect_err("set not after")
            {
                assert_eq!(names, "until", "invalid error argument name");
                assert_eq!(
                    msg,
                    "cannot be previous or equal to the not before valid time of the permission",
                    "invalid error argument message"
                );
            } else {
                panic!("expected an invalid argument error");
            }
        }

        // removing not before and after
        {
            perm.set_not_before(None).expect("set not before");
            assert_eq!(perm.not_before(), None, "removing not before");

            perm.set_not_after(None).expect("set not after");
            assert_eq!(perm.not_after(), None, "removing not after");
        }
    }

    /*** Ensurer implementations tests ***/
    #[test]
    fn test_ensurer_ulksys_access_result_valid() {
        {
            // Has an access.
            let acc_res = ulksys::UplinkAccessResult {
                access: &mut ulksys::UplinkAccess { _handle: 0 },
                error: ptr::null_mut::<ulksys::UplinkError>(),
            };

            acc_res.ensure();
        }

        {
            // Has an error.
            let acc_res = ulksys::UplinkAccessResult {
                access: ptr::null_mut::<ulksys::UplinkAccess>(),
                error: &mut ulksys::UplinkError {
                    code: 0,
                    message: ptr::null_mut(),
                },
            };

            acc_res.ensure();
        }
    }

    #[test]
    #[should_panic(
        expected = "underlying c-binding returned an invalid UplinkAccessResult; access and error fields are both NULL"
    )]
    fn test_ensurer_ulksys_access_result_invalid_both_null() {
        let acc_res = ulksys::UplinkAccessResult {
            access: ptr::null_mut::<ulksys::UplinkAccess>(),
            error: ptr::null_mut::<ulksys::UplinkError>(),
        };

        acc_res.ensure();
    }

    #[test]
    #[should_panic(
        expected = "underlying c-binding returned an invalid UplinkAccessResult; access and error fields are both NOT NULL"
    )]
    fn test_ensurer_ulksys_access_result_invalid_both_not_null() {
        let acc_res = ulksys::UplinkAccessResult {
            access: &mut ulksys::UplinkAccess { _handle: 0 },
            error: &mut ulksys::UplinkError {
                code: 0,
                message: ptr::null_mut(),
            },
        };

        acc_res.ensure();
    }

    #[test]
    fn test_ensurer_ulksys_string_result_valid() {
        {
            // Has a string.
            let str_res = ulksys::UplinkStringResult {
                string: CString::new("whatever").unwrap().into_raw(),
                error: ptr::null_mut::<ulksys::UplinkError>(),
            };

            str_res.ensure();
        }

        {
            // Has an error.
            let str_res = ulksys::UplinkStringResult {
                string: ptr::null_mut(),
                error: &mut ulksys::UplinkError {
                    code: 0,
                    message: ptr::null_mut(),
                },
            };

            str_res.ensure();
        }
    }

    #[test]
    #[should_panic(
        expected = "underlying c-binding returned an invalid UplinkStringResult; string and error fields are both NULL"
    )]
    fn test_ensurer_ulksys_string_result_invalid_both_null() {
        let str_res = ulksys::UplinkStringResult {
            string: ptr::null_mut(),
            error: ptr::null_mut::<ulksys::UplinkError>(),
        };

        str_res.ensure();
    }

    #[test]
    #[should_panic(
        expected = "underlying c-binding returned an invalid UplinkStringResult; string and error fields are both NOT NULL"
    )]
    fn test_ensurer_ulksys_string_result_invalid_both_not_null() {
        let str_res = ulksys::UplinkStringResult {
            string: CString::new("whatever").unwrap().into_raw(),
            error: &mut ulksys::UplinkError {
                code: 0,
                message: ptr::null_mut(),
            },
        };

        str_res.ensure();
    }
}
