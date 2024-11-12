//! # Implementation design
//!
//! ## Errors
//!
//! All the public functions and methods of this crate that can return an error, they return this
//! specific [`Result` type](crate::Result).
//!
//! This guarantee that the consumers of the crate know exactly the returned
//! [error type](crate::Error) and contemplate exactly how to handle each different error, for
//! example aborting the execution and reporting instructions to the user, logging it and continue,
//! etc.
//!
//! [`Error`](crate::Error) is an `enum` which each variant is of a certain type. Each of these
//! types expose different information accordingly to what they represent.
//!
//! Consumers can make a decision according to the [`Error`](crate::Error) variant, however, for the
//! [`Error::Uplink` variant](crate::Error::Uplink), which represents a Storj DCS error returned by
//! they FFI crate and the majority of them returned by the Storj DCS network, they can make a
//! decision based on the [`error::Uplink` variants](crate::error::Uplink).
//!
//! ### Panics
//!
//! This crate explicitly panics if it finds inconsistency in values returned by the FFI (Rust or C
//! part) or if a non-public function/method is called with in appropriated value, for example those
//! that receive a raw pointer and the passed value is `NULL`.
//!
//! We decided to panic in those situations because we consider that they are bugs and it doesn't
//! make sense to continue the execution in that case.
//!
//! Having explicit panics in cases that we know that there is a bug is worthwhile because we give
//! a panic message that indicates that we have found a bug making it clear to the end users. We
//! expect users to report this panics, in the case that they happen, so we can fix it as soon as
//! possible.
//!
//! Outside of the mentioned specific case, we never panic explicitly, we return errors as
//! mentioned.
//!
//! Obviously, there may be bugs we cannot detect or have thought that they could happen and they
//! may lead to panic. In those cases, Rust leads to panic with its native message. They are not
//! explicit panics, but they may happen.
//!
//! ## Constructors from FFI
//!
//! All the types that are created from FFI types has a constructor named `from_ffi_<type>` or
//! `with_ffi_<type>`<sup>1</sup>, where `<type>` is the name of FFI's type (without the
//! `Uplink` prefix) that hey take as a parameter.
//!
//! These constructors are never public, the most of them have crate visibility, but some of them
//! have module visibility.
//!
//! The differences between the names is conventional. If they take ownership or not of the passed
//! FFI values. See [FFI values "ownership"](#ffi-values-ownership) for knowing about it.
//!
//! These constructors panic when the passed FFI value is invalid, for example:
//!
//! * If the parameter is a raw pointer, the value must not be `NULL`.
//! * If the FFI type is a result (i.e. `Uplink<type_name>Result`), the raw pointer field to the
//!   type (i.e. `Uplink<type_name>`) must not be `NULL` when the raw pointer field to the error
//!   (i.e. `UplinkError`) is `NULL`. See [the `Ensurer` trait](#the-ensurer-trait).
//!
//! These panics are part of [the explicit documented panics](#panics).
//!
//! Some of this constructors return a [`Result`](crate::Result) because a "valid" FFI value may
//! contain data values which are not expected, for example:
//!
//! * A C string that doesn't end with the `NULL` character.
//! * A C string that doesn't contain valid UTF-8 characters.
//!
//! The result with an error due to one of those causes has an
//! [`Error::Internal` variant](crate::Error::Internal) and it isn't mention in their documentation.
//!
//! <sup>1</sup> We are using the FFI type's name as a suffix because some types are created from
//! several FFI types, hence, they have more than one `from_ffi_` constructor and for having the
//! same name convention for all of them and avoid future name clashing, we use this format for all
//! of them despite if they have one or more `from_ffi_` constructors, and we have consequently made
//! the same decision for the `with_ffi_` constructors.
//!
//! ### FFI values "ownership"
//!
//! There is usually a public type of this crate that represents the FFI one.
//!
//! When the constructor is named `from_ffi_` it takes "ownership" of the passed FFI value, and when
//! the name is `with_ffi_` it doesn't.
//!
//! We define the `with_ffi_` constructors when the passed FFI value doesn't have an associated
//! function for freeing it. This happens on those values that are associated to others and their
//! resources are freed by the value that holds them. Any `with_ffi_` constructor must always
//! guarantee that make a copy of the fileds' values of the passed FFI value to untangle the two
//! instanes and not having dangling pointers.
//!
//! `from_ffi_` constructors takes ownership, but, unfortunately, this is not from the Rust's
//! ownership meaning. We cannot enforce the Rust ownership because they are raw pointers or values
//! of struct types that contain raw pointers, hence the Rust's ownership mechanism isn't applied
//! due to they are passed by copy.
//!
//! This fictional "ownership" means that once the FFI value is used to instantiate one of the
//! public types, the FFI value must not be used further because the public type instance is the
//! owner of it and it takes care of freeing it at the right time.
//!
//! ### The `Ensurer` trait
//!
//! This crate defines a trait named `Ensurer` which is only visible inside of the crate.
//!
//! The trait is implemented for all the FFI types that their field values could contain an
//! inconsistency, for example the result types (i.e. `Uplink<type_name>Result`) shouldn't never
//! has both fields (the one that points to the type value and the one points to an error value)
//! `NULL`.
//!
//! The only method that trait has is called internally across this implementation before its used
//! to avoid that an inconsistency cause an ugly panic. An inconsistency should only happen in the
//! case of bug in the FFI (Rust or C part).
//!
//! The method doesn't save of panicking, it explicitly panics when the value is inconsistent
//! following what we specify in our [panics section](#panics).
//!
//! ### Exceptions
//!
//! For convenience, the [`Error` type](crate::error::Error) doesn't follow these conventions.
//!
//! The [`Error`](crate::error::Error) has a crate level constructor named
//! [`new_uplink`](crate::error::Error::new_uplink) and the [`error::Uplink`
//! type](crate::error::Uplink) has a crate level constructor named
//! [`new`](crate::error::Uplink::new) that don't take ownership nor explicitly panic when the
//! passed raw pointer is `NULL`.
//!
//! This exception exists because:
//!
//! * FFI mostly returns errors in "_result_" structs, for example
//!   [`UplinkBucketResult`](uplink_sys::UplinkBucketResult). These "_result_" types usually have
//!   their own "_free_" associated function for example
//!   [`uplink_free_bucket_result`](uplink_sys::uplink_free_bucket_result), hence, a double free
//!   race condition would happen if the caller needs to free a "_result_" does have an error but
//!   also a partial "_result value_".
//! * These two constructors return an `Option` that it's `None` when the passed raw pointer to
//!   [`UplinkError`](uplink_sys::UplinkError) is `NULL` because it's pretty convenient to handle
//!   errors more idiomatically than having to always check by `NULL` inequality for calling or not
//!   the constructor.
//!
//! ## From Rust to FFI
//!
//! When needed, the public types of this crate have a method to return their FFI representation,
//! this method has always crate visibility.
//!
//! The method name is `to_ffi_<type>` or `as_ffi_<type>`<sup>1</sup> depending of the operation
//! cost which follows the current
//! [Rust API naming conventions](https://rust-lang.github.io/api-guidelines/naming.html).
//!
//! These methods always return
//! * a borrowed value (see [FFI values "ownership"](#ffi-values-ownership)), so the returned lives
//!   as long as the instance that returns it.
//! * a value when the returned type only contains simple values, so there isn't any extra heap
//!   allocation.
//!
//! In any of these cases, the caller has worry in freeing their used memory.
//!
//! In some cases this method requires a mutable reference because of some internal constrains,
//! when that's the case, the reasons are mentioned in the method documentation.
//!
//! <sup>1</sup> We have follow the same name convetion as `fom_ffi_` and `with_ffi_` constructors
//! despite unlikely a name clashing will happen. See [contructors from FFI
//! section](#contructors-from-ffi).
