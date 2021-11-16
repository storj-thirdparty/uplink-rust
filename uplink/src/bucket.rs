//! Storj DSC Bucket and related types.

use crate::{Ensurer, Error, Result};

use std::ffi::CStr;
use std::time::Duration;

use uplink_sys as ulksys;

/// Contains information about a specific bucket.
pub struct Bucket<'a> {
    /// The bucket type of the underlying c-bindings Rust crate that an instance
    /// of this struct represents and guard its life time until this instance
    /// drops.
    inner: *mut ulksys::UplinkBucket,

    /// Name of the bucket.
    pub name: &'a str,
    /// Unix Epoch time when the bucket was created.
    pub created_at: Duration,
}

impl<'a> Bucket<'a> {
    /// Creates a Bucket instance from the type exposed by the uplink
    /// c-bindings.
    ///
    /// The returned Bucket owns the address of the passed pointer, hence the
    /// caller should not use that pointer after this call nor free it because
    /// the returned Bucket will free it when it is dropped.
    pub(crate) fn from_uplink_c(uc_bucket: *mut ulksys::UplinkBucket) -> Result<Self> {
        if uc_bucket.is_null() {
            return Err(Error::new_invalid_arguments("uc_bucket", "cannot be null"));
        }

        let name: &str;
        let created_at: Duration;
        // SAFETY: uc_bucket cannot be null because it's checked at the
        // beginning of the function and we ensure uc_bucket doesn't have fields
        // with NULL pointers through the ensure method of the implemented
        // Ensurer trait.
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
        }

        Ok(Bucket {
            inner: uc_bucket,
            name,
            created_at,
        })
    }
}

impl<'a> Drop for Bucket<'a> {
    fn drop(&mut self) {
        // SAFETY: we trust that the underlying c-binding is safe freeing the
        // memory of a correct UplinkBucket value.
        unsafe { ulksys::uplink_free_bucket(self.inner) }
    }
}

/// Iterates over a collection of buckets.
pub struct Iterator {
    /// They bucket iterator type of the underlying c-bindings Rust crate that
    /// an instance of this struct represents and guard its life time until this
    /// instance drops.
    inner: *mut ulksys::UplinkBucketIterator,
}

impl Iterator {
    /// Creates a buckets Iterator instance from type exposed by the unlink
    /// c-bindings.
    pub(crate) fn from_uplink_c(uc_iterator: *mut ulksys::UplinkBucketIterator) -> Result<Self> {
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
    type Item = Result<Bucket<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
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
        // SAFETY: we trust that the underlying c-binding is safe freeing the
        // memory of a correct UplinkBukcetIterator value.
        unsafe {
            ulksys::uplink_free_bucket_iterator(self.inner);
        }
    }
}

impl Ensurer for ulksys::UplinkBucket {
    fn ensure(&self) -> &Self {
        assert!(
            !self.name.is_null(),
            "invalid underlying c-binding returned invalid UplinkBucket; name field is NULL"
        );
        self
    }
}
