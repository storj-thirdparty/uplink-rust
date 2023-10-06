//! Contains information and operations for uploading objects.

use crate::uplink_c::Ensurer;
use crate::{metadata, Error, Object, Result};

use std::ffi::{CStr, CString};
use std::time;
use std::vec::Vec;

use uplink_sys as ulksys;

/// Allows to upload the object's data to the Storj DCS network.
#[derive(Debug)]
pub struct Upload {
    /// The upload type of the FFI that an instance of this struct represents and guards its life
    /// time until the instances drops.
    ///
    /// It's an upload result because it's the one that holds the upload and allows to free its
    /// memory.
    ///
    /// `inner.error` must be NULL when this instance is created and should usually remain NULL
    /// except for the identified circumstance of the `self.write` method.
    inner: ulksys::UplinkUploadResult,
}

impl Upload {
    /// Creates a new instance from the FFI representation.
    ///
    /// It returns an error, through the
    /// [`Error::new_uplink` constructor](crate::Error::new_uplink), if `uc_upload` contains a non
    /// `NULL` pointer in the `error` field.
    pub(crate) fn from_ffi_upload_result(uc_upload: ulksys::UplinkUploadResult) -> Result<Self> {
        uc_upload.ensure();

        if let Some(err) = Error::new_uplink(uc_upload.error) {
            // SAFETY: we trust the FFI is safe freeing the memory of a correct value.
            unsafe { ulksys::uplink_free_upload_result(uc_upload) };
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
        // SAFETY: we trust the FFI when dealing with a correct instance.
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
        // SAFETY: we trust the FFI when dealing with a correct instance.
        let err = unsafe { ulksys::uplink_upload_commit(self.inner.upload) };
        if let Some(err) = Error::new_uplink(err) {
            Err(err)
        } else {
            Ok(())
        }
    }

    /// Returns the last information about the uploaded object.
    ///
    /// It returns an [`Error::Uplink`] if any of the calls to the FFI returns an error.
    pub fn info(&self) -> Result<Object> {
        // SAFETY: we trust the FFI when dealing with a correct instance.
        let uc_obj_res = unsafe { ulksys::uplink_upload_info(self.inner.upload) };

        Object::from_ffi_object_result(uc_obj_res)
            .map(|op| op.expect("successful upload info must always return an object"))
    }

    /// Updates the custom metadata to be included with the object.
    pub fn set_custom_metadata(&mut self, metadata: &mut metadata::Custom) -> Result<()> {
        let err = unsafe {
            ulksys::uplink_upload_set_custom_metadata(
                self.inner.upload,
                metadata.to_ffi_custom_metadata(),
            )
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
        // happened, we keep the returned FFI error in `self.inner.error` and in the next call to
        // `write` that the caller should to write the rest of the bytes, we return the error
        // returned on the previous call.
        if !self.inner.error.is_null() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                Error::new_uplink(self.inner.error)
                    .expect("BUG: missing a non NULL verification previous to this call"),
            ));
        }

        // SAFETY: we trust the FFI when dealing with a correct instance.
        //
        // We cannot use `buf.as_mut_ptr()` because `buf` is not passed as a mutable reference,
        // hence we have to directly cast it and it should not be a problem because the FFI
        // function doesn't write in this pointer despite the parameter is a `*mut c_void`.
        // We believe that the parameter is `mut` because it's what _bindgen_ has unfairly
        // generated.
        let uc_res = unsafe {
            ulksys::uplink_upload_write(
                self.inner.upload,
                (buf.as_ptr() as *mut u8).cast(),
                buf.len(),
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
        // SAFETY: we trust the FFI is safe freeing the memory of a correct value.
        unsafe { ulksys::uplink_free_upload_result(self.inner) };
    }
}

/// Iterator over a collection of uncommitted uploads.
pub struct Iterator {
    /// The upload iterator type of the FFI that an instance of this struct represents and guards
    /// its lifetime until the instance drops.
    inner: *mut ulksys::UplinkUploadIterator,
}

impl Iterator {
    /// Creates a new instance from the FFI representation.
    pub(crate) fn from_ffi_upload_iterator(uc_iterator: *mut ulksys::UplinkUploadIterator) -> Self {
        Self { inner: uc_iterator }
    }
}

impl std::iter::Iterator for Iterator {
    type Item = Result<Info>;

    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: we trust the FFI functions don't panic when called with an instance returned by
        // them and they don't return any invalid memory references or `null` if next returns
        // `true`.
        unsafe {
            if !ulksys::uplink_upload_iterator_next(self.inner) {
                let uc_error = ulksys::uplink_upload_iterator_err(self.inner);
                return Error::new_uplink(uc_error).map(Err);
            }

            Some(Ok(Info::from_ffi_upload_info(
                ulksys::uplink_upload_iterator_item(self.inner),
            )))
        }
    }
}

impl Drop for Iterator {
    fn drop(&mut self) {
        // SAFETY: we trust the FFI is safe freeing the memory of a correct pointer.
        unsafe { ulksys::uplink_free_upload_iterator(self.inner) };
    }
}

/// Contains information about a multipart upload operation.
pub struct Info {
    /// The ID associated to the upload.
    pub upload_id: String,
    /// The object's key associated to the upload.
    pub key: String,
    /// If `key` is a prefix or not.
    pub is_prefix: bool,
    /// The system metadata associated to the upload.
    pub metadata_system: metadata::System,
    /// The custom metadata associated to the upload.
    pub metadata_custom: metadata::Custom,
}

impl Info {
    /// Creates a new instance from the FFI representation.
    fn from_ffi_upload_info(uc_upload: *mut ulksys::UplinkUploadInfo) -> Self {
        assert!(
            !uc_upload.is_null(),
            "BUG: `uc_upload` argument cannot be NULL"
        );
        // SAFETY: we just checked above that this pointer isn't NULL.
        let upload = unsafe { *uc_upload };
        upload.ensure();

        let is_prefix = upload.is_prefix;
        let upload_id;
        let key;
        unsafe {
            upload_id = CStr::from_ptr(upload.upload_id)
                .to_str()
                .expect("FFI returned an invalid upload's ID; it contains invalid UTF-8 characters")
                .to_string();
            key = CStr::from_ptr(upload.key)
                .to_str()
                .expect(
                    "FFI returned an invalid upload's key; it contains invalid UTF-8 characters",
                )
                .to_string();

            ulksys::uplink_free_upload_info(uc_upload);
        }

        Self {
            upload_id,
            key,
            is_prefix,
            metadata_system: metadata::System::with_ffi_system_metadata(&upload.system),
            metadata_custom: metadata::Custom::with_ffi_custom_metadata(&upload.custom),
        }
    }

    /// Creates a new instance from the FFI representation for a info's result.
    ///
    /// It returns an error, through the
    /// [`Error::new_uplink` constructor](crate::Error::new_uplink), if `uc_result` contains a non
    /// `NULL` pointer in the `error` field.
    pub(crate) fn from_ffi_upload_info_result(
        uc_result: ulksys::UplinkUploadInfoResult,
    ) -> Result<Self> {
        uc_result.ensure();

        if let Some(err) = Error::new_uplink(uc_result.error) {
            // SAFETY: we trust the FFI is safe freeing the memory of a valid pointer.
            unsafe { ulksys::uplink_free_upload_info_result(uc_result) };
            return Err(err);
        }

        // At this point we don't need to free the `uc_result` because the following function free
        // the `info` pointer and the `error` pointer is `NULL`, and that's what the free function
        // for the `uc_result` does (i.e. call a free specific function for each pointer returning
        // without doing anything if it's `NULL`).
        Ok(Self::from_ffi_upload_info(uc_result.info))
    }
}

/// Metadata associated to an upload part of a multipart upload operation.
pub struct Part {
    /// The number of the part.
    pub part_number: u32,
    /// Plain size of the part
    pub size: usize,
    /// When the part was modified.
    pub modified: time::Duration,
    /// The entity tag of the part.
    pub etag: Vec<u8>,
}

impl Part {
    /// Creates a new instance from the FFI representation.
    fn from_ffi_part(uc_part: *mut ulksys::UplinkPart) -> Self {
        assert!(!uc_part.is_null(), "BUG: `uc_part` argument cannot be NULL");

        // SAFETY: we just checked above that this pointer isn't NULL.
        let part = unsafe { *uc_part };
        let modified = if part.modified < 0 {
            0
        } else {
            part.modified as u64
        };

        let part_number = part.part_number;
        let size = part.size;
        let mut etag = Vec::with_capacity(part.etag_length);
        // SAFETY: we trust the FFI in returning a correct length of the array that the `etag`
        // pointer points to, hence we believe that we are not accessing to a memory outside of the
        // array's bounds.
        unsafe {
            for i in 0..part.etag_length as isize {
                etag.push(*part.etag.offset(i) as u8)
            }

            ulksys::uplink_free_part(uc_part);
        }

        Self {
            part_number,
            size,
            modified: time::Duration::from_secs(modified),
            etag,
        }
    }

    /// Creates a new instance from the FFI representation for a part's result.
    ///
    /// It returns an error, through the
    /// [`Error::new_uplink` constructor](crate::Error::new_uplink), if `uc_result` contains a non
    /// `NULL` pointer in the `error` field.
    pub(crate) fn from_ffi_part_result(uc_result: ulksys::UplinkPartResult) -> Result<Self> {
        uc_result.ensure();

        if let Some(err) = Error::new_uplink(uc_result.error) {
            // SAFETY: we trust the FFI is safe freeing the memory of a valid pointer.
            unsafe { ulksys::uplink_free_part_result(uc_result) };
            return Err(err);
        }

        // At this point we don't need to free the `uc_result` because the following function free
        // the `part` pointer and the `error` pointer is `NULL`, and that's what the free function
        // for the `uc_result` does (i.e. call a free specific function for each pointer returning
        // without doing anything if it's `NULL`).
        Ok(Self::from_ffi_part(uc_result.part))
    }
}

/// Allows to upload partial object's data to the Storj DCS network through a multipart upload
/// operation.
pub struct PartUpload {
    /// The upload type of the FFI that an instance of this struct represents and guards its life
    /// time until the instances drops.
    ///
    /// It's an upload result because it's the one that holds the part upload and allows to free its
    /// memory.
    ///
    /// `inner.error` must be NULL when this instance is created and should usually remain NULL
    /// except for the identified circumstance of the `self.write` method.
    inner: ulksys::UplinkPartUploadResult,
}

impl PartUpload {
    /// Creates a new instance from the FFI representation.
    ///
    /// It returns an error, through the
    /// [`Error::new_uplink` constructor](crate::Error::new_uplink), if `uc_upload` contains a non
    /// `NULL` pointer in the `error` field.
    pub(crate) fn from_ffi_part_upload_result(
        uc_pupload: ulksys::UplinkPartUploadResult,
    ) -> Result<Self> {
        uc_pupload.ensure();

        if let Some(err) = Error::new_uplink(uc_pupload.error) {
            // SAFETY: we trust the FFI is safe freeing the memory of a valid value.
            unsafe { ulksys::uplink_free_part_upload_result(uc_pupload) };
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
        // SAFETY: we trust the FFI when dealing with a correct instance.
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
        // SAFETY: we trust the FFI when dealing with a correct instance.
        let err = unsafe { ulksys::uplink_part_upload_commit(self.inner.part_upload) };
        if let Some(err) = Error::new_uplink(err) {
            Err(err)
        } else {
            Ok(())
        }
    }

    /// Returns the last information about the uploaded part.
    ///
    /// It returns an [`Error::Uplink`] if any of the calls to the FFI returns an error.
    pub fn info(&self) -> Result<Part> {
        // SAFETY: we trust the FFI when dealing with a correct instance.
        let uc_part_res = unsafe { ulksys::uplink_part_upload_info(self.inner.part_upload) };

        Part::from_ffi_part_result(uc_part_res)
    }

    /// Sets the ETag for the part upload.
    ///
    /// It returns an [`Error::InvalidArguments`] if `etag` contains a 0 byte (NULL byte) or an
    /// [`Error::Uplink`] if the FFI returns an error.
    pub fn set_etag(&mut self, etag: &[u8]) -> Result<()> {
        let res = CString::new(etag);
        let res = res.map_err(|_| {
            Error::new_invalid_arguments(
                "etag",
                "cannot contain any 0 bytes (NULL bytes) due to the FFI requirements",
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
        // happened, we keep the returned FFI error in `self.inner.error` and in the next call to
        // `write` that the caller should to write the rest of the bytes, we return the error
        // returned on the previous call.
        if !self.inner.error.is_null() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                Error::new_uplink(self.inner.error)
                    .expect("BUG: missing a non NULL verification previous to this call"),
            ));
        }

        // SAFETY: we trust the FFI when dealing with a correct instance.
        //
        // We cannot use `buf.as_mut_ptr()` because `buf` is not passed as a mutable reference,
        // hence we have to directly cast it and it should not be a problem because the FFI
        // function doesn't write in this pointer despite the parameter is a `*mut c_void`.
        // We believe that the parameter is `mut` because it's what _bindgen_ has unfairly
        // generated.
        let uc_res = unsafe {
            ulksys::uplink_part_upload_write(
                self.inner.part_upload,
                (buf.as_ptr() as *mut u8).cast(),
                buf.len(),
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
        // SAFETY: we trust the FFI is safe freeing the memory of a valid value.
        unsafe { ulksys::uplink_free_part_upload_result(self.inner) };
    }
}

/// Iterator over a collection of parts of a multipart upload operation.
pub struct PartIterator {
    /// The upload iterator type of the FFI that an instance of this struct represents and guards
    /// its lifetime until the instance drops.
    inner: *mut ulksys::UplinkPartIterator,
}

impl PartIterator {
    /// Creates a new instance from the type exposed by the FFI.
    pub(crate) fn from_ffi_part_iterator(uc_iterator: *mut ulksys::UplinkPartIterator) -> Self {
        assert!(
            !uc_iterator.is_null(),
            "BUG: `uc_iterator` argument cannot be NULL"
        );

        Self { inner: uc_iterator }
    }
}

impl std::iter::Iterator for PartIterator {
    type Item = Result<Part>;

    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: we trust the FFI functions don't panic when called with an instance returned by
        // them and they don't return any invalid memory references or `null` if next returns
        // `true`.
        unsafe {
            if !ulksys::uplink_part_iterator_next(self.inner) {
                let uc_error = ulksys::uplink_part_iterator_err(self.inner);
                return Error::new_uplink(uc_error).map(Err);
            }

            Some(Ok(Part::from_ffi_part(ulksys::uplink_part_iterator_item(
                self.inner,
            ))))
        }
    }
}

impl Drop for PartIterator {
    fn drop(&mut self) {
        // SAFETY: we trust the FFI is safe freeing the memory of a correct pointer.
        unsafe { ulksys::uplink_free_part_iterator(self.inner) };
    }
}
