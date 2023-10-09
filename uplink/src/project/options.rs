//! All the Storj DCS options types related to a Project.

use crate::{helpers, metadata::Custom, Error, Result};

use std::ffi::CString;
use std::time::Duration;

use uplink_sys as ulksys;

/// Options for committing a multipart upload.
pub struct CommitUpload<'a> {
    /// Custom metadata to assign to a multipart upload.
    custom_metadata: &'a mut Custom,
}

impl<'a> CommitUpload<'a> {
    /// Creates an instance of commit upload options.
    ///
    /// It's mutable because converting to a Uplink-C representation requires it.
    pub fn new(custom_metadata: &'a mut Custom) -> Self {
        Self { custom_metadata }
    }

    /// Returns the FFI representation of the options.
    ///
    /// It takes a mutable reference because [`metadata::Custom.to_ffi_c`] requires a mutable
    /// reference.
    #[allow(clippy::wrong_self_convention)]
    pub(crate) fn to_ffi_commit_upload_options(&mut self) -> ulksys::UplinkCommitUploadOptions {
        ulksys::UplinkCommitUploadOptions {
            custom_metadata: self.custom_metadata.to_ffi_custom_metadata(),
        }
    }
}

/// Options for copying objects to a different bucket or/and key without downloading and uploading
/// it.
#[derive(Default)]
pub struct CopyObject {}

impl CopyObject {
    /// Returns the FFI representation of the options.
    pub(crate) fn as_ffi_copy_object_options(&self) -> ulksys::UplinkCopyObjectOptions {
        ulksys::UplinkCopyObjectOptions {}
    }
}

/// Options for downloading an object.
#[derive(Default)]
pub struct Download {
    /// The initial point of the object's blob to download.
    /// If it's negative, it will start at the suffix of the blob but it's isn't supported to be
    /// negative with a positive `length`.
    pub offset: i64,
    /// The length of the blob starting from `offset` to download.
    /// If it's negative, it will read until the end of the blob.
    pub length: i64,
}

impl Download {
    /// Returns the FFI representation of the options.
    pub(crate) fn as_ffi_download_options(&self) -> ulksys::UplinkDownloadOptions {
        ulksys::UplinkDownloadOptions {
            offset: self.offset,
            length: self.length,
        }
    }
}

/// Options for listing buckets.
#[derive(Debug, Default)]
pub struct ListBuckets {
    /// C representation of `cursor` for providing it to the FFI and guards its lifetime until
    /// `self` gets dropped.
    inner_cursor: CString,
}

impl ListBuckets {
    /// Creates options for listing buckets with the specified cursor value.
    /// It returns an error if `cursor` contains any null byte (0 byte).
    pub fn with_cursor(cursor: &str) -> Result<Self> {
        let inner_cursor = helpers::cstring_from_str_fn_arg("cursor", cursor)?;
        Ok(Self { inner_cursor })
    }

    /// Returns the FFI representation of the options.
    pub(crate) fn as_ffi_list_buckets_options(&self) -> ulksys::UplinkListBucketsOptions {
        ulksys::UplinkListBucketsOptions {
            cursor: self.inner_cursor.as_ptr(),
        }
    }
}

/// Options for listing objects.
#[derive(Debug, Default)]
pub struct ListObjects {
    /// Only list objects with this key prefix. When not empty, it must ends with slash.
    ///
    /// C representation of `prefix` for providing it to the FFI and guards its lifetime until
    /// `self` gets dropped.
    inner_prefix: CString,
    /// Specifies the starting position of the iterator by offsetting from the first object of the
    /// list.
    /// The first item of the list is the one after the cursor.
    /// The list of objects depends on the `prefix`.
    ///
    /// C representation of `cursor` for providing it to the FFI and guards its lifetime until
    /// `self` gets dropped.
    inner_cursor: CString,
    /// Iterate the objects without collapsing prefixes.
    pub recursive: bool,
    /// Include the "system metadata" associated with the objects.
    pub system: bool,
    /// Include the "custom metadata" associated with the objects.
    pub custom: bool,
}

impl ListObjects {
    /// Creates options of listing objects options with the specified prefix.
    ///
    /// `prefix` must:
    /// * not be empty.
    /// * end with '/'.
    /// * not contain any null byte (0 byte).
    pub fn with_prefix(prefix: &str) -> Result<Self> {
        if !prefix.ends_with('/') {
            return Err(Error::new_invalid_arguments(
                "prefix",
                "cannot be empty and must end with '/'",
            ));
        }

        Self::new(prefix, "")
    }

    /// Creates options of listing objects options with the specified cursor.
    ///
    /// `cursor` must:
    /// * not be empty.
    /// * not contain any null byte (0 byte).
    pub fn with_cursor(cursor: &str) -> Result<Self> {
        if cursor.is_empty() {
            return Err(Error::new_invalid_arguments("cursor", "cannot be empty"));
        }

        Self::new("", cursor)
    }

    /// Creates options of listing objects options with the specified prefix and cursor.
    ///
    /// `prefix` and `cursor` must:
    /// * not be empty.
    /// * not contain any null byte (0 byte).
    ///
    /// `prefix` must also end with '/'.
    pub fn with_prefix_and_cursor(prefix: &str, cursor: &str) -> Result<Self> {
        if !prefix.ends_with('/') {
            return Err(Error::new_invalid_arguments(
                "prefix",
                "cannot be empty and must end with '/'",
            ));
        }

        if cursor.is_empty() {
            return Err(Error::new_invalid_arguments("cursor", "cannot be empty"));
        }

        Self::new(prefix, cursor)
    }

    /// Creates options for listing objects with only verifying that `prefix` and `cursor` don't
    /// contain any null byte (0 byte), which are essential for working fine with the FFI.
    ///
    /// This is a convenient constructor to be used by the public constructors which impose more
    /// contains  on `prefix` and `cursor`.
    fn new(prefix: &str, cursor: &str) -> Result<Self> {
        let inner_prefix = helpers::cstring_from_str_fn_arg("prefix", prefix)?;
        let inner_cursor = helpers::cstring_from_str_fn_arg("cursor", cursor)?;

        Ok(Self {
            inner_prefix,
            inner_cursor,
            ..Default::default()
        })
    }

    /// Returns the FFI representation of the options.
    pub(crate) fn as_ffi_list_objects_options(&self) -> ulksys::UplinkListObjectsOptions {
        ulksys::UplinkListObjectsOptions {
            prefix: self.inner_prefix.as_ptr(),
            cursor: self.inner_cursor.as_ptr(),
            recursive: self.recursive,
            system: self.system,
            custom: self.custom,
        }
    }
}

/// Options for listing uncommitted uploads.
#[derive(Debug, Default)]
pub struct ListUploads {
    /// Only list uncommitted uploads with this key prefix. When not empty, it must ends with slash.
    ///
    /// C representation of `prefix` for providing it to the FFI and guards its lifetime until
    /// `self` gets dropped.
    inner_prefix: CString,
    /// Specifies the starting position of the iterator by offsetting from the first object of the
    /// list.
    /// The first item of the list is the one after the cursor.
    /// The list of objects depends on the `prefix`.
    ///
    /// C representation of `cursor` for providing it to the FFI and guards its lifetime until
    /// `self` gets dropped.
    inner_cursor: CString,
    /// Iterate the objects without collapsing prefixes.
    pub recursive: bool,
    /// Include the "system metadata" associated with the objects.
    pub system: bool,
    /// Include the "custom metadata" associated with the objects.
    pub custom: bool,
}

impl ListUploads {
    /// Creates options of listing uncommitted uploads options with the specified prefix.
    ///
    /// `prefix` must:
    /// * not be empty.
    /// * end with '/'.
    /// * not contain any null byte (0 byte).
    pub fn with_prefix(prefix: &str) -> Result<Self> {
        if !prefix.ends_with('/') {
            return Err(Error::new_invalid_arguments(
                "prefix",
                "cannot be empty and must end with '/'",
            ));
        }

        Self::new(prefix, "")
    }

    /// Creates options of listing uncommitted uploads options with the specified cursor.
    ///
    /// `cursor` must:
    /// * not be empty.
    /// * not contain any null byte (0 byte).
    pub fn with_cursor(cursor: &str) -> Result<Self> {
        if cursor.is_empty() {
            return Err(Error::new_invalid_arguments("cursor", "cannot be empty"));
        }

        Self::new("", cursor)
    }

    /// Creates options of listing uncommitted options with the specified prefix and cursor.
    ///
    /// `prefix` and `cursor` must:
    /// * not be empty.
    /// * not contain any null byte (0 byte).
    ///
    /// `prefix` must also end with '/'.
    pub fn with_prefix_and_cursor(prefix: &str, cursor: &str) -> Result<Self> {
        if !prefix.ends_with('/') {
            return Err(Error::new_invalid_arguments(
                "prefix",
                "cannot be empty and must end with '/'",
            ));
        }

        if cursor.is_empty() {
            return Err(Error::new_invalid_arguments("cursor", "cannot be empty"));
        }

        Self::new(prefix, cursor)
    }

    /// Creates options for listing uncommitted with only verify that `prefix` and `cursor` don't
    /// contain any null byte (0 byte), which are essential for working fine with the FFI.
    ///
    /// This is a convenient constructor to be used by the public constructors which impose more
    /// contains  on `prefix` and `cursor`.
    fn new(prefix: &str, cursor: &str) -> Result<Self> {
        let inner_prefix = helpers::cstring_from_str_fn_arg("prefix", prefix)?;
        let inner_cursor = helpers::cstring_from_str_fn_arg("cursor", cursor)?;

        Ok(Self {
            inner_prefix,
            inner_cursor,
            ..Default::default()
        })
    }

    /// Returns the FFI representation of the options.
    pub(crate) fn as_ffi_list_uploads_options(&self) -> ulksys::UplinkListUploadsOptions {
        ulksys::UplinkListUploadsOptions {
            prefix: self.inner_prefix.as_ptr(),
            cursor: self.inner_cursor.as_ptr(),
            recursive: self.recursive,
            system: self.system,
            custom: self.custom,
        }
    }
}

/// Options for listing uploads parts.
#[derive(Default)]
pub struct ListUploadParts {
    /// Specifies the starting position of the iterator by offsetting from the first object of the
    /// list.
    ///
    /// The first item of the list is the one after the cursor.
    /// Parts start with index 1.
    pub cursor: u32,
}

impl ListUploadParts {
    /// Returns the FFI representation of the options.
    pub(crate) fn as_ffi_list_upload_parts_options(&self) -> ulksys::UplinkListUploadPartsOptions {
        ulksys::UplinkListUploadPartsOptions {
            cursor: self.cursor,
        }
    }
}

/// Options for moving objects to a different bucket or/and key.
#[derive(Default)]
pub struct MoveObject {}

impl MoveObject {
    /// Returns the FFI representation of the options.
    pub(crate) fn as_ffi_move_object_options(&self) -> ulksys::UplinkMoveObjectOptions {
        ulksys::UplinkMoveObjectOptions {}
    }
}

/// Options for uploading objects.
#[derive(Default)]
pub struct Upload {
    /// Determine when the object expires.
    ///
    /// The time is measured with the number of seconds since the Unix Epoch time. 0 is never and
    /// it's the same as `None`.
    pub expires: Option<Duration>,
}

impl Upload {
    /// Returns the FFI representation of the options.
    pub(crate) fn as_ffi_upload_options(&self) -> ulksys::UplinkUploadOptions {
        let expires = self.expires.unwrap_or(Duration::ZERO);

        ulksys::UplinkUploadOptions {
            expires: expires.as_secs() as i64,
        }
    }
}

/// Options for updating object's metadata.
///
/// Reserved for future use.
#[derive(Default)]
pub struct UploadObjectMetadata {}

impl UploadObjectMetadata {
    /// Returns the FFI representation of the options.
    pub(crate) fn as_ffi_upload_object_metadata_options(
        &self,
    ) -> ulksys::UplinkUploadObjectMetadataOptions {
        ulksys::UplinkUploadObjectMetadataOptions {}
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::error;

    #[test]
    fn test_list_buckets_with_cursor() {
        {
            // OK.
            let cursor = "some-cursor-id";
            let lo = ListBuckets::with_cursor(cursor)
                .expect("no error with a string without the NULL character");
            assert_eq!(cursor, lo.inner_cursor.to_str().unwrap(), "cursor value");
        }
        {
            // Error: invalid cursor value.
            if let Error::InvalidArguments(error::Args { names, msg }) =
                ListBuckets::with_cursor("cursor\0id")
                    .expect_err("when passing a cursor value with NULL bytes")
            {
                assert_eq!(names, "cursor", "invalid error argument name");
                assert_eq!(
                    msg, "cannot contains null bytes (0 byte). Null byte found at 6",
                    "invalid error argument message"
                );
            } else {
                panic!("expected an invalid argument error");
            }
        }
    }

    #[test]
    fn test_list_objects_with_prefix() {
        {
            // OK.
            let prefix = "a/b/";
            let lo = ListObjects::with_prefix(prefix)
                .expect("no error with a string without the NULL character and ending with '/'");
            assert_eq!(prefix, lo.inner_prefix.to_str().unwrap(), "prefix value");
        }
        {
            // Error: prefix doesn't end with `/`.
            if let Error::InvalidArguments(error::Args { names, msg }) =
                ListObjects::with_prefix("a/b")
                    .expect_err("when passing a prefix value without ending with '/'")
            {
                assert_eq!(names, "prefix", "invalid error argument name");
                assert_eq!(
                    msg, "cannot be empty and must end with '/'",
                    "invalid error argument message"
                );
            } else {
                panic!("expected an invalid argument error");
            }
        }
        {
            // Error: invalid prefix value.
            if let Error::InvalidArguments(error::Args { names, msg }) =
                ListObjects::with_prefix("a/b/\0/c/")
                    .expect_err("when passing a prefix value with NULL bytes")
            {
                assert_eq!(names, "prefix", "invalid error argument name");
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
    fn test_list_objects_with_cursor() {
        {
            // OK.
            let cursor = "some-cursor-id";
            let lb = ListObjects::with_cursor(cursor)
                .expect("no error with a string without the NULL character");
            assert_eq!(cursor, lb.inner_cursor.to_str().unwrap(), "cursor value");
        }
        {
            // Error: invalid cursor value.
            if let Error::InvalidArguments(error::Args { names, msg }) =
                ListObjects::with_cursor("cursor\0id")
                    .expect_err("when passing a cursor value with NULL bytes")
            {
                assert_eq!(names, "cursor", "invalid error argument name");
                assert_eq!(
                    msg, "cannot contains null bytes (0 byte). Null byte found at 6",
                    "invalid error argument message"
                );
            } else {
                panic!("expected an invalid argument error");
            }
        }
    }

    #[test]
    fn test_list_objects_with_prefix_and_cursor() {
        {
            // OK.
            let prefix = "a/b/";
            let cursor = "cursor-id";
            let lo = ListObjects::with_prefix_and_cursor(prefix, cursor)
                .expect("no error with a valid prefix and cursor");
            assert_eq!(prefix, lo.inner_prefix.to_str().unwrap(), "prefix value");
            assert_eq!(cursor, lo.inner_cursor.to_str().unwrap(), "cursor value");
        }
        {
            // Error: prefix doesn't end with `/`.
            if let Error::InvalidArguments(error::Args { names, msg }) =
                ListObjects::with_prefix_and_cursor("a/b", "cursor-id")
                    .expect_err("when passing a prefix value without ending with '/'")
            {
                assert_eq!(names, "prefix", "invalid error argument name");
                assert_eq!(
                    msg, "cannot be empty and must end with '/'",
                    "invalid error argument message"
                );
            } else {
                panic!("expected an invalid argument error");
            }
        }
        {
            // Error: invalid prefix value.
            if let Error::InvalidArguments(error::Args { names, msg }) =
                ListObjects::with_prefix_and_cursor("a/b/\0/c/", "cursor-id")
                    .expect_err("when passing a cursor value with NULL bytes")
            {
                assert_eq!(names, "prefix", "invalid error argument name");
                assert_eq!(
                    msg, "cannot contains null bytes (0 byte). Null byte found at 4",
                    "invalid error argument message"
                );
            } else {
                panic!("expected an invalid argument error");
            }
        }
        {
            // Error: invalid cursor value.
            if let Error::InvalidArguments(error::Args { names, msg }) =
                ListObjects::with_prefix_and_cursor("a/b/", "cursor\0id")
                    .expect_err("when passing a cursor value with NULL bytes")
            {
                assert_eq!(names, "cursor", "invalid error argument name");
                assert_eq!(
                    msg, "cannot contains null bytes (0 byte). Null byte found at 6",
                    "invalid error argument message"
                );
            } else {
                panic!("expected an invalid argument error");
            }
        }
    }

    #[test]
    fn test_list_uploads_with_prefix() {
        {
            // OK.
            let prefix = "a/b/";
            let lu = ListUploads::with_prefix(prefix)
                .expect("no error with a string without the NULL character and ending with '/'");
            assert_eq!(prefix, lu.inner_prefix.to_str().unwrap(), "prefix value");
        }
        {
            // Error: prefix doesn't end with `/`.
            if let Error::InvalidArguments(error::Args { names, msg }) =
                ListUploads::with_prefix("a/b")
                    .expect_err("when passing a prefix value without ending with '/'")
            {
                assert_eq!(names, "prefix", "invalid error argument name");
                assert_eq!(
                    msg, "cannot be empty and must end with '/'",
                    "invalid error argument message"
                );
            } else {
                panic!("expected an invalid argument error");
            }
        }
        {
            // Error: invalid prefix value.
            if let Error::InvalidArguments(error::Args { names, msg }) =
                ListUploads::with_prefix("a/b/\0/c/")
                    .expect_err("when passing a prefix value with NULL bytes")
            {
                assert_eq!(names, "prefix", "invalid error argument name");
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
    fn test_list_uploads_with_cursor() {
        {
            // OK.
            let cursor = "some-cursor-id";
            let lu = ListUploads::with_cursor(cursor)
                .expect("no error with a string without the NULL character");
            assert_eq!(cursor, lu.inner_cursor.to_str().unwrap(), "cursor value");
        }
        {
            // Error: invalid cursor value.
            if let Error::InvalidArguments(error::Args { names, msg }) =
                ListUploads::with_cursor("cursor\0id")
                    .expect_err("when passing a cursor value with NULL bytes")
            {
                assert_eq!(names, "cursor", "invalid error argument name");
                assert_eq!(
                    msg, "cannot contains null bytes (0 byte). Null byte found at 6",
                    "invalid error argument message"
                );
            } else {
                panic!("expected an invalid argument error");
            }
        }
    }

    #[test]
    fn test_list_uploads_with_prefix_and_cursor() {
        {
            // OK.
            let prefix = "a/b/";
            let cursor = "cursor-id";
            let lu = ListUploads::with_prefix_and_cursor(prefix, cursor)
                .expect("no error with a valid prefix and cursor");
            assert_eq!(prefix, lu.inner_prefix.to_str().unwrap(), "prefix value");
            assert_eq!(cursor, lu.inner_cursor.to_str().unwrap(), "cursor value");
        }
        {
            // Error: prefix doesn't end with `/`.
            if let Error::InvalidArguments(error::Args { names, msg }) =
                ListUploads::with_prefix_and_cursor("a/b", "cursor-id")
                    .expect_err("when passing a prefix value without ending with '/'")
            {
                assert_eq!(names, "prefix", "invalid error argument name");
                assert_eq!(
                    msg, "cannot be empty and must end with '/'",
                    "invalid error argument message"
                );
            } else {
                panic!("expected an invalid argument error");
            }
        }
        {
            // Error: invalid prefix value.
            if let Error::InvalidArguments(error::Args { names, msg }) =
                ListUploads::with_prefix_and_cursor("a/b/\0/c/", "cursor-id")
                    .expect_err("when passing a cursor value with NULL bytes")
            {
                assert_eq!(names, "prefix", "invalid error argument name");
                assert_eq!(
                    msg, "cannot contains null bytes (0 byte). Null byte found at 4",
                    "invalid error argument message"
                );
            } else {
                panic!("expected an invalid argument error");
            }
        }
        {
            // Error: invalid cursor value.
            if let Error::InvalidArguments(error::Args { names, msg }) =
                ListUploads::with_prefix_and_cursor("a/b/", "cursor\0id")
                    .expect_err("when passing a cursor value with NULL bytes")
            {
                assert_eq!(names, "cursor", "invalid error argument name");
                assert_eq!(
                    msg, "cannot contains null bytes (0 byte). Null byte found at 6",
                    "invalid error argument message"
                );
            } else {
                panic!("expected an invalid argument error");
            }
        }
    }
}
