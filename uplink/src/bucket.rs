//! Storj DSC Bucket and related types.

use crate::uplink_c::Ensurer;
use crate::{Error, Result};

use std::ffi::{CStr, CString};
use std::time::Duration;

use uplink_sys as ulksys;

/// Contains information about a specific bucket.
#[derive(Debug)]
pub struct Bucket {
    /// Name of the bucket.
    pub name: String,
    /// Unix Epoch time when the bucket was created.
    pub created_at: Duration,
}

impl Bucket {
    /// Creates a Bucket instance from the type exposed by the FFI.
    ///
    /// It returns an [`Error:Internal`](crate::Error::Internal) if `uc_bucket`'s name invalid
    /// UTF-8.
    pub(crate) fn from_ffi_bucket(uc_bucket: *mut ulksys::UplinkBucket) -> Result<Self> {
        assert!(
            !uc_bucket.is_null(),
            "BUG: `uc_bucket` argument cannot be NULL"
        );

        let uc_bucket_ptr = uc_bucket;
        // SAFETY: We have checked just above that the pointer isn't NULL.
        let uc_bucket = unsafe { *uc_bucket_ptr };
        uc_bucket.ensure();

        let name;
        let created_at: Duration;
        // SAFETY: we have check that the `uc_bucket` doesn't have fields with NULL pointers through
        // the `ensure` method.
        unsafe {
            // User create buckets and satellites and/or client libraries or applications don't
            // likely allow to use invalid UTF-8 characters in their names. Nonetheless, we don't
            // panic if they contain some and we return an internal error because we see it's a
            // limitation of Rust and C interoperability and consumers of this crate would have a
            // chance to deal with them appropriately.
            let cs = CString::from(CStr::from_ptr(uc_bucket.name));
            name = cs.into_string().map_err(|err| {
                ulksys::uplink_free_bucket(uc_bucket_ptr);
                Error::new_internal(
                    "FFI returned an invalid bucket's name; it contains invalid UTF-8 characters",
                    err.into(),
                )
            })?;
            created_at = Duration::new(uc_bucket.created as u64, 0);
            ulksys::uplink_free_bucket(uc_bucket_ptr);
        }

        Ok(Bucket { name, created_at })
    }

    /// Creates a new instance from the FFI representation for a bucket's result.
    ///
    /// It returns the following errors:
    /// * an [`Error::new_uplink` constructor](crate::Error::new_uplink), if `uc_result` contains a
    ///   non `NULL` pointer in the `error` field.
    /// * an [`Error::Internal`](crate::Error::Internal) if `uc_result.bucket`'s name contains
    ///   invalid UTF-8 characters.
    pub(crate) fn from_ffi_bucket_result(uc_result: ulksys::UplinkBucketResult) -> Result<Self> {
        uc_result.ensure();

        if let Some(err) = Error::new_uplink(uc_result.error) {
            // SAFETY: we trust the FFI is safe freeing the memory of a valid pointer.
            unsafe { ulksys::uplink_free_bucket_result(uc_result) };
            return Err(err);
        }

        // At this point we don't need to free the `uc_result` because the following function free
        // the `info` pointer and the `error` pointer is `NULL`, and that's what the free function
        // for the `uc_result` does (i.e. call a free specific function for each pointer returning
        // without doing anything if it's `NULL`).
        Self::from_ffi_bucket(uc_result.bucket)
    }
}

/// Iterates over a collection of buckets.
pub struct Iterator {
    /// The bucket iterator type of the FFI that an instance of this struct represents and guards
    /// its lifetime until this instance drops.
    inner: *mut ulksys::UplinkBucketIterator,
}

impl Iterator {
    /// Creates a new instance from the type exposed by the FFI.
    pub(crate) fn from_ffi_bucket_iterator(uc_iterator: *mut ulksys::UplinkBucketIterator) -> Self {
        assert!(
            !uc_iterator.is_null(),
            "BUG: `uc_iterator` argument cannot be NULL"
        );

        Iterator { inner: uc_iterator }
    }
}

impl std::iter::Iterator for Iterator {
    type Item = Result<Bucket>;

    /// It returns an:
    ///
    /// * [`Error::Uplink`](crate::Error::Uplink) when FFI returns an error when retrieving the
    ///   item.
    /// * [`Error:Internal`](crate::Error::Internal) if `uc_bucket`'s name invalid UTF-8.
    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: we trust that the FFI functions don't panic when called with an instance returned
        // by them and they don't return invalid memory references or `null` if next returns `true`.
        unsafe {
            if !ulksys::uplink_bucket_iterator_next(self.inner) {
                let uc_error = ulksys::uplink_bucket_iterator_err(self.inner);
                return Error::new_uplink(uc_error).map(Err);
            }

            Some(Bucket::from_ffi_bucket(
                ulksys::uplink_bucket_iterator_item(self.inner),
            ))
        }
    }
}

impl Drop for Iterator {
    fn drop(&mut self) {
        // SAFETY: we trust that the FFI is safe freeing the memory of a correct
        // `UplinkBukcetIterator` value.
        unsafe {
            ulksys::uplink_free_bucket_iterator(self.inner);
        }
    }
}
