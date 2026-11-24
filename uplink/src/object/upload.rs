//! Contains information and operations for uploading objects.

use crate::guard::Guard;
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
    inner: Guard<ulksys::UplinkUploadResult>,
    /// The result of the last write when some bytes were written before an error happened.
    failed_write: Option<Guard<ulksys::UplinkWriteResult>>,
}

impl Upload {
    /// Creates a new instance from the FFI representation.
    ///
    /// It returns an [`Error::Uplink`] if the FFI result contains an error.
    pub(crate) fn from_ffi_upload_result(
        uc_upload: Guard<ulksys::UplinkUploadResult>,
    ) -> Result<Self> {
        uc_upload.ensure();
        uc_upload.check()?;

        Ok(Self {
            inner: uc_upload,
            failed_write: None,
        })
    }

    /// Aborts a non-finalized upload.
    ///
    /// Returns an [`crate::Error::Uplink`] with the [`crate::error::Uplink::UploadDone`] if this
    /// method or [`Self::commit`] was previously called. It may return others [`Error::Uplink`]
    /// variants in other cases.
    pub fn abort(&mut self) -> Result<()> {
        // SAFETY: we trust the FFI when dealing with a correct instance.
        unsafe { Guard::new(ulksys::uplink_upload_abort(self.inner.upload)) }.check()
    }

    /// Commits the object's data to the store.
    ///
    /// Returns an [`crate::Error::Uplink`] with the [`crate::error::Uplink::UploadDone`] if this
    /// method or [`Self::abort`] was previously called. It may return others [`Error::Uplink`]
    /// variants in other cases.
    pub fn commit(&mut self) -> Result<()> {
        // SAFETY: we trust the FFI when dealing with a correct instance.
        unsafe { Guard::new(ulksys::uplink_upload_commit(self.inner.upload)) }.check()
    }

    /// Returns the last information about the uploaded object.
    ///
    /// It returns an [`Error::Uplink`] if any of the calls to the FFI returns an error.
    pub fn info(&self) -> Result<Object> {
        // SAFETY: we trust the FFI when dealing with a correct instance.
        let uc_obj_res = unsafe { Guard::new(ulksys::uplink_upload_info(self.inner.upload)) };

        Object::from_ffi_object_result(uc_obj_res)
            .map(|op| op.expect("successful upload info must always return an object"))
    }

    /// Updates the custom metadata to be included with the object.
    pub fn set_custom_metadata(&mut self, metadata: &mut metadata::Custom) -> Result<()> {
        // SAFETY: We are sure that FFI doesn't take ownership of the two parameters.
        // Metadata FFI type is obtained from a safe wrapper.
        let err = unsafe {
            Guard::new(ulksys::uplink_upload_set_custom_metadata(
                self.inner.upload,
                metadata.to_ffi_custom_metadata(),
            ))
        };

        err.check()
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
        // When some bytes are written but an error happens, we return the number of written bytes
        // and we keep the result for returning its error in the next call that the caller should do
        // to write the rest of the bytes.
        if let Some(err) = self.failed_write.as_ref().and_then(Guard::error) {
            return Err(std::io::Error::other(err));
        }

        // SAFETY: we trust the FFI when dealing with a correct instance.
        //
        // We cannot use `buf.as_mut_ptr()` because `buf` is not passed as a mutable reference,
        // hence we have to directly cast it and it should not be a problem because the FFI
        // function doesn't write in this pointer despite the parameter is a `*mut c_void`.
        // We believe that the parameter is `mut` because it's what _bindgen_ has unfairly
        // generated.
        let uc_res = unsafe {
            Guard::new(ulksys::uplink_upload_write(
                self.inner.upload,
                (buf.as_ptr() as *mut u8).cast(),
                buf.len(),
            ))
        };

        let written = uc_res.bytes_written as usize;
        if let Some(err) = uc_res.error() {
            // There is an error and the operation didn't upload any byte, so we return the error
            // directly.
            if written == 0 {
                return Err(std::io::Error::other(err));
            }

            self.failed_write = Some(uc_res);
        }

        Ok(written)
    }
}

/// Iterator over a collection of uncommitted uploads.
pub struct Iterator {
    /// The upload iterator type of the FFI that an instance of this struct represents and guards
    /// its lifetime until the instance drops.
    inner: Guard<*mut ulksys::UplinkUploadIterator>,
}

impl Iterator {
    /// Creates a new instance from the FFI representation.
    pub(crate) fn from_ffi_upload_iterator(
        uc_iterator: Guard<*mut ulksys::UplinkUploadIterator>,
    ) -> Self {
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
            if !ulksys::uplink_upload_iterator_next(*self.inner) {
                let uc_error = Guard::new(ulksys::uplink_upload_iterator_err(*self.inner));
                return uc_error.error().map(Err);
            }

            let uc_upload = Guard::new(ulksys::uplink_upload_iterator_item(*self.inner));
            Some(Ok(Info::with_ffi_upload_info(uc_upload.value().expect(
                "an iterator that indicated that there is a next element always returns it",
            ))))
        }
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
    fn with_ffi_upload_info(uc_upload: &ulksys::UplinkUploadInfo) -> Self {
        uc_upload.ensure();

        // SAFETY: We have guarantee that upload fields aren't null through the `ensure` method
        // call of the `Ensurer` trait.
        let (upload_id, key) = unsafe {
            (
                CStr::from_ptr(uc_upload.upload_id),
                CStr::from_ptr(uc_upload.key),
            )
        };

        Self {
            upload_id: upload_id
                .to_str()
                .expect("FFI returned an invalid upload's ID; it contains invalid UTF-8 characters")
                .to_string(),
            key: key
                .to_str()
                .expect(
                    "FFI returned an invalid upload's key; it contains invalid UTF-8 characters",
                )
                .to_string(),
            is_prefix: uc_upload.is_prefix,
            metadata_system: metadata::System::with_ffi_system_metadata(&uc_upload.system),
            metadata_custom: metadata::Custom::with_ffi_custom_metadata(&uc_upload.custom),
        }
    }

    /// Creates a new instance from the FFI representation for a info's result.
    ///
    /// It returns an [`Error::Uplink`] if `uc_result` contains an error.
    pub(crate) fn from_ffi_upload_info_result(
        uc_result: Guard<ulksys::UplinkUploadInfoResult>,
    ) -> Result<Self> {
        uc_result.ensure();
        uc_result.check()?;

        Ok(Self::with_ffi_upload_info(uc_result.value().expect(
            "successful upload info result must always have an upload info",
        )))
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
    fn with_ffi_part(uc_part: &ulksys::UplinkPart) -> Self {
        let modified = if uc_part.modified < 0 {
            0
        } else {
            uc_part.modified as u64
        };

        let mut etag = Vec::with_capacity(uc_part.etag_length);
        // SAFETY: we trust the FFI in returning a correct length of the array that the `etag`
        // pointer points to, hence we believe that we are not accessing to a memory outside of the
        // array's bounds.
        unsafe {
            for i in 0..uc_part.etag_length as isize {
                etag.push(*uc_part.etag.offset(i) as u8)
            }
        }

        Self {
            part_number: uc_part.part_number,
            size: uc_part.size,
            modified: time::Duration::from_secs(modified),
            etag,
        }
    }

    /// Creates a new instance from the FFI representation for a part's result.
    ///
    /// It returns an [`Error::Uplink`] if `uc_result` contains an error.
    pub(crate) fn from_ffi_part_result(uc_result: Guard<ulksys::UplinkPartResult>) -> Result<Self> {
        uc_result.ensure();
        uc_result.check()?;

        Ok(Self::with_ffi_part(
            uc_result
                .value()
                .expect("successful part result must always have a part"),
        ))
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
    inner: Guard<ulksys::UplinkPartUploadResult>,
    /// The result of the last write when some bytes were written before an error happened.
    failed_write: Option<Guard<ulksys::UplinkWriteResult>>,
}

impl PartUpload {
    /// Creates a new instance from the FFI representation.
    ///
    /// It returns an [`Error::Uplink`] if the FFI result contains an error.
    pub(crate) fn from_ffi_part_upload_result(
        uc_pupload: Guard<ulksys::UplinkPartUploadResult>,
    ) -> Result<Self> {
        uc_pupload.ensure();
        uc_pupload.check()?;

        Ok(Self {
            inner: uc_pupload,
            failed_write: None,
        })
    }

    /// Aborts the part upload.
    ///
    ///
    /// Returns an [`crate::Error::Uplink`] with the [`crate::error::Uplink::UploadDone`] if this
    /// method or [`Self::commit`] was previously called. It may return others [`Error::Uplink`]
    /// variants in other cases.
    pub fn abort(&mut self) -> Result<()> {
        // SAFETY: we trust the FFI when dealing with a correct instance.
        unsafe { Guard::new(ulksys::uplink_part_upload_abort(self.inner.part_upload)) }.check()
    }

    /// Commits the part upload to the store.
    ///
    /// Returns an [`crate::Error::Uplink`] with the [`crate::error::Uplink::UploadDone`] if this
    /// method or [`Self::abort`] was previously called. It may return others [`Error::Uplink`]
    /// variants in other cases.
    pub fn commit(&mut self) -> Result<()> {
        // SAFETY: we trust the FFI when dealing with a correct instance.
        unsafe { Guard::new(ulksys::uplink_part_upload_commit(self.inner.part_upload)) }.check()
    }

    /// Returns the last information about the uploaded part.
    ///
    /// It returns an [`Error::Uplink`] if any of the calls to the FFI returns an error.
    pub fn info(&self) -> Result<Part> {
        // SAFETY: we trust the FFI when dealing with a correct instance.
        let uc_part_res =
            unsafe { Guard::new(ulksys::uplink_part_upload_info(self.inner.part_upload)) };

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
        // SAFETY: We get a pointer from a Rust string after guarantee that doesn't contain any 0
        // byte (NULL byte) and we pass it to the FFI.
        // `self.inner` is guarantee to be valid by the constructor of this struct.
        let err = unsafe {
            Guard::new(ulksys::uplink_part_upload_set_etag(
                self.inner.part_upload,
                c_etag.as_ptr() as *mut std::os::raw::c_char,
            ))
        };

        err.check()
    }
}

impl std::io::Write for PartUpload {
    /// Flush doesn't do anything, it only exists to fulfill the [`std::io::Write`] trait
    /// implementation. It always return `Ok(())`.
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // When some bytes are written but an error happens, we return the number of written bytes
        // and we keep the result for returning its error in the next call that the caller should do
        // to write the rest of the bytes.
        if let Some(err) = self.failed_write.as_ref().and_then(Guard::error) {
            return Err(std::io::Error::other(err));
        }

        // SAFETY: we trust the FFI when dealing with a correct instance.
        //
        // We cannot use `buf.as_mut_ptr()` because `buf` is not passed as a mutable reference,
        // hence we have to directly cast it and it should not be a problem because the FFI
        // function doesn't write in this pointer despite the parameter is a `*mut c_void`.
        // We believe that the parameter is `mut` because it's what _bindgen_ has unfairly
        // generated.
        let uc_res = unsafe {
            Guard::new(ulksys::uplink_part_upload_write(
                self.inner.part_upload,
                (buf.as_ptr() as *mut u8).cast(),
                buf.len(),
            ))
        };

        let written = uc_res.bytes_written as usize;
        if let Some(err) = uc_res.error() {
            // There is an error and the operation didn't upload any byte, so we return the error
            // directly.
            if written == 0 {
                return Err(std::io::Error::other(err));
            }

            self.failed_write = Some(uc_res);
        }

        Ok(written)
    }
}

/// Iterator over a collection of parts of a multipart upload operation.
pub struct PartIterator {
    /// The upload iterator type of the FFI that an instance of this struct represents and guards
    /// its lifetime until the instance drops.
    inner: Guard<*mut ulksys::UplinkPartIterator>,
}

impl PartIterator {
    /// Creates a new instance from the type exposed by the FFI.
    pub(crate) fn from_ffi_part_iterator(
        uc_iterator: Guard<*mut ulksys::UplinkPartIterator>,
    ) -> Self {
        assert!(
            uc_iterator.value().is_some(),
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
            if !ulksys::uplink_part_iterator_next(*self.inner) {
                let uc_error = Guard::new(ulksys::uplink_part_iterator_err(*self.inner));
                return uc_error.error().map(Err);
            }

            let uc_part = Guard::new(ulksys::uplink_part_iterator_item(*self.inner));
            Some(Ok(Part::with_ffi_part(uc_part.value().expect(
                "an iterator that indicated that there is a next element always returns it",
            ))))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::guard::test::{c_array, c_box, c_string};

    use std::ptr;

    #[test]
    fn test_info_from_ffi_upload_info_result() {
        let entries = [("key-a", "val-a"), ("key-b", "val-b")].map(|(key, value)| {
            ulksys::UplinkCustomMetadataEntry {
                key: c_string(key),
                key_length: key.len(),
                value: c_string(value),
                value_length: value.len(),
            }
        });
        // SAFETY: the pointers are valid and nothing else frees them.
        let uc_result = unsafe {
            Guard::new(ulksys::UplinkUploadInfoResult {
                info: c_box(ulksys::UplinkUploadInfo {
                    upload_id: c_string("upload-id"),
                    key: c_string("object-key"),
                    is_prefix: false,
                    system: ulksys::UplinkSystemMetadata {
                        created: 10,
                        expires: 20,
                        content_length: 30,
                    },
                    custom: ulksys::UplinkCustomMetadata {
                        entries: c_array(entries),
                        count: 2,
                    },
                }),
                error: ptr::null_mut(),
            })
        };

        let info = Info::from_ffi_upload_info_result(uc_result).expect("valid upload info");

        assert_eq!(info.upload_id, "upload-id", "upload ID");
        assert_eq!(info.key, "object-key", "key");
        assert!(!info.is_prefix, "is prefix");
        assert_eq!(
            info.metadata_system.created,
            time::Duration::from_secs(10),
            "system metadata: created"
        );
        assert_eq!(
            info.metadata_system.expires,
            Some(time::Duration::from_secs(20)),
            "system metadata: expires"
        );
        assert_eq!(
            info.metadata_system.content_length, 30,
            "system metadata: content length"
        );
        assert_eq!(info.metadata_custom.count(), 2, "custom metadata: count");
        for (key, value) in [("key-a", "val-a"), ("key-b", "val-b")] {
            assert_eq!(
                info.metadata_custom.get(key).map(String::as_str),
                Some(value),
                "custom metadata: {key}"
            );
        }
    }
}
