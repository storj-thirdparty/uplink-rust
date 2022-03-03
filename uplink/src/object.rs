//! Storj DCS Object and related types.

use crate::{error::BoxError, metadata, Ensurer, Error, Result};

use std::ffi::CStr;

use uplink_sys as ulksys;

/// Contains information about an object.
pub struct Info<'a> {
    /// The identifier of the object inside of the bucket which it belongs.
    pub key: &'a str,
    /// Indicates if the key is a prefix for other objects.
    pub is_prefix: bool,
    /// The system metadata associated with the object.
    pub metadata_system: metadata::System,
    /// The custom metadata associated with the object.
    pub metadata_custom: metadata::Custom,
}

impl Info<'_> {
    /// Creates new instance from the underlying c-binding representation.
    ///
    /// It returns an error if `uc_obj` contains a key with invalid UTF-8 characters or
    /// [`metadata::Custom::from_uplink_c`] return an error.
    ///
    /// It consumes `uc_obj` hence the pointer isn't valid anymore after calling this method.
    pub(crate) fn from_uplink_c(uc_obj: *mut ulksys::UplinkObject) -> Result<Self> {
        if uc_obj.is_null() {
            return Err(Error::new_invalid_arguments("uc_obj", "cannot be null"));
        }

        let key: &str;
        let is_prefix: bool;
        let metadata_system: metadata::System;
        let metadata_custom: metadata::Custom;
        // SAFETY: we check before this block that pointer isn't NULL and inside of this block we
        // ensure that `uc_obj` doesn't have fields with NULL pointers through the `ensure` method
        // of the implemented `Ensurer` trait, and we also trust the underlying c-binding is safe
        // freeing the memory.
        unsafe {
            (*uc_obj).ensure();
            key = CStr::from_ptr((*uc_obj).key).to_str().map_err(|err| {
                Error::new_internal_with_inner(
                    "underlying-c binding returned an invalid object's key",
                    BoxError::from(err),
                )
            })?;
            metadata_custom = metadata::Custom::from_uplink_c(&(*uc_obj).custom)?;
            metadata_system = metadata::System::from_uplink_c(&(*uc_obj).system);
            is_prefix = (*uc_obj).is_prefix;
        }

        Ok(Self {
            key,
            is_prefix,
            metadata_system,
            metadata_custom,
        })
    }
}

impl Ensurer for ulksys::UplinkObject {
    fn ensure(&self) -> &Self {
        assert!(
            !self.key.is_null(),
            "underlying c-binding returned an invalid UplinkObject; key field is NULL",
        );

        self
    }
}

/// Iterates over a collection of objects' information.
pub struct Iterator {
    /// The object iterator type of the underlying c-bindings Rust crate that an instance of this
    /// struct represents and guards its life time until this instance drops.
    inner: *mut ulksys::UplinkObjectIterator,
}

impl Iterator {
    /// Creates an objects iterator instance from the type exposed by the uplink c-bindings.
    pub(crate) fn from_uplink_c(uc_iterator: *mut ulksys::UplinkObjectIterator) -> Result<Self> {
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
    type Item = Result<Info<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: we trust that the underlying c-bindings functions don't panic when called with
        // an instance returned by them and they don't return invalid memory references or `null`
        // if next returns `true`.
        unsafe {
            if !ulksys::uplink_object_iterator_next(self.inner) {
                let uc_error = ulksys::uplink_object_iterator_err(self.inner);
                return Error::new_uplink(uc_error).map(Err);
            }

            Some(Info::from_uplink_c(ulksys::uplink_object_iterator_item(
                self.inner,
            )))
        }
    }
}

impl Drop for Iterator {
    fn drop(&mut self) {
        // SAFETY: we trust that the underlying c-binding is safe freeing the memory of a correct
        // `UplinkObjectIterator` value.
        unsafe {
            ulksys::uplink_free_object_iterator(self.inner);
        }
    }
}
