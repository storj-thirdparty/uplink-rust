//! Storj DCS Object and related types.

mod upload;
pub use upload::Upload;

use crate::{error::BoxError, metadata, Ensurer, Error, Result};

use std::ffi::CStr;

use uplink_sys as ulksys;

/// Contains information about an object.
pub struct Info<'a> {
    /// The identifier of the object inside of the bucket which it belongs.
    pub key: &'a str,
    /// Indicates if the key is a prefix for other objects.
    pub is_prefix: bool,
    /// The system metadata associated with the object.
    pub metadata_system: metadata::System,
    /// The custom metadata associated with the object.
    pub metadata_custom: metadata::Custom,
}

impl Info<'_> {
    /// Creates new instance from the underlying c-binding representation.
    ///
    /// It returns an error if `uc_obj` contains a key with invalid UTF-8 characters or
    /// [`metadata::Custom::from_uplink_c`] return an error.
    ///
    /// It consumes `uc_obj` hence the pointer isn't valid anymore after calling this method.
    pub(crate) fn from_uplink_c(uc_obj: *mut ulksys::UplinkObject) -> Result<Self> {
        if uc_obj.is_null() {
            return Err(Error::new_invalid_arguments("uc_obj", "cannot be null"));
        }

        let key: &str;
        let is_prefix: bool;
        let metadata_system: metadata::System;
        let metadata_custom: metadata::Custom;
        // SAFETY: we check before this block that pointer isn't NULL and inside of this block we
        // ensure that `uc_obj` doesn't have fields with NULL pointers through the `ensure` method
        // of the implemented `Ensurer` trait, and we also trust the underlying c-binding is safe
        // freeing the memory.
        unsafe {
            (*uc_obj).ensure();
            key = CStr::from_ptr((*uc_obj).key).to_str().map_err(|err| {
                Error::new_internal_with_inner(
                    "underlying-c binding returned an invalid object's key",
                    BoxError::from(err),
                )
            })?;
            metadata_custom = metadata::Custom::from_uplink_c(&(*uc_obj).custom);
            metadata_system = metadata::System::from_uplink_c(&(*uc_obj).system);
            is_prefix = (*uc_obj).is_prefix;
        }

        Ok(Self {
            key,
            is_prefix,
            metadata_system,
            metadata_custom,
        })
    }
}

impl Ensurer for ulksys::UplinkObject {
    fn ensure(&self) -> &Self {
        assert!(
            !self.key.is_null(),
            "underlying c-binding returned an invalid UplinkObject; key field is NULL",
        );

        self
    }
}

/// Iterates over a collection of objects' information.
pub struct Iterator {
    /// The object iterator type of the underlying c-bindings Rust crate that an instance of this
    /// struct represents and guards its life time until the instance drops.
    inner: *mut ulksys::UplinkObjectIterator,
}

impl Iterator {
    /// Creates a new instance from the type exposed by the uplink c-bindings.
    pub(crate) fn from_uplink_c(uc_iterator: *mut ulksys::UplinkObjectIterator) -> Result<Self> {
        if uc_iterator.is_null() {
            return Err(Error::new_invalid_arguments(
                "uc_iterator",
                "cannot be null",
            ));
        }

        Ok(Iterator { inner: uc_iterator })
    }
}

impl<'a> std::iter::Iterator for &'a Iterator {
    type Item = Result<Info<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: we trust that the underlying c-bindings functions don't panic when called with
        // an instance returned by them and they don't return any invalid memory references or
        // `null` if next returns `true`.
        unsafe {
            if !ulksys::uplink_object_iterator_next(self.inner) {
                let uc_error = ulksys::uplink_object_iterator_err(self.inner);
                return Error::new_uplink(uc_error).map(Err);
            }

            Some(Info::from_uplink_c(ulksys::uplink_object_iterator_item(
                self.inner,
            )))
        }
    }
}

impl Drop for Iterator {
    fn drop(&mut self) {
        // SAFETY: we trust that the underlying c-binding is safe freeing the memory of a correct
        // `UplinkObjectIterator` pointer.
        unsafe {
            ulksys::uplink_free_object_iterator(self.inner);
        }
    }
}

/// Represents a download object operation from Storj DCS network.
pub struct Download {
    /// The download type of the underlying c-bindings than an instance of this struct represents
    /// and guards its life time until the instance drops.
    ///
    /// It's an access result
    inner: ulksys::UplinkDownloadResult,
}

impl Download {
    /// Creates a new instance from the underlying c-bindings representation. The parameter is an
    /// `UplinkDownloadResult` because the it's the type that holds a `UplinkDownload` pointer and
    /// the function that frees the memory requires this type.
    ///
    /// It returns an error if `uc_download` contains a non NULL pointer in the `error` field.
    ///
    /// This function panics if `uc_download` is invalid, i.e. `download` and `error` fields are
    /// both NULL or not NULL>
    pub(crate) fn from_uplink_c(uc_download: ulksys::UplinkDownloadResult) -> Result<Self> {
        // Ensure it's valid.
        uc_download.ensure();

        if let Some(err) = Error::new_uplink(uc_download.error) {
            return Err(err);
        }

        Ok(Self { inner: uc_download })
    }

    /// Returns the last information about the object.
    pub fn info(&self) -> Result<Info> {
        // SAFETY: We trust the underlying c-bindings is behaving correctly when passing a valid
        // `UplinkDownload` instance.
        let obj_res = unsafe { ulksys::uplink_download_info(self.inner.download) };
        if let Some(err) = Error::new_uplink(obj_res.error) {
            return Err(err);
        }

        Info::from_uplink_c(obj_res.object)
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
        // SAFETY: we trust the underlying c-bindings of dealing with a correct `UplinkDownload`
        // instance and an allocated buffer.
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
        // SAFETY: we trust that the underlying c-bindings is doing correct operations when closing
        // and freeing a correctly created `UplinkDownloadResult` value.
        unsafe {
            // At this point we cannot do anything about the error, so discarded.
            // TODO: find out if retrying the operation it's the right thing to do for some of the
            // kind of errors that this function may return.
            let _ = ulksys::uplink_close_download(self.inner.download);
            ulksys::uplink_free_download_result(self.inner);
        }
    }
}

impl Ensurer for ulksys::UplinkObjectResult {
    fn ensure(&self) -> &Self {
        assert!(!self.object.is_null() || !self.error.is_null(), "underlying c-bindings returned an invalid UplinkObjectResult; object and error fields are both NULL");
        assert!((self.object.is_null() && !self.error.is_null())
            || (!self.object.is_null() && self.error.is_null())
            , "underlying c-bindings returned an invalid UplinkObjectResult; object and error fields are both NOT NULL");
        self
    }
}

impl Ensurer for ulksys::UplinkDownloadResult {
    fn ensure(&self) -> &Self {
        assert!(!self.download.is_null() || !self.error.is_null(), "underlying c-bindings returned an invalid UplinkDownloadResult; download and error fields are both NULL");
        assert!((self.download.is_null() && !self.error.is_null())
            || (!self.download.is_null() && self.error.is_null())
            , "underlying c-bindings returned an invalid UplinkDownloadResult; download and error fields are both NOT NULL");
        self
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use std::ptr;

    use uplink_sys as ulksys;

    #[test]
    #[should_panic(
        expected = "underlying c-bindings returned an invalid UplinkObjectResult; object and error fields are both NULL"
    )]
    fn test_ensurer_uplink_object_result_invalid_both_null() {
        let upload_res = ulksys::UplinkObjectResult {
            object: ptr::null_mut(),
            error: ptr::null_mut(),
        };

        upload_res.ensure();
    }

    #[test]
    #[should_panic(
        expected = "underlying c-bindings returned an invalid UplinkObjectResult; object and error fields are both NOT NULL"
    )]
    fn test_ensurer_uplink_object_result_invalid_both_not_null() {
        let upload_res = ulksys::UplinkObjectResult {
            object: &mut ulksys::UplinkObject {
                key: ptr::null_mut(),
                is_prefix: false,
                system: ulksys::UplinkSystemMetadata {
                    created: 0,
                    expires: 0,
                    content_length: 0,
                },
                custom: ulksys::UplinkCustomMetadata {
                    entries: ptr::null_mut(),
                    count: 0,
                },
            },
            error: &mut ulksys::UplinkError {
                code: 0,
                message: ptr::null_mut(),
            },
        };

        upload_res.ensure();
    }

    #[test]
    #[should_panic(
        expected = "underlying c-bindings returned an invalid UplinkDownloadResult; download and error fields are both NULL"
    )]
    fn test_ensurer_uplink_upload_result_invalid_both_null() {
        let upload_res = ulksys::UplinkDownloadResult {
            download: ptr::null_mut(),
            error: ptr::null_mut(),
        };

        upload_res.ensure();
    }

    #[test]
    #[should_panic(
        expected = "underlying c-bindings returned an invalid UplinkDownloadResult; download and error fields are both NOT NULL"
    )]
    fn test_ensurer_uplink_upload_result_invalid_both_not_null() {
        let upload_res = ulksys::UplinkDownloadResult {
            download: &mut ulksys::UplinkDownload { _handle: 0 },
            error: &mut ulksys::UplinkError {
                code: 0,
                message: ptr::null_mut(),
            },
        };

        upload_res.ensure();
    }
}
