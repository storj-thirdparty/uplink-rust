//! Storj DCS Object and related types.

pub mod upload;

pub use upload::Upload;

use crate::error::BoxError;
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
    /// When no error an `Option` is returned which is `None` when `uc_obj` is `NULL`. This happens
    /// in some specific situations.
    ///
    /// An [`Error::Internal`](crate::Error::Internal) if `uc_obj`'s key contains invalid UTF-8
    /// characters or [`metadata::Custom::with_ffi_custom_metadata`] return an error.
    fn from_ffi_object(uc_obj: *mut ulksys::UplinkObject) -> Result<Option<Self>> {
        if uc_obj.is_null() {
            return Ok(None);
        }

        let uc_obj_ptr = uc_obj;
        // SAFETY: We have checked just above that the pointer isn't NULL.
        let uc_obj = unsafe { *uc_obj_ptr };
        uc_obj.ensure();

        let key;
        let is_prefix;
        let metadata_system: metadata::System;
        let metadata_custom: metadata::Custom;
        // SAFETY: we have check that the `uc_obj` doesn't have fields with NULL pointers through
        // the `ensure` method.
        unsafe {
            let cs = CString::from(CStr::from_ptr(uc_obj.key));
            key = cs.into_string().map_err(|err| {
                ulksys::uplink_free_object(uc_obj_ptr);
                Error::new_internal(
                    "FFI returned an invalid object's key; it contains invalid UTF-8 characters",
                    BoxError::from(err),
                )
            })?;
            metadata_custom = metadata::Custom::with_ffi_custom_metadata(&uc_obj.custom);
            metadata_system = metadata::System::with_ffi_system_metadata(&uc_obj.system);
            is_prefix = uc_obj.is_prefix;
            ulksys::uplink_free_object(uc_obj_ptr);
        }

        Ok(Some(Self {
            key,
            is_prefix,
            metadata_system,
            metadata_custom,
        }))
    }

    /// Creates a new instance from the FFI representation for an object's result.
    ///
    /// See [`from_ffi_object`](Self::from_ffi_object) why an `Option` is returned when result is
    /// OK.
    ///
    /// It returns the following errors:
    /// * an [`Error::new_uplink` constructor](crate::Error::new_uplink), if `uc_result` contains a
    ///   non `NULL` pointer in the `error` field.
    /// * an [`Error::Internal`](crate::Error::Internal) if `uc_result.object`'s key contains
    ///   invalid UTF-8 characters or [`metadata::Custom::with_ffi_custom_metadata`] return an
    ///   error.
    pub(crate) fn from_ffi_object_result(
        uc_result: ulksys::UplinkObjectResult,
    ) -> Result<Option<Self>> {
        if let Some(err) = Error::new_uplink(uc_result.error) {
            // SAFETY: we trust the FFI is safe freeing the memory of a valid pointer.
            unsafe { ulksys::uplink_free_object_result(uc_result) };
            return Err(err);
        }

        // At this point we don't need to free the `uc_result` because the following function free
        // the `object` pointer and the `error` pointer is `NULL`, and that's what the free function
        // for the `uc_result` does (i.e. call a free specific function for each pointer returning
        // without doing anything if it's `NULL`).
        Self::from_ffi_object(uc_result.object)
    }

    /// Creates a new instance from the FFI representation for a commit upload's result.
    ///
    /// It returns the following errors:
    /// * an [`Error::new_uplink` constructor](crate::Error::new_uplink), if `uc_result` contains a
    ///   non `NULL` pointer in the `error` field.
    /// * an [`Error::Internal`](crate::Error::Internal) if `uc_result.object`'s key contains
    ///   invalid UTF-8 characters or [`metadata::Custom::with_ffi_custom_metadata`] return an
    ///   error.
    pub(crate) fn from_ffi_commit_upload_result(
        uc_result: ulksys::UplinkCommitUploadResult,
    ) -> Result<Self> {
        uc_result.ensure();

        if let Some(err) = Error::new_uplink(uc_result.error) {
            // SAFETY: we trust the FFI is safe freeing the memory of a valid pointer.
            unsafe { ulksys::uplink_free_commit_upload_result(uc_result) };
            return Err(err);
        }

        // At this point we don't need to free the `uc_result` because the following function free
        // the `info` pointer and the `error` pointer is `NULL`, and that's what the free function
        // for the `uc_result` does (i.e. call a free specific function for each pointer returning
        // without doing anything if it's `NULL`).
        Self::from_ffi_object(uc_result.object)
            .map(|op| op.expect("successful committed upload must always return an object"))
    }
}

/// Iterates over a collection of objects' information.
#[derive(Debug)]
pub struct Iterator {
    /// The object iterator type of the FFI that an instance of this struct represents and guards
    /// its lifetime until the instance drops.
    inner: *mut ulksys::UplinkObjectIterator,
}

impl Iterator {
    /// Creates a new instance from the type exposed by the FFI.
    pub(crate) fn from_ffi_object_iterator(uc_iterator: *mut ulksys::UplinkObjectIterator) -> Self {
        assert!(
            !uc_iterator.is_null(),
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
            if !ulksys::uplink_object_iterator_next(self.inner) {
                let uc_error = ulksys::uplink_object_iterator_err(self.inner);
                return Error::new_uplink(uc_error).map(Err);
            }

            Some(
                Object::from_ffi_object(ulksys::uplink_object_iterator_item(self.inner)).map(
                    |op| op.expect("an iterator that indicated that there is a next element always returns it")
                ),
            )
        }
    }
}

impl Drop for Iterator {
    fn drop(&mut self) {
        // SAFETY: we trust that the FFI is safe freeing the memory of a correct
        // `UplinkObjectIterator` pointer.
        unsafe {
            ulksys::uplink_free_object_iterator(self.inner);
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
    inner: ulksys::UplinkDownloadResult,
}

impl Download {
    /// Creates a new instance from the FFI representation.
    ///
    /// It returns an error, through the
    /// [`Error::new_uplink` constructor](crate::Error::new_uplink), if `uc_result` contains a non
    /// `NULL` pointer in the `error` field.
    pub(crate) fn from_ffi_download_result(
        uc_result: ulksys::UplinkDownloadResult,
    ) -> Result<Self> {
        uc_result.ensure();

        if let Some(err) = Error::new_uplink(uc_result.error) {
            // SAFETY: we trust the FFI is safe freeing the memory of a valid pointer.
            unsafe { ulksys::uplink_free_download_result(uc_result) };
            return Err(err);
        }

        Ok(Self { inner: uc_result })
    }

    /// Returns the last information about the object.
    ///
    /// It returns if FFI returns an error when retrieving the information.
    pub fn info(&self) -> Result<Object> {
        // SAFETY: We trust the FFI is behaving correctly when passing a valid `UplinkDownload`
        // instance.
        let obj_res = unsafe { ulksys::uplink_download_info(self.inner.download) };
        if let Some(err) = Error::new_uplink(obj_res.error) {
            return Err(err);
        }

        Object::from_ffi_object(obj_res.object)
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
        for _ in 1..3 {
            let bp = buf.as_mut_ptr();
            // SAFETY: we trust the FFI of dealing with a correct `UplinkDownload` instance and an
            // allocated buffer.
            let read_res =
                unsafe { ulksys::uplink_download_read(self.inner.download, bp.cast(), buf.len()) };

            if let Some(err) = Error::new_uplink(read_res.error) {
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
        // SAFETY: we trust that the FFI is doing correct operations when closing and freeing a
        // correctly created `UplinkDownloadResult` value.
        unsafe {
            // At this point we cannot do anything about the error, so discarded.
            // TODO(https://github.com/storj-thirdparty/uplink-rust/issues/51).
            let _ = ulksys::uplink_close_download(self.inner.download);
            ulksys::uplink_free_download_result(self.inner);
        }
    }
}
