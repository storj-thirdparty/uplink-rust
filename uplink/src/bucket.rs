//! Storj DSC Bucket and related types.

use crate::guard::Guard;
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
    /// Creates a new instance from the FFI representation.
    ///
    /// It returns an [`Error:Internal`](crate::Error::Internal) if `uc_bucket`'s name invalid
    /// UTF-8.
    pub(crate) fn with_ffi_bucket(uc_bucket: &ulksys::UplinkBucket) -> Result<Self> {
        uc_bucket.ensure();

        // SAFETY: we have check that the `uc_bucket` doesn't have fields with NULL pointers through
        // the `ensure` method.
        let name = CString::from(unsafe { CStr::from_ptr(uc_bucket.name) });
        // User create buckets and satellites and/or client libraries or applications don't likely
        // allow to use invalid UTF-8 characters in their names. Nonetheless, we don't panic if they
        // contain some and we return an internal error because we see it's a limitation of Rust and
        // C interoperability and consumers of this crate would have a chance to deal with them
        // appropriately.
        let name = name.into_string().map_err(|err| {
            Error::new_internal(
                "FFI returned an invalid bucket's name; it contains invalid UTF-8 characters",
                err.into(),
            )
        })?;

        Ok(Bucket {
            name,
            created_at: Duration::new(uc_bucket.created as u64, 0),
        })
    }

    /// Creates a new instance from the FFI representation for a bucket's result.
    ///
    /// It returns the following errors:
    /// * an [`Error::Uplink`](crate::Error::Uplink) if `uc_result` contains an error.
    /// * an [`Error::Internal`](crate::Error::Internal) if `uc_result.bucket`'s name contains
    ///   invalid UTF-8 characters.
    pub(crate) fn from_ffi_bucket_result(
        uc_result: Guard<ulksys::UplinkBucketResult>,
    ) -> Result<Self> {
        uc_result.ensure();
        uc_result.check()?;

        Self::with_ffi_bucket(
            uc_result
                .value()
                .expect("successful bucket result must always have a bucket"),
        )
    }
}

/// Iterates over a collection of buckets.
pub struct Iterator {
    /// The bucket iterator type of the FFI that an instance of this struct represents and guards
    /// its lifetime until this instance drops.
    inner: Guard<*mut ulksys::UplinkBucketIterator>,
}

impl Iterator {
    /// Creates a new instance from the type exposed by the FFI.
    pub(crate) fn from_ffi_bucket_iterator(
        uc_iterator: Guard<*mut ulksys::UplinkBucketIterator>,
    ) -> Self {
        assert!(
            uc_iterator.value().is_some(),
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
            if !ulksys::uplink_bucket_iterator_next(*self.inner) {
                let uc_error = Guard::new(ulksys::uplink_bucket_iterator_err(*self.inner));
                return uc_error.error().map(Err);
            }

            let uc_bucket = Guard::new(ulksys::uplink_bucket_iterator_item(*self.inner));
            Some(Bucket::with_ffi_bucket(uc_bucket.value().expect(
                "an iterator that indicated that there is a next element always returns it",
            )))
        }
    }
}
