//! Storj DCS Object and related types.

pub mod upload;

pub use upload::Upload;

use crate::error::BoxError;
use crate::guard::Guard;
use crate::uplink_c::Ensurer;
use crate::{error, metadata, Error, Result};

use std::ffi::{CStr, CString};

use uplink_sys as ulksys;

/// Contains information about an object.
#[derive(Debug)]
pub struct Object {
    /// The identifier of the object inside of the bucket which it belongs.
    pub key: String,
    /// Indicates if the key is a prefix for other objects.
    pub is_prefix: bool,
    /// The system metadata associated with the object.
    pub metadata_system: metadata::System,
    /// The custom metadata associated with the object.
    pub metadata_custom: metadata::Custom,
}

impl Object {
    /// Creates new instance from the FFI representation.
    ///
    /// It returns an [`Error::Internal`](crate::Error::Internal) if `uc_obj`'s key contains invalid
    /// UTF-8 characters.
    fn with_ffi_object(uc_obj: &ulksys::UplinkObject) -> Result<Self> {
        uc_obj.ensure();

        // SAFETY: we have check that the `uc_obj` doesn't have fields with NULL pointers through
        // the `ensure` method.
        let key = CString::from(unsafe { CStr::from_ptr(uc_obj.key) });
        let key = key.into_string().map_err(|err| {
            Error::new_internal(
                "FFI returned an invalid object's key; it contains invalid UTF-8 characters",
                BoxError::from(err),
            )
        })?;

        Ok(Self {
            key,
            is_prefix: uc_obj.is_prefix,
            metadata_system: metadata::System::with_ffi_system_metadata(&uc_obj.system),
            metadata_custom: metadata::Custom::with_ffi_custom_metadata(&uc_obj.custom),
        })
    }

    /// Creates a new instance from the FFI representation for an object's result.
    ///
    /// When there isn't an error, it returns `None` if the result doesn't have an object. This
    /// happens in some specific situations.
    ///
    /// It returns the following errors:
    /// * an [`Error::Uplink`](crate::Error::Uplink) if `uc_result` contains an error.
    /// * an [`Error::Internal`](crate::Error::Internal) if `uc_result.object`'s key contains
    ///   invalid UTF-8 characters.
    pub(crate) fn from_ffi_object_result(
        uc_result: Guard<ulksys::UplinkObjectResult>,
    ) -> Result<Option<Self>> {
        uc_result.check()?;

        uc_result.value().map(Self::with_ffi_object).transpose()
    }

    /// Creates a new instance from the FFI representation for a commit upload's result.
    ///
    /// It returns the following errors:
    /// * an [`Error::Uplink`](crate::Error::Uplink) if `uc_result` contains an error.
    /// * an [`Error::Internal`](crate::Error::Internal) if `uc_result.object`'s key contains
    ///   invalid UTF-8 characters.
    pub(crate) fn from_ffi_commit_upload_result(
        uc_result: Guard<ulksys::UplinkCommitUploadResult>,
    ) -> Result<Self> {
        uc_result.ensure();
        uc_result.check()?;

        Self::with_ffi_object(
            uc_result
                .value()
                .expect("successful committed upload must always return an object"),
        )
    }
}

/// Iterates over a collection of objects' information.
#[derive(Debug)]
pub struct Iterator {
    /// The object iterator type of the FFI that an instance of this struct represents and guards
    /// its lifetime until the instance drops.
    inner: Guard<*mut ulksys::UplinkObjectIterator>,
}

impl Iterator {
    /// Creates a new instance from the type exposed by the FFI.
    pub(crate) fn from_ffi_object_iterator(
        uc_iterator: Guard<*mut ulksys::UplinkObjectIterator>,
    ) -> Self {
        assert!(
            uc_iterator.value().is_some(),
            "BUG: `uc_iterator` argument cannot be NULL"
        );

        Iterator { inner: uc_iterator }
    }
}

impl std::iter::Iterator for Iterator {
    type Item = Result<Object>;

    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: we trust that the FFI functions don't panic when called with an instance returned
        // by them and they don't return any invalid memory references or `null` if next returns
        // `true`.
        unsafe {
            if !ulksys::uplink_object_iterator_next(*self.inner) {
                let uc_error = Guard::new(ulksys::uplink_object_iterator_err(*self.inner));
                return uc_error.error().map(Err);
            }

            let uc_obj = Guard::new(ulksys::uplink_object_iterator_item(*self.inner));
            Some(Object::with_ffi_object(uc_obj.value().expect(
                "an iterator that indicated that there is a next element always returns it",
            )))
        }
    }
}

/// Represents a download object operation from Storj DCS network.
#[derive(Debug)]
pub struct Download {
    /// The download type of the FFI than an instance of this struct represents and guards its
    /// lifetime until the instance drops.
    ///
    /// It's an access result
    inner: Guard<ulksys::UplinkDownloadResult>,
}

impl Download {
    /// Creates a new instance from the FFI representation.
    ///
    /// It returns an [`Error::Uplink`](crate::Error::Uplink) if `uc_result` contains an error.
    pub(crate) fn from_ffi_download_result(
        uc_result: Guard<ulksys::UplinkDownloadResult>,
    ) -> Result<Self> {
        uc_result.ensure();
        uc_result.check()?;

        Ok(Self { inner: uc_result })
    }

    /// Returns the last information about the object.
    ///
    /// It returns if FFI returns an error when retrieving the information.
    pub fn info(&self) -> Result<Object> {
        // SAFETY: We trust the FFI is behaving correctly when passing a valid `UplinkDownload`
        // instance.
        let obj_res = unsafe { Guard::new(ulksys::uplink_download_info(self.inner.download)) };

        Object::from_ffi_object_result(obj_res)
            .map(|op| op.expect("successful download object info must always return an object"))
    }
}

impl std::io::Read for Download {
    /// Downloads the object's data stream into `buf` and return the number of downloaded bytes,
    /// which are at most the `buf` length, when there isn't any error.
    ///
    /// When it returns an error is always a [`std::io::ErrorKind::Other`] and the error payload is
    /// an [`Error::Uplink`].
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // Retry in case that zero bytes are read but no error is returned. We retry 3 times for
        // being safe of not looping infinitely despite 1 retry should always be enough.
        // See Uplink issue: https://github.com/storj/uplink/issues/99.
        for _ in 0..=3 {
            let bp = buf.as_mut_ptr();
            // SAFETY: we trust the FFI of dealing with a correct `UplinkDownload` instance and an
            // allocated buffer.
            let read_res = unsafe {
                Guard::new(ulksys::uplink_download_read(
                    self.inner.download,
                    bp.cast(),
                    buf.len(),
                ))
            };

            if let Some(err) = read_res.error() {
                // According to the Uplink C bindings version that we are targeting v1.7.0 all the
                // errors are mapped to an specific exported code, except EOF, see
                // https://github.com/storj/uplink-c/blob/v1.7.0/error.go#L37
                // The `ulksys::uplink_download_read` always use the error mapping function
                // (`mallocError`), hence, we can assume that an unknown error is the EOF error.
                //
                // Although EOF is usually -1 it's platform-dependent of the C standard library, so
                // it looks safer an better to compare with 'Unknown' variant than relying in -1
                // comparison or adding libc as a direct dependency of this crate.
                if let Error::Uplink(error::Uplink::Unknown(_)) = err {
                    return Ok(read_res.bytes_read as usize);
                }

                use std::io::Error as IOErr;
                return Err(IOErr::other(err));
            }

            if read_res.bytes_read != 0 {
                return Ok(read_res.bytes_read as usize);
            }
        }

        Ok(0)
    }
}

impl Drop for Download {
    fn drop(&mut self) {
        // SAFETY: we trust that the FFI is doing correct operations when closing a correctly created
        // `UplinkDownloadResult` value, which is freed when `self.inner` is dropped.
        unsafe {
            // At this point we cannot do anything about the error, so discarded.
            // TODO(https://github.com/storj-thirdparty/uplink-rust/issues/51).
            drop(Guard::new(ulksys::uplink_close_download(
                self.inner.download,
            )));
        }
    }
}
