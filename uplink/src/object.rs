//! Storj DCS Object and related types.

pub mod upload;

pub use upload::Upload;

use crate::error::BoxError;
use crate::uplink_c::Ensurer;
use crate::{metadata, Error, Result};

use std::ffi::CStr;

use uplink_sys as ulksys;

/// Contains information about an object.
pub struct Object<'a> {
    /// The identifier of the object inside of the bucket which it belongs.
    pub key: &'a str,
    /// Indicates if the key is a prefix for other objects.
    pub is_prefix: bool,
    /// The system metadata associated with the object.
    pub metadata_system: metadata::System,
    /// The custom metadata associated with the object.
    pub metadata_custom: metadata::Custom,
}

impl Object<'_> {
    /// Creates new instance from the FFI representation.
    ///
    /// An [`Error::Internal`](crate::Error::Internal) if `uc_obj`'s key contains invalid UTF-8
    /// characters or [`metadata::Custom::with_ffi_custom_metadata`] return an error.
    fn from_ffi_object(uc_obj: *mut ulksys::UplinkObject) -> Result<Self> {
        assert!(!uc_obj.is_null(), "BUG: `uc_obj` argument cannot be NULL");

        let uc_obj_ptr = uc_obj;
        // SAFETY: We have checked just above that the pointer isn't NULL.
        let uc_obj = unsafe { *uc_obj_ptr };
        uc_obj.ensure();

        let key: &str;
        let is_prefix: bool;
        let metadata_system: metadata::System;
        let metadata_custom: metadata::Custom;
        // SAFETY: we have check that the `uc_obj` doesn't have fields with NULL pointers through
        // the `ensure` method.
        unsafe {
            key = CStr::from_ptr(uc_obj.key).to_str().map_err(|err| {
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

        Ok(Self {
            key,
            is_prefix,
            metadata_system,
            metadata_custom,
        })
    }

    /// Creates a new instance from the FFI representation for an object's result.
    ///
    /// It returns the following errors:
    /// * an [`Error::new_uplink` constructor](crate::Error::new_uplink), if `uc_result` contains a
    ///   non `NULL` pointer in the `error` field.
    /// * an [`Error::Internal`](crate::Error::Internal) if `uc_result.object`'s key contains
    ///   invalid UTF-8 characters or [`metadata::Custom::with_ffi_custom_metadata`] return an
    ///   error.
    pub(crate) fn from_ffi_object_result(uc_result: ulksys::UplinkObjectResult) -> Result<Self> {
        uc_result.ensure();

        if let Some(err) = Error::new_uplink(uc_result.error) {
            // SAFETY: we trust the FFI is safe freeing the memory of a valid pointer.
            unsafe { ulksys::uplink_free_object_result(uc_result) };
            return Err(err);
        }

        // At this point we don't need to free the `uc_result` because the following function free
        // the `info` pointer and the `error` pointer is `NULL`, and that's what the free function
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
    }
}

/// Iterates over a collection of objects' information.
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

impl<'a> std::iter::Iterator for &'a Iterator {
    type Item = Result<Object<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: we trust that the FFI functions don't panic when called with an instance returned
        // by them and they don't return any invalid memory references or `null` if next returns
        // `true`.
        unsafe {
            if !ulksys::uplink_object_iterator_next(self.inner) {
                let uc_error = ulksys::uplink_object_iterator_err(self.inner);
                return Error::new_uplink(uc_error).map(Err);
            }

            Some(Object::from_ffi_object(
                ulksys::uplink_object_iterator_item(self.inner),
            ))
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
    }
}

impl std::io::Read for Download {
    /// Downloads the object's data stream into `buf` and return the number of downloaded bytes,
    /// which are at most the `buf` length, when there isn't any error.
    ///
    /// When it returns an error is always a [`std::io::ErrorKind::Other`] and the error payload is
    /// an [`Error::Uplink`].
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let bp = buf.as_mut_ptr();
        // SAFETY: we trust the FFI of dealing with a correct `UplinkDownload` instance and an
        // allocated buffer.
        let read_res = unsafe {
            ulksys::uplink_download_read(self.inner.download, bp.cast(), buf.len() as u64)
        };

        if let Some(err) = Error::new_uplink(read_res.error) {
            use std::io::{Error as IOErr, ErrorKind};
            return Err(IOErr::new(ErrorKind::Other, err));
        }

        Ok(read_res.bytes_read as usize)
    }
}

impl Drop for Download {
    fn drop(&mut self) {
        // SAFETY: we trust that the FFI is doing correct operations when closing and freeing a
        // correctly created `UplinkDownloadResult` value.
        unsafe {
            // At this point we cannot do anything about the error, so discarded.
            // TODO: find out if retrying the operation it's the right thing to do for some of the
            // kind of errors that this function may return.
            let _ = ulksys::uplink_close_download(self.inner.download);
            ulksys::uplink_free_download_result(self.inner);
        }
    }
}
