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

    /// Returns the underlying c-bindings representation of the options.
    /// The returned options' pointers are valid for as long as the `custom_metadata` shared
    /// reference received by [`Self::new`].
    ///
    /// It takes a mutable reference because [`metadata::Custom.to_uplink_c`] requires a mutable
    /// reference.
    pub(crate) fn to_uplink_c(&mut self) -> ulksys::UplinkCommitUploadOptions {
        ulksys::UplinkCommitUploadOptions {
            custom_metadata: self.custom_metadata.to_uplink_c(),
        }
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
    /// Returns the underlying c-bindings representation of the options.
    pub(crate) fn to_uplink_c(&self) -> ulksys::UplinkDownloadOptions {
        ulksys::UplinkDownloadOptions {
            offset: self.offset,
            length: self.length,
        }
    }
}

/// Options for listing buckets.
#[derive(Default)]
pub struct ListBuckets<'a> {
    /// It's the starting position of the iterator. The first item of the list
    /// is the one right after the cursor.
    cursor: &'a str,
    /// C representation of `cursor` for providing it to the underlying
    /// c-bindings and guards its lifetime until `self` gets dropped.
    inner_cursor: CString,
}

impl<'a> ListBuckets<'a> {
    /// Creates options for listing buckets with the specified cursor value.
    /// It returns an error if `cursor` contains any null byte (0 byte).
    pub fn with_cursor(cursor: &'a str) -> Result<Self> {
        let inner_cursor = helpers::cstring_from_str_fn_arg("cursor", cursor)?;
        Ok(Self {
            cursor,
            inner_cursor,
        })
    }

    /// Returns the underlying c-bindings representation of the options.
    /// The returned options' pointers are valid for as long as `self`.
    pub(crate) fn to_uplink_c(&self) -> ulksys::UplinkListBucketsOptions {
        ulksys::UplinkListBucketsOptions {
            cursor: self.inner_cursor.as_ptr(),
        }
    }
}

/// Options for listing objects.
#[derive(Default)]
pub struct ListObjects<'a> {
    /// Only list objects with this key prefix. When not empty, it must ends with slash.
    prefix: &'a str,
    /// C representation of `prefix` for providing it to the underlying c-bindings and guards its
    /// lifetime until `self` gets dropped.
    inner_prefix: CString,
    /// Specifies the starting position of the iterator by offsetting from the first object of the
    /// list.
    /// The first item of the list is the one after the cursor.
    /// The list of objects depends on the `prefix`.
    cursor: &'a str,
    /// C representation of `cursor` for providing it to the underlying c-bindings and guards its
    /// lifetime until `self` gets dropped.
    inner_cursor: CString,
    /// Iterate the objects without collapsing prefixes.
    pub recursive: bool,
    /// Include the "system metadata" associated with the objects.
    pub system: bool,
    /// Include the "custom metadata" associated with the objects.
    pub custom: bool,
}

impl<'a> ListObjects<'a> {
    /// Creates options of listing objects options with the specified prefix.
    ///
    /// `prefix` must:
    /// * not be empty.
    /// * end with '/'.
    /// * not contain any null byte (0 byte).
    pub fn with_prefix(prefix: &'a str) -> Result<Self> {
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
    pub fn with_cursor(cursor: &'a str) -> Result<Self> {
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
    pub fn with_prefix_and_cursor(prefix: &'a str, cursor: &'a str) -> Result<Self> {
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
    /// contain any null byte (0 byte), which are essential for working fine with the underlying
    /// c-bindings.
    ///
    /// This is a convenient constructor to be used by the public constructors which impose more
    /// contains  on `prefix` and `cursor`.
    fn new(prefix: &'a str, cursor: &'a str) -> Result<Self> {
        let inner_prefix = helpers::cstring_from_str_fn_arg("prefix", prefix)?;
        let inner_cursor = helpers::cstring_from_str_fn_arg("cursor", cursor)?;

        Ok(Self {
            prefix,
            inner_prefix,
            cursor,
            inner_cursor,
            ..Default::default()
        })
    }

    /// Returns the underlying c-binding representation of the options.
    /// The returned options' pointer are valid as long as `self`.
    pub(crate) fn to_uplink_c(&self) -> ulksys::UplinkListObjectsOptions {
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
#[derive(Default)]
pub struct ListUploads<'a> {
    /// Only list uncommitted uploads with this key prefix. When not empty, it must ends with slash.
    prefix: &'a str,
    /// C representation of `prefix` for providing it to the underlying
    /// c-bindings and guards its lifetime until `self` gets dropped.
    inner_prefix: CString,
    /// Specifies the starting position of the iterator by offsetting from the first object of the
    /// list.
    /// The first item of the list is the one after the cursor.
    /// The list of objects depends on the `prefix`.
    cursor: &'a str,
    /// C representation of `cursor` for providing it to the underlying c-bindings and guards its
    /// lifetime until `self` gets dropped.
    inner_cursor: CString,
    /// Iterate the objects without collapsing prefixes.
    pub recursive: bool,
    /// Include the "system metadata" associated with the objects.
    pub system: bool,
    /// Include the "custom metadata" associated with the objects.
    pub custom: bool,
}

impl<'a> ListUploads<'a> {
    /// Creates options of listing uncommitted uploads options with the specified prefix.
    ///
    /// `prefix` must:
    /// * not be empty.
    /// * end with '/'.
    /// * not contain any null byte (0 byte).
    pub fn with_prefix(prefix: &'a str) -> Result<Self> {
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
    pub fn with_cursor(cursor: &'a str) -> Result<Self> {
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
    pub fn with_prefix_and_cursor(prefix: &'a str, cursor: &'a str) -> Result<Self> {
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
    /// contain any null byte (0 byte), which are essential for working fine with the underlying
    /// c-bindings.
    ///
    /// This is a convenient constructor to be used by the public constructors which impose more
    /// contains  on `prefix` and `cursor`.
    fn new(prefix: &'a str, cursor: &'a str) -> Result<Self> {
        let inner_prefix = helpers::cstring_from_str_fn_arg("prefix", prefix)?;
        let inner_cursor = helpers::cstring_from_str_fn_arg("cursor", cursor)?;

        Ok(Self {
            prefix,
            inner_prefix,
            cursor,
            inner_cursor,
            ..Default::default()
        })
    }

    /// Returns the underlying c-binding representation of the options.
    /// The returned options' pointer are valid as long as `self`.
    pub(crate) fn to_uplink_c(&self) -> ulksys::UplinkListUploadsOptions {
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
    /// Returns the underlying c-binding representation of the options.
    pub(crate) fn to_uplink_c(&self) -> ulksys::UplinkListUploadPartsOptions {
        ulksys::UplinkListUploadPartsOptions {
            cursor: self.cursor,
        }
    }
}

/// Options for moving objects to a different bucket or/and key.
#[derive(Default)]
pub struct MoveObject {}

impl MoveObject {
    /// The returned options' pointer are valid as long as `self`.
    pub(crate) fn to_uplink_c(&self) -> ulksys::UplinkMoveObjectOptions {
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
    /// Returns the underlying c-bindings representation of the options.
    pub(crate) fn to_uplink_c(&self) -> ulksys::UplinkUploadOptions {
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
    /// Returns the underlying c-bindings representation of the options.
    pub(crate) fn to_uplink_c(&self) -> ulksys::UplinkUploadObjectMetadataOptions {
        ulksys::UplinkUploadObjectMetadataOptions {}
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_list_buckets_with_cursor() {
        // TODO: implement test for checking OK and error cases.
    }

    #[test]
    fn test_list_objects_with_prefix() {
        // TODO: implement test for checking OK and error cases.
    }

    #[test]
    fn test_list_objects_with_cursor() {
        // TODO: implement test for checking OK and error cases.
    }

    #[test]
    fn test_list_objects_with_prefix_and_cursor() {
        // TODO: implement test for checking OK and error cases.
    }

    #[test]
    fn test_list_uploads_with_prefix() {
        // TODO: implement test for checking OK and error cases.
    }

    #[test]
    fn test_list_uploads_with_cursor() {
        // TODO: implement test for checking OK and error cases.
    }

    #[test]
    fn test_list_uploads_with_prefix_and_cursor() {
        // TODO: implement test for checking OK and error cases.
    }
}
