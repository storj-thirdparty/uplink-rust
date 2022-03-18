//! Contains information and operations for uploading objects.

use crate::{metadata, object, Ensurer, Error, Result};

use std::ffi::{CStr, CString};
use std::time;
use std::vec::Vec;

use uplink_sys as ulksys;

/// Allows to upload the object's data to the Storj DCS network.
pub struct Upload {
    /// The upload type of the underlying c-bindings Rust crate that an instance of this struct
    /// represents and guards its life time until the instances drops.
    ///
    /// It's an upload result because it's the one that holds the upload and allows to free its
    /// memory.
    ///
    /// `inner.error` must be NULL when this instance is created and should usually remain NULL
    /// except for the identified circumstance of the `self.write` method.
    inner: ulksys::UplinkUploadResult,
}

impl Upload {
    /// Creates a new instance from the underlying c-bindings representation. The parameter is an
    /// `UplinkUploadResult` because the it's the type that holds a `UplinkUpload` pointer and
    /// the function that frees the memory requires this type.
    ///
    /// It returns an error if `uc_upload` contains a non NULL pointer in the `error` field.
    ///
    /// This function panics if `uc_upload` is invalid, i.e. `download` and `error` fields are
    /// both NULL or not NULL>
    pub(crate) fn from_uplink_c(uc_upload: ulksys::UplinkUploadResult) -> Result<Self> {
        // Ensure it's valid.
        uc_upload.ensure();

        if let Some(err) = Error::new_uplink(uc_upload.error) {
            Err(err)
        } else {
            Ok(Self { inner: uc_upload })
        }
    }

    /// Aborts a non-finalized upload.
    ///
    /// Returns an [`crate::Error::Uplink`] with the [`crate::error::Uplink::UploadDone`] if this
    /// method or [`Self::commit`] was previously called. It may return others [`Error::Uplink`]
    /// variants in other cases.
    pub fn abort(&mut self) -> Result<()> {
        // SAFETY: we trust the underlying c-bidings when dealing with a correct instance.
        let err = unsafe { ulksys::uplink_upload_abort(self.inner.upload) };
        if let Some(err) = Error::new_uplink(err) {
            Err(err)
        } else {
            Ok(())
        }
    }

    /// Commits the object's data to the store.
    ///
    /// Returns an [`crate::Error::Uplink`] with the [`crate::error::Uplink::UploadDone`] if this
    /// method or [`Self::abort`] was previously called. It may return others [`Error::Uplink`]
    /// variants in other cases.
    pub fn commit(&mut self) -> Result<()> {
        // SAFETY: we trust the underlying c-bidings when dealing with a correct instance.
        let err = unsafe { ulksys::uplink_upload_commit(self.inner.upload) };
        if let Some(err) = Error::new_uplink(err) {
            Err(err)
        } else {
            Ok(())
        }
    }

    /// Returns the last information about the uploaded object.
    ///
    /// It returns an [`Error::Uplink`] if any of the calls to the underlying-c binding returns an
    /// error.
    pub fn info(&self) -> Result<object::Info> {
        // SAFETY: we trust the underlying c-bidings when dealing with a correct instance.
        let uc_obj_res = unsafe { ulksys::uplink_upload_info(self.inner.upload) };

        // Ensure that's a valid instance.
        uc_obj_res.ensure();

        if let Some(err) = Error::new_uplink(uc_obj_res.error) {
            Err(err)
        } else {
            object::Info::from_uplink_c(uc_obj_res.object)
        }
    }

    /// Updates the custom metadata to be included with the object.
    pub fn set_custom_metadata(&mut self, metadata: &mut metadata::Custom) -> Result<()> {
        let err = unsafe {
            ulksys::uplink_upload_set_custom_metadata(self.inner.upload, metadata.to_uplink_c())
        };
        if let Some(err) = Error::new_uplink(err) {
            Err(err)
        } else {
            Ok(())
        }
    }
}

impl std::io::Write for Upload {
    /// Flush doesn't do anything, it only exists to fulfill the [`std::io::Write`] trait
    /// implementation. It always return `Ok(())`.
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    /// Uploads the bytes in `buf` to the object's data stream. It returns the total number of
    /// written bytes which are between 0 and the `buf` length or an error.
    ///
    /// When it returns an error is always a [`std::io::ErrorKind::Other`] and the error payload is
    /// an [`Error::Uplink`].
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // When self is created, it ensures that `self.inner.error` is NULL, but in order of being
        // able to return the written bytes when some of them are written but an error has
        // happened, we keep the returned c-bindings error in `self.inner.error` and in the next
        // call to `write` that the caller should to write the rest of the bytes, we return the
        // error returned on the previous call.
        if !self.inner.error.is_null() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                Error::new_uplink(self.inner.error)
                    .expect("BUG: missing a non NULL verification previous to this call"),
            ));
        }

        // SAFETY: we trust the underlying c-bindings when dealing with a correct instance.
        //
        // We cannot use `buf.as_mut_ptr()` because `buf` is not passed as a mutable reference,
        // hence we have to directly cast it and it should not be a problem because the c-bindings
        // function doesn't write in this pointer despite the parameter is a `*mut c_void`.
        // We believe that the parameter is `mut` because it's what _bindgen_ has unfairly
        // generated.
        let uc_res = unsafe {
            ulksys::uplink_upload_write(
                self.inner.upload,
                (buf.as_ptr() as *mut u8).cast(),
                buf.len() as u64,
            )
        };

        if !uc_res.error.is_null() {
            // There is an error and the operation didn't upload any byte, so we return the error
            // directly.
            if uc_res.bytes_written == 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    Error::new_uplink(uc_res.error)
                        .expect("BUG: missing a non NULL verification previous to this call"),
                ));
            }

            // There is an error but the operation uploaded a few bytes, so keep the error for
            // returning it on the next call `write` and this call returns the amount of uploaded
            // bytes.
            self.inner.error = uc_res.error;
        }

        Ok(uc_res.bytes_written as usize)
    }
}

impl Drop for Upload {
    fn drop(&mut self) {
        // SAFETY: we trust the underlying c-bindings is safe freeing the memory of a correct
        // value.
        unsafe { ulksys::uplink_free_upload_result(self.inner) };
    }
}

/// Iterator over a collection of uncommitted uploads.
pub struct Iterator {
    /// The upload iterator type of the underlying c-bindings Rust crate that an instance of this
    /// struct represents and guards its life time until the instance drops.
    inner: *mut ulksys::UplinkUploadIterator,
}

impl Iterator {
    /// Creates a new instance from the underlying c-bidings representation.
    pub(crate) fn from_uplink_c(uc_iterator: *mut ulksys::UplinkUploadIterator) -> Self {
        Self { inner: uc_iterator }
    }
}

impl<'a> std::iter::Iterator for &'a Iterator {
    type Item = Result<Info<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: we trust the underlying c-bindings functions don't panic when called with an
        // instance returned by them and they don't return any invalid memory references or `null`
        // if next returns `true`.
        unsafe {
            if !ulksys::uplink_upload_iterator_next(self.inner) {
                let uc_error = ulksys::uplink_upload_iterator_err(self.inner);
                return Error::new_uplink(uc_error).map(Err);
            }

            Some(Ok(Info::from_uplink_c(
                ulksys::uplink_upload_iterator_item(self.inner),
            )))
        }
    }
}

impl Drop for Iterator {
    fn drop(&mut self) {
        // SAFETY: we trust the underlying c-bindings is safe freeing the memory of a correct
        // pointer.
        unsafe { ulksys::uplink_free_upload_iterator(self.inner) };
    }
}

/// Contains information about a multipart upload operation.
pub struct Info<'a> {
    /// The ID associted to the upload.
    pub upload_id: &'a str,
    /// The object's key associted to the upload.
    pub key: &'a str,
    /// If `key` is a prefix or not.
    pub is_prefix: bool,
    /// The system metadata associated to the upload.
    pub metadata_system: metadata::System,
    /// The custom metadata associated to the upload.
    metadata_custom: metadata::Custom,
}

impl Info<'_> {
    /// Creates a new instance from the underlying c-bindings representation. It consumes the passed
    /// pointer so the caller must not use the pointer once it calls this function.
    ///
    /// It panics if `uc_upload` is NULL or is invalid, see [`ulksys::UplinkUploadInfo::ensure`].
    pub(crate) fn from_uplink_c(uc_upload: *mut ulksys::UplinkUploadInfo) -> Self {
        assert!(
            !uc_upload.is_null(),
            "BUG: `uc_upload` argument cannot be NULL"
        );

        // SAFETY: we just checked above thas this pointer isn't NULL.
        let upload = unsafe { *uc_upload };
        upload.ensure();

        let upload_id: &str;
        let key: &str;
        unsafe {
            upload_id = CStr::from_ptr(upload.upload_id)
                .to_str()
                .expect("invalid underlying c-binding, c-string with invalid UTF-8 characters");
            key = CStr::from_ptr(upload.key)
                .to_str()
                .expect("invalid underlying c-binding, c-string with invalid UTF-8 characters");
        }

        let info = Self {
            upload_id,
            key,
            is_prefix: upload.is_prefix,
            metadata_system: metadata::System::from_uplink_c(&upload.system),
            metadata_custom: metadata::Custom::from_uplink_c(&upload.custom),
        };

        // SAFETY: we just checked above that this pointer isn't NULL and we trust the underlying
        // c-bindings is safe freeing the memory of a valid pointer.
        unsafe { ulksys::uplink_free_upload_info(uc_upload) };

        info
    }
}

/// Metadata associated to an upload part of a multipart upload operation.
pub struct Part {
    pub part_number: u32,
    pub size: usize,
    pub modified: time::Duration,
    pub etag: Vec<u8>,
}

impl Part {
    /// Creates a new instance from the underlying c-bidnings representation. It consumes the
    /// passed pointer so the caller must no use the pointer once it calls this function.
    ///
    /// It panics if `uc_part` is NULL.
    pub(crate) fn from_uplink_c(uc_part: *mut ulksys::UplinkPart) -> Self {
        assert!(!uc_part.is_null(), "BUG: `uc_part` argument cannot be NULL");

        // SAFETY: we just checked above thas this pointer isn't NULL.
        let uc_partv = unsafe { *uc_part };
        let modified = if uc_partv.modified < 0 {
            0
        } else {
            uc_partv.modified as u64
        };

        let mut etag = Vec::with_capacity(uc_partv.etag_length as usize);
        // SAFETY: we trust the underlying c-bindings in returning a correct length of the array
        // that the `etag` pointer points to, hence we believe that we are not accessing to a
        // memory outside of the array's bounds.
        unsafe {
            for i in 0..uc_partv.etag_length as isize {
                etag.push(*uc_partv.etag.offset(i) as u8)
            }
        }

        let part = Self {
            part_number: uc_partv.part_number,
            size: uc_partv.size as usize,
            modified: time::Duration::from_secs(modified),
            etag,
        };

        // SAFETY: we just checked above that this pointer isn't NULL and we trust the underlying
        // c-bindings is safe freeing the memory of a valid pointer.
        unsafe { ulksys::uplink_free_part(uc_part) };

        part
    }

    /// Creates a new instance from the underlying c-bidnigs representation for a part's result. It
    /// returns an error if `uc_result` contains an error.
    ///
    /// It panics if `uc_result` is invalid, i.e., `uc_result.part` and `uc_result.error` are both
    /// NULL or not NULL.
    ///
    /// It returns an error if `uc_result.error` is not NULL.
    pub(crate) fn from_uplink_c_result(uc_result: ulksys::UplinkPartResult) -> Result<Self> {
        // Ensures that uc_result is valid.
        uc_result.ensure();

        if let Some(err) = Error::new_uplink(uc_result.error) {
            // SAFETY: we trust the underlying c-bindings is safe freeing the memory of a valid
            // pointer.
            unsafe { ulksys::uplink_free_part_result(uc_result) };
            return Err(err);
        }

        // At this point we don't need to free the `uc_result` because the following function free
        // the `part` pointer and the `error` pointer is NULL, and that's what the free function
        // for the `uc_result` does (i.e. call a free specific function for each pointer returning
        // without doing anything if it's NULL).
        Ok(Self::from_uplink_c(uc_result.part))
    }
}

/// Allows to upload partial object's data to the Storj DCS network through a multipart upload
/// operation.
pub struct PartUpload {
    /// The upload type of the underlying c-bindings Rust crate that an instance of this struct
    /// represents and guards its life time until the instances drops.
    ///
    /// It's an upload result because it's the one that holds the part upload and allows to free its
    /// memory.
    ///
    /// `inner.error` must be NULL when this instance is created and should usually remain NULL
    /// except for the identified circumstance of the `self.write` method.
    inner: ulksys::UplinkPartUploadResult,
}

impl PartUpload {
    /// Creates a new instance from the underlying c-bindings representation. The parameter is an
    /// `UplinkartUploadResult` because the it's the type that holds a `UplinkPartUpload` pointer
    /// and the function that frees the memory requires this type.
    ///
    /// It returns an error if `uc_pupload` contains a non NULL pointer in the `error` field.
    ///
    /// This function panics if `uc_pupload` is invalid, i.e. `download` and `error` fields are
    /// both NULL or not NULL>
    pub(crate) fn from_uplink_c(uc_pupload: ulksys::UplinkPartUploadResult) -> Result<Self> {
        // Ensure it's valid.
        uc_pupload.ensure();

        if let Some(err) = Error::new_uplink(uc_pupload.error) {
            Err(err)
        } else {
            Ok(Self { inner: uc_pupload })
        }
    }

    /// Aborts the part upload.
    ///
    ///
    /// Returns an [`crate::Error::Uplink`] with the [`crate::error::Uplink::UploadDone`] if this
    /// method or [`Self::commit`] was previously called. It may return others [`Error::Uplink`]
    /// variants in other cases.
    pub fn abort(&mut self) -> Result<()> {
        // SAFETY: we trust the underlying c-bidings when dealing with a correct instance.
        let err = unsafe { ulksys::uplink_part_upload_abort(self.inner.part_upload) };
        if let Some(err) = Error::new_uplink(err) {
            Err(err)
        } else {
            Ok(())
        }
    }

    /// Commits the part upload to the store.
    ///
    /// Returns an [`crate::Error::Uplink`] with the [`crate::error::Uplink::UploadDone`] if this
    /// method or [`Self::abort`] was previously called. It may return others [`Error::Uplink`]
    /// variants in other cases.
    pub fn commit(&mut self) -> Result<()> {
        // SAFETY: we trust the underlying c-bidings when dealing with a correct instance.
        let err = unsafe { ulksys::uplink_part_upload_commit(self.inner.part_upload) };
        if let Some(err) = Error::new_uplink(err) {
            Err(err)
        } else {
            Ok(())
        }
    }

    /// Returns the last information about the uploaded part.
    ///
    /// It returns an [`Error::Uplink`] if any of the calls to the underlying-c binding returns an
    /// error.
    pub fn info(&self) -> Result<Part> {
        // SAFETY: we trust the underlying c-bidings when dealing with a correct instance.
        let uc_part_res = unsafe { ulksys::uplink_part_upload_info(self.inner.part_upload) };

        // Ensure that's a valid instance.
        uc_part_res.ensure();

        if let Some(err) = Error::new_uplink(uc_part_res.error) {
            Err(err)
        } else {
            Ok(Part::from_uplink_c(uc_part_res.part))
        }
    }

    /// Sets the ETag for the part upload.
    ///
    /// It returns an [`Error::InvalidArguments`] if `etag` contains a 0 byte (NULL byte) or an
    /// [`Error::Uplink`] if the underlying-c call returns an error.
    pub fn set_etag(&mut self, etag: &[u8]) -> Result<()> {
        let res = CString::new(etag);
        let res = res.map_err(|_| {
            Error::new_invalid_arguments(
                "etag",
                "cannot contain any 0 bytes (NULL bytes) due to the c-bindings requirements",
            )
        });
        if res.is_err() {
            return res.map(|_| ());
        }

        let c_etag = res.expect("BUG: this result was verified to be an Ok, probably the check has been accidentally removed due to a refactoring");
        let err = unsafe {
            ulksys::uplink_part_upload_set_etag(
                self.inner.part_upload,
                c_etag.as_ptr() as *mut std::os::raw::c_char,
            )
        };

        if let Some(err) = Error::new_uplink(err) {
            Err(err)
        } else {
            Ok(())
        }
    }
}

impl std::io::Write for PartUpload {
    /// Flush doesn't do anything, it only exists to fulfill the [`std::io::Write`] trait
    /// implementation. It always return `Ok(())`.
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // When self is created, it ensures that `self.inner.error` is NULL, but in order of being
        // able to return the written bytes when some of them are written but an error has
        // happened, we keep the returned c-bindings error in `self.inner.error` and in the next
        // call to `write` that the caller should to write the rest of the bytes, we return the
        // error returned on the previous call.
        if !self.inner.error.is_null() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                Error::new_uplink(self.inner.error)
                    .expect("BUG: missing a non NULL verification previous to this call"),
            ));
        }

        // SAFETY: we trust the underlying c-bindings when dealing with a correct instance.
        //
        // We cannot use `buf.as_mut_ptr()` because `buf` is not passed as a mutable reference,
        // hence we have to directly cast it and it should not be a problem because the c-bindings
        // function doesn't write in this pointer despite the parameter is a `*mut c_void`.
        // We believe that the parameter is `mut` because it's what _bindgen_ has unfairly
        // generated.
        let uc_res = unsafe {
            ulksys::uplink_part_upload_write(
                self.inner.part_upload,
                (buf.as_ptr() as *mut u8).cast(),
                buf.len() as u64,
            )
        };

        if !uc_res.error.is_null() {
            // There is an error and the operation didn't upload any byte, so we return the error
            // directly.
            if uc_res.bytes_written == 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    Error::new_uplink(uc_res.error)
                        .expect("BUG: missing a non NULL verification previous to this call"),
                ));
            }

            // There is an error but the operation uploaded a few bytes, so keep the error for
            // returning it on the next call `write` and this call returns the amount of uploaded
            // bytes.
            self.inner.error = uc_res.error;
        }

        Ok(uc_res.bytes_written as usize)
    }
}

impl Drop for PartUpload {
    fn drop(&mut self) {
        // SAFETY: we trust the underlying c-bindings is safe freeing the memory of a valid value.
        unsafe { ulksys::uplink_free_part_upload_result(self.inner) };
    }
}

/// Iterator over a collection of parts of a multipart upload operation.
pub struct PartIterator {
    /// The upload iterator type of the underlying c-bindings Rust crate that an instance of this
    /// struct represents and guards its life time until the instance drops.
    inner: *mut ulksys::UplinkPartIterator,
}

impl std::iter::Iterator for PartIterator {
    type Item = Result<Part>;

    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: we trust the underlying c-bindings functions don't panic when called with an
        // instance returned by them and they don't return any invalid memory references or `null`
        // if next returns `true`.
        unsafe {
            if !ulksys::uplink_part_iterator_next(self.inner) {
                let uc_error = ulksys::uplink_part_iterator_err(self.inner);
                return Error::new_uplink(uc_error).map(Err);
            }

            Some(Ok(Part::from_uplink_c(ulksys::uplink_part_iterator_item(
                self.inner,
            ))))
        }
    }
}

impl Drop for PartIterator {
    fn drop(&mut self) {
        // SAFETY: we trust the underlying c-bindings is safe freeing the memory of a correct
        // pointer.
        unsafe { ulksys::uplink_free_part_iterator(self.inner) };
    }
}

impl Ensurer for ulksys::UplinkUploadInfo {
    fn ensure(&self) -> &Self {
        assert!(
            !self.upload_id.is_null(),
            "underlying c-bindings returned an invalid UplinkUploadInfo; upload_id field is NULL"
        );
        assert!(
            !self.key.is_null(),
            "underlying c-bindings returned an invalid UplinkUploadInfo; key field is NULL"
        );
        self
    }
}

impl Ensurer for ulksys::UplinkUploadResult {
    fn ensure(&self) -> &Self {
        assert!(!self.upload.is_null() || !self.error.is_null(), "underlying c-bindings returned an invalid UplinkUploadResult; upload and error fields are both NULL");
        assert!((self.upload.is_null() && !self.error.is_null())
            || (!self.upload.is_null() && self.error.is_null())
             ,"underlying c-bindings returned an invalid UplinkUploadResult; upload and error fields are both NOT NULL");
        self
    }
}

impl Ensurer for ulksys::UplinkPartResult {
    fn ensure(&self) -> &Self {
        assert!(!self.part.is_null() || !self.error.is_null(), "underlying c-bindings returned an invalid UplinkPartResult; part and error fields are both NULL");
        assert!((self.part.is_null() && !self.error.is_null())
            || (!self.part.is_null() && self.error.is_null())
             ,"underlying c-bindings returned an invalid UplinkPartResult; part and error fields are both NOT NULL");
        self
    }
}

impl Ensurer for ulksys::UplinkPartUploadResult {
    fn ensure(&self) -> &Self {
        assert!(!self.part_upload.is_null() || !self.error.is_null(), "underlying c-bindings returned an invalid UplinkPartUploadResult; part_upload and error fields are both NULL");
        assert!((self.part_upload.is_null() && !self.error.is_null())
            || (!self.part_upload.is_null() && self.error.is_null())
             ,"underlying c-bindings returned an invalid UplinkPartUploadResult; part_upload and error fields are both NOT NULL");
        self
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use std::ptr;

    use uplink_sys as ulksys;

    /*** Ensurer implementations tests ***/
    #[test]
    #[should_panic(
        expected = "underlying c-bindings returned an invalid UplinkUploadInfo; upload_id field is NULL"
    )]
    fn test_ensurer_uplink_upload_info_null_id() {
        let key = CString::new("some-key")
            .expect("BUG: the passed string should contain only valid UTF-8 chars");
        let info = ulksys::UplinkUploadInfo {
            upload_id: ptr::null_mut(),
            key: key.as_ptr() as *mut i8,
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
        };

        info.ensure();
    }

    #[test]
    #[should_panic(
        expected = "underlying c-bindings returned an invalid UplinkUploadInfo; key field is NULL"
    )]
    fn test_ensurer_uplink_upload_info_null_key() {
        let id = CString::new("95584")
            .expect("BUG: the passed string should contain only valid UTF-8 chars");
        let info = ulksys::UplinkUploadInfo {
            upload_id: id.as_ptr() as *mut i8,
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
        };

        info.ensure();
    }

    #[test]
    #[should_panic(
        expected = "underlying c-bindings returned an invalid UplinkUploadResult; upload and error fields are both NULL"
    )]
    fn test_ensurer_uplink_upload_result_invalid_both_null() {
        let upload_res = ulksys::UplinkUploadResult {
            upload: ptr::null_mut(),
            error: ptr::null_mut(),
        };

        upload_res.ensure();
    }

    #[test]
    #[should_panic(
        expected = "underlying c-bindings returned an invalid UplinkUploadResult; upload and error fields are both NOT NULL"
    )]
    fn test_ensurer_uplink_upload_result_invalid_both_not_null() {
        let upload_res = ulksys::UplinkUploadResult {
            upload: &mut ulksys::UplinkUpload { _handle: 0 },
            error: &mut ulksys::UplinkError {
                code: 0,
                message: ptr::null_mut(),
            },
        };

        upload_res.ensure();
    }

    #[test]
    #[should_panic(
        expected = "underlying c-bindings returned an invalid UplinkPartResult; part and error fields are both NULL"
    )]
    fn test_ensurer_uplink_part_result_invalid_both_null() {
        let upload_res = ulksys::UplinkPartResult {
            part: ptr::null_mut(),
            error: ptr::null_mut(),
        };

        upload_res.ensure();
    }

    #[test]
    #[should_panic(
        expected = "underlying c-bindings returned an invalid UplinkPartResult; part and error fields are both NOT NULL"
    )]
    fn test_ensurer_uplink_part_result_invalid_both_not_null() {
        let upload_res = ulksys::UplinkPartResult {
            part: &mut ulksys::UplinkPart {
                part_number: 0,
                size: 0,
                modified: 0,
                etag: ptr::null_mut(),
                etag_length: 0,
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
        expected = "underlying c-bindings returned an invalid UplinkPartUploadResult; part_upload and error fields are both NULL"
    )]
    fn test_ensurer_uplink_part_upload_result_invalid_both_null() {
        let pupload_res = ulksys::UplinkPartUploadResult {
            part_upload: ptr::null_mut(),
            error: ptr::null_mut(),
        };

        pupload_res.ensure();
    }

    #[test]
    #[should_panic(
        expected = "underlying c-bindings returned an invalid UplinkPartUploadResult; part_upload and error fields are both NOT NULL"
    )]
    fn test_ensurer_uplink_part_upload_result_invalid_both_not_null() {
        let upload_res = ulksys::UplinkPartUploadResult {
            part_upload: &mut ulksys::UplinkPartUpload { _handle: 0 },
            error: &mut ulksys::UplinkError {
                code: 0,
                message: ptr::null_mut(),
            },
        };

        upload_res.ensure();
    }
}
