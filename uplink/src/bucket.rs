//! Storj DSC Bucket and related types.

use crate::{Ensurer, Error, Result};

use std::ffi::CStr;
use std::time::Duration;

use uplink_sys as ulksys;

/// Contains information about a specific bucket.
pub struct Bucket<'a> {
    /// Name of the bucket.
    pub name: &'a str,
    /// Unix Epoch time when the bucket was created.
    pub created_at: Duration,
}

impl<'a> Bucket<'a> {
    /// Creates a Bucket instance from the type exposed by the uplink c-bindings.
    ///
    /// The caller can free the `uc_obj` after this call without affecting the returned value.
    ///
    /// It panics if `uc_part` is NULL.
    pub(crate) fn from_uplink_c(uc_bucket: *mut ulksys::UplinkBucket) -> Result<Self> {
        assert!(
            !uc_bucket.is_null(),
            "BUG: `uc_bucket` argument cannot be NULL"
        );

        let name: &str;
        let created_at: Duration;
        // SAFETY: we check before this block that pointer isn't NULL and inside of this block we
        // ensure that `uc_bucket` doesn't have fields with NULL pointers through the `ensure`
        // method of the implemented `Ensurer` trait, and we also trust the underlying c-binding is
        // safe freeing the memory.
        unsafe {
            (*uc_bucket).ensure();
            match CStr::from_ptr((*uc_bucket).name).to_str() {
                Ok(n) => name = n,
                Err(err) => {
                    return Err(Error::new_internal_with_inner(
                        "invalid bucket name because it contains invalid UTF-8 characters",
                        err.into(),
                    ));
                }
            };
            created_at = Duration::new((*uc_bucket).created as u64, 0);
            ulksys::uplink_free_bucket(uc_bucket)
        }

        Ok(Bucket { name, created_at })
    }
}

/// Iterates over a collection of buckets.
pub struct Iterator {
    /// The bucket iterator type of the underlying c-bindings Rust crate that an instance of this
    /// struct represents and guards its life time until this instance drops.
    inner: *mut ulksys::UplinkBucketIterator,
}

impl Iterator {
    /// Creates a new instance from the type exposed by the uplink c-bindings.
    ///
    /// It panics if `uc_iterator` is NULL.
    pub(crate) fn from_uplink_c(uc_iterator: *mut ulksys::UplinkBucketIterator) -> Self {
        assert!(
            !uc_iterator.is_null(),
            "BUG: `uc_iterator` argument cannot be NULL"
        );

        Iterator { inner: uc_iterator }
    }
}

impl<'a> std::iter::Iterator for &'a Iterator {
    type Item = Result<Bucket<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: we trust that the underlying c-bindings functions don't panic when called with
        // an instance returned by them and they don't return invalid memory references or `null`
        // if next returns `true`.
        unsafe {
            if !ulksys::uplink_bucket_iterator_next(self.inner) {
                let uc_error = ulksys::uplink_bucket_iterator_err(self.inner);
                return Error::new_uplink(uc_error).map(Err);
            }

            Some(Bucket::from_uplink_c(ulksys::uplink_bucket_iterator_item(
                self.inner,
            )))
        }
    }
}

impl Drop for Iterator {
    fn drop(&mut self) {
        // SAFETY: we trust that the underlying c-binding is safe freeing the memory of a correct
        // `UplinkBukcetIterator` value.
        unsafe {
            ulksys::uplink_free_bucket_iterator(self.inner);
        }
    }
}

impl Ensurer for ulksys::UplinkBucket {
    fn ensure(&self) -> &Self {
        assert!(
            !self.name.is_null(),
            "underlying c-binding returned an invalid UplinkBucket; name field is NULL"
        );
        self
    }
}
