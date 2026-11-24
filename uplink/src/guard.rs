//! Ownership of the values returned by the FFI.

use crate::{Error, Result};

use std::mem::ManuallyDrop;
use std::ops::Deref;

use uplink_sys as ulksys;

/// Owns a value returned by the FFI and frees it when it's dropped.
#[derive(Debug)]
pub(crate) struct Guard<T: Free>(ManuallyDrop<T>);

impl<T: Free> Guard<T> {
    /// Takes ownership of `value`.
    ///
    /// # Safety
    ///
    /// `value` must have been returned by the FFI, its pointers must be valid or `NULL`, and nothing
    /// else must free it.
    pub(crate) unsafe fn new(value: T) -> Self {
        Self(ManuallyDrop::new(value))
    }
}

impl<T: Owns> Guard<T> {
    /// Returns the owned value, or `None` if it's `NULL`.
    pub(crate) fn value(&self) -> Option<&T::Value> {
        // SAFETY: the pointer is valid or `NULL`, see `new`, and it isn't freed while `self` is
        // borrowed.
        unsafe { self.0.value_ptr().as_ref() }
    }
}

impl<T: Fallible> Guard<T> {
    /// Returns the error, or `None` if there isn't any.
    pub(crate) fn error(&self) -> Option<Error> {
        // SAFETY: the pointer is valid or `NULL`, see `new`, and it isn't freed while `self` is
        // borrowed.
        unsafe { self.0.error_ptr().as_ref() }.map(Error::with_ffi_error)
    }

    /// Returns an `Err` if there is an error.
    pub(crate) fn check(&self) -> Result<()> {
        self.error().map_or(Ok(()), Err)
    }
}

impl<T: Free> Deref for Guard<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T: Free> Drop for Guard<T> {
    fn drop(&mut self) {
        // SAFETY: `self` owns the value and it isn't used after this call.
        unsafe { ManuallyDrop::take(&mut self.0).free() }
    }
}

/// An FFI value that must be freed.
///
/// # Safety
///
/// [`Self::free`] must release everything that the value owns, and it must accept `NULL` pointers.
pub(crate) unsafe trait Free {
    /// Frees the value.
    ///
    /// # Safety
    ///
    /// The value must have been returned by the FFI and it must not be used after this call.
    unsafe fn free(self);
}

/// An FFI value that owns another one through a pointer.
///
/// # Safety
///
/// The pointer must be one that [`Free::free`] frees.
pub(crate) unsafe trait Owns: Free {
    /// The type of the owned value.
    type Value;

    /// Returns the pointer to the owned value.
    fn value_ptr(&self) -> *const Self::Value;
}

/// An FFI value that may have an error.
///
/// # Safety
///
/// The pointer must be one that [`Free::free`] frees.
pub(crate) unsafe trait Fallible: Free {
    /// Returns the pointer to the error, which is `NULL` when there isn't any.
    fn error_ptr(&self) -> *const ulksys::UplinkError;
}

/// Implements [`Free`] for each type with its FFI free function.
macro_rules! free {
    ($($type:ty => $free:ident,)*) => {$(
        // SAFETY: it's the function that the FFI offers for freeing this type.
        unsafe impl Free for $type {
            unsafe fn free(self) {
                ulksys::$free(self)
            }
        }
    )*};
}

/// Implements [`Free`], [`Fallible`], and, for the ones with a value, [`Owns`] for the FFI results.
macro_rules! results {
    ($($type:ident $({ $field:ident: $value:ident })? => $free:ident,)*) => {
        free! { $(ulksys::$type => $free,)* }
        $(
            // SAFETY: the FFI free functions of the results free their errors.
            unsafe impl Fallible for ulksys::$type {
                fn error_ptr(&self) -> *const ulksys::UplinkError {
                    self.error
                }
            }

            $(
                // SAFETY: the FFI free functions of the results free their values.
                unsafe impl Owns for ulksys::$type {
                    type Value = ulksys::$value;

                    fn value_ptr(&self) -> *const ulksys::$value {
                        self.$field
                    }
                }
            )?
        )*
    };
}

/// Implements [`Free`] and [`Owns`] for the pointers that the FFI returns.
macro_rules! pointers {
    ($($type:ident => $free:ident,)*) => {
        free! { $(*mut ulksys::$type => $free,)* }
        $(
            // SAFETY: the pointer is the one that is freed.
            unsafe impl Owns for *mut ulksys::$type {
                type Value = ulksys::$type;

                fn value_ptr(&self) -> *const ulksys::$type {
                    *self
                }
            }
        )*
    };
}

results! {
    EdgeCredentialsResult { credentials: EdgeCredentials } => edge_free_credentials_result,
    UplinkAccessResult { access: UplinkAccess } => uplink_free_access_result,
    UplinkBucketResult { bucket: UplinkBucket } => uplink_free_bucket_result,
    UplinkCommitUploadResult { object: UplinkObject } => uplink_free_commit_upload_result,
    UplinkDownloadResult { download: UplinkDownload } => uplink_free_download_result,
    UplinkEncryptionKeyResult { encryption_key: UplinkEncryptionKey } => uplink_free_encryption_key_result,
    UplinkObjectResult { object: UplinkObject } => uplink_free_object_result,
    UplinkPartResult { part: UplinkPart } => uplink_free_part_result,
    UplinkPartUploadResult { part_upload: UplinkPartUpload } => uplink_free_part_upload_result,
    UplinkProjectResult { project: UplinkProject } => uplink_free_project_result,
    UplinkReadResult => uplink_free_read_result,
    UplinkStringResult => uplink_free_string_result,
    UplinkUploadInfoResult { info: UplinkUploadInfo } => uplink_free_upload_info_result,
    UplinkUploadResult { upload: UplinkUpload } => uplink_free_upload_result,
    UplinkWriteResult => uplink_free_write_result,
}

pointers! {
    UplinkBucket => uplink_free_bucket,
    UplinkBucketIterator => uplink_free_bucket_iterator,
    UplinkError => uplink_free_error,
    UplinkObject => uplink_free_object,
    UplinkObjectIterator => uplink_free_object_iterator,
    UplinkPart => uplink_free_part,
    UplinkPartIterator => uplink_free_part_iterator,
    UplinkUploadInfo => uplink_free_upload_info,
    UplinkUploadIterator => uplink_free_upload_iterator,
}

// SAFETY: the pointer is the one that is freed.
unsafe impl Fallible for *mut ulksys::UplinkError {
    fn error_ptr(&self) -> *const ulksys::UplinkError {
        *self
    }
}

#[cfg(test)]
pub(crate) mod test {
    use super::*;

    use crate::error;

    use std::cell::Cell;
    use std::ffi::{c_void, CString};
    use std::os::raw::c_char;
    use std::ptr;

    // The FFI frees its values with the C allocator, so the values that the tests create must be
    // allocated with it.
    extern "C" {
        fn calloc(nitems: usize, size: usize) -> *mut c_void;
        fn strdup(s: *const c_char) -> *mut c_char;
    }

    /// Returns a pointer to a copy of `vals` allocated with the C allocator.
    pub(crate) fn c_array<T, const N: usize>(vals: [T; N]) -> *mut T {
        // SAFETY: the returned pointer, when it isn't `NULL`, has enough room for the values.
        unsafe {
            let ptr = calloc(N, std::mem::size_of::<T>()).cast::<T>();
            assert!(!ptr.is_null(), "C allocation failed");
            for (i, val) in vals.into_iter().enumerate() {
                ptr.add(i).write(val);
            }
            ptr
        }
    }

    /// Returns a pointer to `val` allocated with the C allocator.
    pub(crate) fn c_box<T>(val: T) -> *mut T {
        c_array([val])
    }

    /// Returns a copy of `val` allocated with the C allocator.
    pub(crate) fn c_string(val: &str) -> *mut c_char {
        let val = CString::new(val).expect("a string without NULL bytes");
        // SAFETY: `val` is a valid C string.
        let ptr = unsafe { strdup(val.as_ptr()) };
        assert!(!ptr.is_null(), "C allocation failed");
        ptr
    }

    /// Returns an FFI error allocated with the C allocator.
    pub(crate) fn c_error(code: u32) -> *mut ulksys::UplinkError {
        c_box(ulksys::UplinkError {
            code: code as i32,
            message: c_string("a message"),
        })
    }

    /// Counts the number of times that it's freed.
    struct Counter<'a>(&'a Cell<usize>);

    // SAFETY: it doesn't own anything.
    unsafe impl Free for Counter<'_> {
        unsafe fn free(self) {
            self.0.set(self.0.get() + 1);
        }
    }

    #[test]
    fn test_guard_frees_once() {
        let count = Cell::new(0);
        // SAFETY: the value doesn't have any pointer.
        let guard = unsafe { Guard::new(Counter(&count)) };
        let moved = guard;
        assert_eq!(count.get(), 0, "before drop");

        drop(moved);
        assert_eq!(count.get(), 1, "after drop");
    }

    #[test]
    fn test_guard_null() {
        // SAFETY: the pointers are `NULL`.
        let guard = unsafe { Guard::new(ptr::null_mut::<ulksys::UplinkError>()) };
        assert!(guard.value().is_none(), "value");
        assert!(guard.error().is_none(), "error");
        guard.check().expect("there isn't an error");

        // SAFETY: the pointers are `NULL`.
        let guard = unsafe {
            Guard::new(ulksys::UplinkObjectResult {
                object: ptr::null_mut(),
                error: ptr::null_mut(),
            })
        };
        assert!(guard.value().is_none(), "value");
        assert!(guard.error().is_none(), "error");
        guard.check().expect("there isn't an error");
    }

    #[test]
    fn test_guard_error() {
        let is_expected = |err| matches!(err, Error::Uplink(error::Uplink::BucketNotFound(msg)) if msg == "a message");

        // SAFETY: the pointers are valid and nothing else frees them.
        let guard = unsafe { Guard::new(c_error(ulksys::UPLINK_ERROR_BUCKET_NOT_FOUND)) };
        assert!(is_expected(guard.check().expect_err("pointer")));
        // The error is a copy, so it can be retrieved more than once.
        assert!(is_expected(guard.check().expect_err("pointer again")));

        // SAFETY: the pointers are valid and nothing else frees them.
        let guard = unsafe {
            Guard::new(ulksys::UplinkReadResult {
                bytes_read: 0,
                error: c_error(ulksys::UPLINK_ERROR_BUCKET_NOT_FOUND),
            })
        };
        let err = guard.check().expect_err("result");
        drop(guard);
        assert!(is_expected(err));
    }
}
